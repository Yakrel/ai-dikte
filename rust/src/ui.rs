//! Small, keyboard-driven terminal UI. All settings use the same validated writer.
use crate::{
    background,
    config::{self, Config, Mode, Notifications},
    diagnostics, live, settings,
};
use anyhow::{Result, bail};
use crossterm::{
    event::{
        self, DisableBracketedPaste, EnableBracketedPaste, Event, KeyCode, KeyEvent, KeyEventKind,
        KeyModifiers,
    },
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};
use std::{
    io::{self, IsTerminal},
    path::PathBuf,
    sync::mpsc,
    time::Duration,
};

const MENU: [&str; 6] = [
    "Status",
    "Settings",
    "Preferences",
    "Diagnostics",
    "Logs",
    "Exit menu",
];
const FIELDS: [&str; 5] = [
    "API key",
    "Spoken language",
    "Writing style",
    "Microphone",
    "Custom vocabulary",
];
const ACCENT: Color = Color::Cyan;
#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Menu,
    Status,
    Settings,
    Preferences,
    Diagnostics,
    Logs,
}

// Drop runs on all ordinary error paths and unwinding. No terminal setup occurs
// until config loading has succeeded; redirected stdin/stdout is rejected.
struct TerminalGuard;
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(
            io::stdout(),
            DisableBracketedPaste,
            LeaveAlternateScreen,
            crossterm::cursor::Show
        );
        let _ = terminal::disable_raw_mode();
    }
}
struct Editor {
    text: String,
    cursor: usize,
    field: usize,
}
impl Editor {
    fn insert(&mut self, text: &str) {
        let clean: String = text
            .chars()
            .filter(|c| !c.is_control() || (*c == '\n' && self.field == 4))
            .collect();
        let byte = self
            .text
            .char_indices()
            .nth(self.cursor)
            .map(|(i, _)| i)
            .unwrap_or(self.text.len());
        self.text.insert_str(byte, &clean);
        self.cursor += clean.chars().count();
    }
    fn backspace(&mut self) {
        if self.cursor > 0 {
            let byte = self.text.char_indices().nth(self.cursor - 1).unwrap().0;
            self.text.remove(byte);
            self.cursor -= 1;
        }
    }
}
type JobResult = Result<(String, Option<bool>)>;
struct App {
    config: Config,
    path: PathBuf,
    key: String,
    has_saved_key: bool,
    vocabulary: String,
    page: Page,
    selected: usize,
    scroll: u16,
    editor: Option<Editor>,
    dirty: bool,
    confirm_discard: bool,
    message: String,
    details: String,
    pending: Option<mpsc::Receiver<JobResult>>,
    saving: bool,
    startup: Option<bool>,
    #[cfg(windows)]
    devices: Vec<String>,
}
impl App {
    fn load(page: Page) -> Result<Self> {
        Self::load_from(config::path()?, page)
    }
    fn load_from(path: PathBuf, page: Page) -> Result<Self> {
        let config = if path.exists() {
            Config::load(&path)?
        } else {
            Config::default()
        };
        let vocabulary = config.custom_vocabulary.join("\n");
        let has_saved_key = config.key().is_ok();
        Ok(Self {
            config,
            path,
            key: String::new(),
            has_saved_key,
            vocabulary,
            page,
            selected: 0,
            scroll: 0,
            editor: None,
            dirty: false,
            confirm_discard: false,
            message: "Win+Z controls recording. Closing this menu leaves dictation running.".into(),
            details: String::new(),
            pending: None,
            saving: false,
            startup: None,
            #[cfg(windows)]
            devices: Vec::new(),
        })
    }
    fn job(&mut self, saving: bool, work: impl FnOnce() -> Result<String> + Send + 'static) {
        self.job_result(saving, move || work().map(|message| (message, None)));
    }
    fn job_result(&mut self, saving: bool, work: impl FnOnce() -> JobResult + Send + 'static) {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(work());
        });
        self.pending = Some(rx);
        self.saving = saving;
        self.message = "Working... Please wait; no microphone audio is sent.".into();
    }
    fn startup(&mut self, toggle: bool) {
        self.job_result(false, move || {
            let mut enabled = background::startup_enabled()?;
            if toggle {
                enabled = !enabled;
                background::set_startup(enabled)?;
            }
            Ok((
                format!(
                    "Start at sign-in: {}{}",
                    if enabled { "enabled" } else { "disabled" },
                    if toggle { " (applied immediately)" } else { "" }
                ),
                Some(enabled),
            ))
        });
    }
    fn poll(&mut self) {
        let Some(rx) = &self.pending else {
            return;
        };
        let result = match rx.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Empty) => return,
            Err(mpsc::TryRecvError::Disconnected) => {
                Err(anyhow::anyhow!("Operation stopped unexpectedly"))
            }
        };
        self.pending = None;
        match result {
            Ok((message, startup)) => {
                if let Some(enabled) = startup {
                    self.startup = Some(enabled);
                }
                if self.saving {
                    self.dirty = false;
                    self.key.clear();
                    self.has_saved_key = true;
                    // Reload the normalized settings so later saves preserve the stored key.
                    match Config::load(&self.path) {
                        Ok(config) => {
                            self.vocabulary = config.custom_vocabulary.join("\n");
                            self.config = config;
                        }
                        Err(error) => {
                            self.message = format!("Saved, but reload failed: {error}");
                            return;
                        }
                    }
                }
                self.message = message;
            }
            Err(error) => self.message = format!("Failed: {error:#}"),
        }
        self.saving = false;
    }
    fn save(&mut self) -> Result<()> {
        let mut config = self.config.clone();
        config.custom_vocabulary = self.vocabulary.lines().map(str::to_owned).collect();
        config.validate()?;
        let key = if self.key.trim().is_empty() {
            config.key()?
        } else {
            self.key.trim().to_owned()
        };
        let path = self.path.clone();
        self.job(true, move || {
            settings::save_verified(config, key, &path, |config, key| {
                tokio::runtime::Runtime::new()?.block_on(live::validate_key(config, key))
            })?;
            Ok("Connection verified. Settings saved for the next recording.".into())
        });
        Ok(())
    }
    fn open(&mut self, page: Page) {
        self.page = page;
        self.selected = 0;
        self.scroll = 0;
        self.details = match page {
            Page::Status => {
                let daemon = match background::running() {
                    Ok(true) => "Running".into(),
                    Ok(false) => "Not running".into(),
                    Err(e) => format!("Unavailable: {e}"),
                };
                format!(
                    "Background app: {daemon}\nMicrophone: {}\nModel: {}\nLanguage: {}\n\nRecording: Win+Z only.\nMenu exit does not stop the background app.\n\n{}",
                    self.config
                        .input_device
                        .as_deref()
                        .unwrap_or("System default"),
                    crate::protocol::MODEL,
                    self.config.language,
                    if cfg!(windows) {
                        "Open ai-dikte without arguments to set up / launch the background app."
                    } else {
                        "Start the installed user service with:\nsystemctl --user start ai-dikte.service"
                    }
                )
            }
            Page::Logs => diagnostics::log().unwrap_or_else(|e| format!("Cannot read logs: {e}")),
            Page::Diagnostics => diagnostics::report(),
            _ => String::new(),
        };
        if page == Page::Preferences {
            self.startup(false);
        }
        #[cfg(windows)]
        if page == Page::Settings {
            match crate::audio::input_devices() {
                Ok(devices) => self.devices = devices,
                Err(e) => self.message = format!("Cannot enumerate microphones: {e}"),
            }
        }
    }
    fn activate_field(&mut self) {
        match self.selected {
            2 => {
                self.config.mode = if self.config.mode == Mode::Smart {
                    Mode::Verbatim
                } else {
                    Mode::Smart
                };
                self.dirty = true;
            }
            3 => {
                #[cfg(windows)]
                {
                    let next = self
                        .config
                        .input_device
                        .as_ref()
                        .and_then(|name| self.devices.iter().position(|d| d == name))
                        .map_or(0, |i| i + 1);
                    self.config.input_device = self.devices.get(next).cloned();
                    self.dirty = true;
                }
                #[cfg(not(windows))]
                {
                    self.message = "Use your system's PipeWire sound settings to choose the default microphone.".into();
                }
            }
            field => {
                let text = match field {
                    0 => self.key.clone(),
                    1 => self.config.language.clone(),
                    _ => self.vocabulary.clone(),
                };
                self.editor = Some(Editor {
                    cursor: text.chars().count(),
                    text,
                    field,
                });
            }
        }
    }
    fn key(&mut self, key: KeyEvent) -> Result<bool> {
        if key.kind == KeyEventKind::Release {
            return Ok(false);
        }
        if self.pending.is_some() {
            self.message =
                "Operation in progress. Wait for the result before editing or exiting.".into();
            return Ok(false);
        }
        if self.confirm_discard {
            if key.code == KeyCode::Char('y') {
                return Ok(true);
            }
            self.confirm_discard = false;
            return Ok(false);
        }
        if let Some(editor) = &mut self.editor {
            match key.code {
                KeyCode::Esc => self.editor = None,
                KeyCode::Enter
                    if key.modifiers.contains(KeyModifiers::ALT) && editor.field == 4 =>
                {
                    editor.insert("\n")
                }
                KeyCode::Enter => {
                    let editor = self.editor.take().unwrap();
                    match editor.field {
                        0 => self.key = editor.text,
                        1 => self.config.language = editor.text,
                        _ => self.vocabulary = editor.text,
                    }
                    self.dirty = true;
                }
                KeyCode::Left => editor.cursor = editor.cursor.saturating_sub(1),
                KeyCode::Right => {
                    editor.cursor = (editor.cursor + 1).min(editor.text.chars().count())
                }
                KeyCode::Home => editor.cursor = 0,
                KeyCode::End => editor.cursor = editor.text.chars().count(),
                KeyCode::Backspace => editor.backspace(),
                KeyCode::Delete => {
                    if let Some((byte, _)) = editor.text.char_indices().nth(editor.cursor) {
                        editor.text.remove(byte);
                    }
                }
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    editor.text.clear();
                    editor.cursor = 0;
                }
                KeyCode::Char(c)
                    if !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                {
                    editor.insert(&c.to_string())
                }
                _ => (),
            }
            return Ok(false);
        }
        let quit = key.code == KeyCode::Char('q')
            || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL));
        if quit || (self.page == Page::Menu && key.code == KeyCode::Esc) {
            if self.dirty {
                self.confirm_discard = true;
                return Ok(false);
            }
            return Ok(true);
        }
        if key.code == KeyCode::Esc {
            self.open(Page::Menu);
            return Ok(false);
        }
        if key.code == KeyCode::Char('s') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.save()?;
            return Ok(false);
        }
        if key.code == KeyCode::Char('r') {
            self.open(self.page);
            return Ok(false);
        }
        let count = match self.page {
            Page::Menu => MENU.len(),
            Page::Settings => FIELDS.len(),
            Page::Preferences => 3,
            _ => 0,
        };
        match key.code {
            KeyCode::Up if count > 0 => self.selected = (self.selected + count - 1) % count,
            KeyCode::Down if count > 0 => self.selected = (self.selected + 1) % count,
            KeyCode::Up => self.scroll = self.scroll.saturating_sub(1),
            KeyCode::Down => self.scroll = self.scroll.saturating_add(1),
            KeyCode::PageDown => self.scroll = self.scroll.saturating_add(10),
            KeyCode::PageUp => self.scroll = self.scroll.saturating_sub(10),
            KeyCode::Enter => match self.page {
                Page::Menu => match self.selected {
                    0 => self.open(Page::Status),
                    1 => self.open(Page::Settings),
                    2 => self.open(Page::Preferences),
                    3 => self.open(Page::Diagnostics),
                    4 => self.open(Page::Logs),
                    _ => {
                        if self.dirty {
                            self.confirm_discard = true;
                        } else {
                            return Ok(true);
                        }
                    }
                },
                Page::Settings => self.activate_field(),
                Page::Preferences => match self.selected {
                    0 => {
                        self.config.audio_cue = !self.config.audio_cue;
                        self.dirty = true;
                    }
                    1 => {
                        self.config.notify_mode = if self.config.notify_mode == Notifications::All {
                            Notifications::None
                        } else {
                            Notifications::All
                        };
                        self.dirty = true;
                    }
                    _ => self.startup(true),
                },
                _ => (),
            },
            KeyCode::Char('t') if self.page == Page::Diagnostics => {
                let config = Config::load(&self.path)?;
                let key = config.key()?;
                self.job(false, move || {
                    tokio::runtime::Runtime::new()?.block_on(live::validate_key(&config, &key))?;
                    Ok("Saved API connection verified. No audio sent.".into())
                });
            }
            KeyCode::Char('c') if self.page == Page::Diagnostics => self.job(false, || {
                diagnostics::check_configuration()?;
                Ok(
                    "Local configuration and device/backend checks passed; no audio recorded."
                        .into(),
                )
            }),
            _ => (),
        }
        Ok(false)
    }
    fn draw(&self, frame: &mut Frame) {
        let area = frame.area();
        if area.width < 44 || area.height < 14 {
            frame.render_widget(
                Paragraph::new("Enlarge terminal to at least 44 x 14. Press q to exit.")
                    .wrap(Wrap { trim: false }),
                area,
            );
            return;
        }
        let [header, body, footer] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(5),
        ])
        .areas(area);
        frame.render_widget(
            Paragraph::new(format!(
                "AI DIKTE  |  Win+Z to dictate{}",
                if self.dirty {
                    "  |  Unsaved changes"
                } else {
                    ""
                }
            ))
            .style(Style::default().fg(ACCENT))
            .block(Block::default().borders(Borders::BOTTOM)),
            header,
        );
        if let Some(editor) = &self.editor {
            let text = if editor.field == 0 {
                "*".repeat(editor.text.chars().count())
            } else {
                editor.text.clone()
            };
            let mut chars: Vec<char> = text.chars().collect();
            chars.insert(editor.cursor.min(chars.len()), '|');
            let before: String = text.chars().take(editor.cursor).collect();
            let row = before.chars().filter(|c| *c == '\n').count();
            let column =
                unicode_width::UnicodeWidthStr::width(before.rsplit('\n').next().unwrap_or(""));
            let vertical = row
                .saturating_sub(body.height.saturating_sub(3) as usize)
                .min(u16::MAX as usize) as u16;
            let horizontal = column
                .saturating_sub(body.width.saturating_sub(3) as usize)
                .min(u16::MAX as usize) as u16;
            frame.render_widget(
                Paragraph::new(chars.into_iter().collect::<String>())
                    .scroll((vertical, horizontal))
                    .block(Block::bordered().title(FIELDS[editor.field])),
                body,
            );
        } else {
            let items: Option<Vec<String>> = match self.page {
                Page::Menu => Some(MENU.iter().map(|s| (*s).into()).collect()),
                Page::Settings => Some(vec![
                    format!(
                        "API key: {}",
                        if self.key.is_empty() {
                            if self.has_saved_key {
                                "configured (unchanged)"
                            } else {
                                "not configured"
                            }
                        } else {
                            "******** (new key)"
                        }
                    ),
                    format!("Spoken language: {}", self.config.language),
                    format!("Writing style: {:?}", self.config.mode),
                    format!(
                        "Microphone: {}",
                        self.config
                            .input_device
                            .as_deref()
                            .unwrap_or("System default")
                    ),
                    format!(
                        "Custom vocabulary: {} entries",
                        self.vocabulary
                            .lines()
                            .filter(|s| !s.trim().is_empty())
                            .count()
                    ),
                ]),
                Page::Preferences => Some(vec![
                    format!("Recording sounds: {}", self.config.audio_cue),
                    format!("Status notifications: {:?}", self.config.notify_mode),
                    format!(
                        "Start at sign-in: {} (Enter toggles immediately)",
                        match self.startup {
                            Some(true) => "enabled",
                            Some(false) => "disabled",
                            None => "unavailable / checking",
                        }
                    ),
                ]),
                _ => None,
            };
            if let Some(items) = items {
                let [list, hint] =
                    Layout::vertical([Constraint::Min(1), Constraint::Length(4)]).areas(body);
                let mut state = ListState::default().with_selected(Some(self.selected));
                frame.render_stateful_widget(
                    List::new(items.into_iter().map(ListItem::new))
                        .block(Block::bordered().title("Menu"))
                        .highlight_style(Style::default().fg(Color::Black).bg(ACCENT))
                        .highlight_symbol("> "),
                    list,
                    &mut state,
                );
                let help = if self.page == Page::Settings {
                    "Smart: removes fillers and adds punctuation. Verbatim: keeps your words and repetitions.\nAPI key: leave blank to retain the saved key. Ctrl+S tests and saves."
                } else {
                    "Enter selects or changes an item. No recording controls in this menu.\nPreferences: errors are always shown. Startup does not start/stop a recording."
                };
                frame.render_widget(Paragraph::new(help).wrap(Wrap { trim: false }), hint);
            } else {
                frame.render_widget(
                    Paragraph::new(self.details.as_str())
                        .wrap(Wrap { trim: false })
                        .scroll((self.scroll, 0))
                        .block(Block::bordered().title(match self.page {
                            Page::Status => "Status",
                            Page::Logs => "Logs",
                            _ => "Diagnostics (saved settings): T test API / C check locally",
                        })),
                    body,
                );
            }
        }
        let hint = if self.confirm_discard {
            "Discard unsaved changes and exit? Y = discard / any other key = stay"
        } else if self.editor.is_some() {
            "Enter apply / Esc cancel / Ctrl+U clear / Alt+Enter new vocabulary line"
        } else {
            "Up/Down navigate | Enter select | Ctrl+S save | R refresh | Esc back | Q exit"
        };
        frame.render_widget(
            Paragraph::new(format!("{}\n{}", hint, self.message))
                .wrap(Wrap { trim: false })
                .block(Block::default().borders(Borders::TOP)),
            footer,
        );
    }
}
fn run(page: Page) -> Result<()> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        bail!(
            "The menu requires an interactive terminal. Use status, doctor, logs or check-config for plain output."
        );
    }
    let mut app = App::load(page)?;
    app.open(page);
    terminal::enable_raw_mode()?;
    let _guard = TerminalGuard;
    execute!(io::stdout(), EnterAlternateScreen, EnableBracketedPaste)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    loop {
        app.poll();
        terminal.draw(|frame| app.draw(frame))?;
        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => match app.key(key) {
                    Ok(true) => break,
                    Ok(false) => (),
                    Err(e) => app.message = format!("Failed: {e:#}"),
                },
                Event::Paste(text) if app.pending.is_none() => {
                    if let Some(editor) = &mut app.editor {
                        editor.insert(&text);
                    }
                }
                _ => (),
            }
        }
    }
    Ok(())
}
pub fn menu() -> Result<()> {
    run(Page::Menu)
}
pub fn setup() -> Result<()> {
    run(Page::Settings)
}
pub fn diagnostics_menu() -> Result<()> {
    run(Page::Diagnostics)
}
pub fn logs_menu() -> Result<()> {
    run(Page::Logs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    fn app() -> (tempfile::TempDir, App) {
        let dir = tempfile::tempdir().unwrap();
        let app = App::load_from(dir.path().join("config.json"), Page::Menu).unwrap();
        (dir, app)
    }
    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }
    fn screen(app: &App, width: u16, height: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|frame| app.draw(frame)).unwrap();
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }
    #[test]
    fn menu_has_no_recording_controls_and_can_exit_without_config() {
        let (_dir, mut app) = app();
        let text = screen(&app, 80, 24);
        for item in MENU {
            assert!(text.contains(item));
        }
        assert!(!text.contains("Start recording"));
        assert!(app.key(key(KeyCode::Char('q'))).unwrap());
        assert!(!app.path.exists());
    }
    #[test]
    fn editor_preserves_unicode_and_filters_terminal_control_sequences() {
        let mut editor = Editor {
            text: "İş🙂".into(),
            cursor: 3,
            field: 1,
        };
        editor.backspace();
        editor.cursor = 1;
        editor.insert("ğ\u{1b}\n");
        assert_eq!(editor.text, "İğş");
        assert_eq!(editor.cursor, 2);
        let mut vocabulary = Editor {
            text: String::new(),
            cursor: 0,
            field: 4,
        };
        vocabulary.insert("Kubernetes\r\nTüpraş");
        assert_eq!(vocabulary.text, "Kubernetes\nTüpraş");
    }
    #[test]
    fn secret_is_masked_in_editor_and_small_resized_screens() {
        let (_dir, mut app) = app();
        app.open(Page::Settings);
        app.editor = Some(Editor {
            text: "secret-api-key".into(),
            cursor: 14,
            field: 0,
        });
        for (width, height) in [(120, 30), (80, 24), (44, 14), (20, 6)] {
            assert!(!screen(&app, width, height).contains("secret-api-key"));
        }
    }
    #[test]
    fn escape_cancels_edit_and_unsaved_exit_requires_explicit_discard() {
        let (_dir, mut app) = app();
        app.page = Page::Settings;
        app.selected = 1;
        app.activate_field();
        app.editor.as_mut().unwrap().text = "en-US".into();
        app.key(key(KeyCode::Esc)).unwrap();
        assert_eq!(app.config.language, "tr-TR");
        assert!(!app.dirty);
        app.activate_field();
        app.editor.as_mut().unwrap().text = "en-US".into();
        app.key(key(KeyCode::Enter)).unwrap();
        assert!(app.dirty);
        assert!(!app.key(key(KeyCode::Char('q'))).unwrap());
        assert!(!app.key(key(KeyCode::Char('n'))).unwrap());
        assert!(!app.key(key(KeyCode::Char('q'))).unwrap());
        assert!(app.key(key(KeyCode::Char('y'))).unwrap());
    }
    #[test]
    fn pending_save_blocks_exit_and_failure_keeps_the_draft() {
        let (_dir, mut app) = app();
        let (tx, rx) = mpsc::channel();
        app.pending = Some(rx);
        app.saving = true;
        app.dirty = true;
        assert!(!app.key(key(KeyCode::Char('q'))).unwrap());
        tx.send(Err(anyhow::anyhow!("API rejected"))).unwrap();
        app.poll();
        assert!(app.pending.is_none());
        assert!(app.dirty);
        assert!(app.message.contains("API rejected"));
        assert!(!app.path.exists());
    }
    #[test]
    fn windows_key_release_does_not_activate_a_second_item() {
        let (_dir, mut app) = app();
        let mut event = key(KeyCode::Down);
        event.kind = KeyEventKind::Release;
        app.key(event).unwrap();
        assert_eq!(app.selected, 0);
        app.key(key(KeyCode::Down)).unwrap();
        assert_eq!(app.selected, 1);
    }
}
