//! Minimalist interactive console menu (MAS / Massgrave style).
//! Plain standard I/O; zero TUI library dependencies. All interface in English.
use crate::{
    background,
    config::{self, Config, Mode},
    diagnostics, live, settings,
};
use anyhow::{Context, Result, bail};
use std::io::{self, Write};
use std::path::Path;

fn read_input() -> Result<Option<String>> {
    let mut input = String::new();
    if io::stdin().read_line(&mut input)? == 0 {
        Ok(None)
    } else {
        Ok(Some(input))
    }
}

#[cfg(windows)]
struct ConsoleEchoGuard {
    handle: windows_sys::Win32::Foundation::HANDLE,
    mode: u32,
}

#[cfg(windows)]
impl Drop for ConsoleEchoGuard {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::System::Console::SetConsoleMode(self.handle, self.mode);
        }
    }
}

#[cfg(unix)]
struct TerminalEchoGuard {
    fd: i32,
    original: libc::termios,
}

#[cfg(unix)]
impl Drop for TerminalEchoGuard {
    fn drop(&mut self) {
        unsafe {
            libc::tcsetattr(self.fd, libc::TCSANOW, &self.original);
        }
    }
}

/// Reads a secret without echoing it when stdin is an interactive terminal.
/// Redirected stdin falls back to normal line input because there is no local
/// terminal echo to suppress. If echo cannot be disabled on a real terminal,
/// fail instead of risking printing a credential in clear text.
#[cfg(windows)]
fn read_secret() -> Result<Option<String>> {
    use windows_sys::Win32::System::Console::{
        ENABLE_ECHO_INPUT, GetConsoleMode, GetStdHandle, STD_INPUT_HANDLE, SetConsoleMode,
    };

    let handle = unsafe { GetStdHandle(STD_INPUT_HANDLE) };
    let mut original = 0;
    if handle.is_null() || unsafe { GetConsoleMode(handle, &mut original) } == 0 {
        return read_input();
    }

    if unsafe { SetConsoleMode(handle, original & !ENABLE_ECHO_INPUT) } == 0 {
        bail!(
            "Cannot disable console echo for API key input: {}",
            io::Error::last_os_error()
        );
    }

    let guard = ConsoleEchoGuard {
        handle,
        mode: original,
    };
    let result = read_input();
    drop(guard);
    println!();
    result
}

#[cfg(unix)]
fn read_secret() -> Result<Option<String>> {
    let fd = libc::STDIN_FILENO;
    if unsafe { libc::isatty(fd) } == 0 {
        return read_input();
    }

    let mut original = std::mem::MaybeUninit::<libc::termios>::uninit();
    if unsafe { libc::tcgetattr(fd, original.as_mut_ptr()) } != 0 {
        bail!(
            "Cannot read terminal settings for API key input: {}",
            io::Error::last_os_error()
        );
    }
    let original = unsafe { original.assume_init() };
    let mut hidden = unsafe { std::ptr::read(&original) };
    hidden.c_lflag &= !(libc::ECHO | libc::ECHONL);

    if unsafe { libc::tcsetattr(fd, libc::TCSANOW, &hidden) } != 0 {
        bail!(
            "Cannot disable terminal echo for API key input: {}",
            io::Error::last_os_error()
        );
    }

    let guard = TerminalEchoGuard { fd, original };
    let result = read_input();
    drop(guard);
    println!();
    result
}

#[cfg(not(any(unix, windows)))]
fn read_secret() -> Result<Option<String>> {
    read_input()
}

/// Explicit `ai-dikte setup` is a one-shot command. This is important for the
/// Windows installer, which waits for setup to finish before continuing.
pub fn setup() -> Result<()> {
    configure_first_run()
}

pub fn diagnostics_menu() -> Result<()> {
    println!("{}", diagnostics::report());
    Ok(())
}

pub fn logs_menu() -> Result<()> {
    println!("{}", diagnostics::log()?);
    Ok(())
}

/// Zero-config launch path: configure once, then continue into the menu.
pub fn first_run_setup() -> Result<()> {
    configure_first_run()?;
    println!("\n Press Enter to continue to the main menu...");
    if read_input()?.is_none() {
        return Ok(());
    }
    menu()
}

fn configure_first_run() -> Result<()> {
    let path = config::path()?;
    let config = if path.exists() {
        Config::load(&path)?
    } else {
        Config::default()
    };

    println!();
    println!("=====================================================");
    println!("                WELCOME TO AI DIKTE                  ");
    println!("=====================================================");
    println!(" AI Dikte is a lightweight, local dictation tool");
    println!(" powered by Gemini Live.");
    println!();
    println!("-----------------------------------------------------");

    configure_api_key(&config, &path)?;

    println!();
    print!(" -> Starting background listener service... ");
    io::stdout().flush()?;
    background::start().context("Could not start background service")?;
    println!("STARTED!");
    background::set_startup(true).context("Could not enable background service at sign-in")?;
    println!(
        " [OK] Background service is active and set to start at sign-in.\n      Press {} to dictate.",
        if cfg!(windows) { "Win+Z" } else { "Meta+Z" }
    );
    Ok(())
}

fn configure_api_key(config: &Config, path: &Path) -> Result<()> {
    #[cfg(windows)]
    let existing = config::credentials::read_optional()?;
    #[cfg(not(windows))]
    let existing = config.api_key.as_deref();
    if existing.is_some_and(|key| !key.trim().is_empty()) {
        println!(" An API key is already configured (hidden).");
        loop {
            print!(" Replace existing API key? [y/N]: ");
            io::stdout().flush()?;
            let Some(answer) = read_input()? else {
                bail!("Input closed; existing API key preserved");
            };
            match answer.trim().to_ascii_lowercase().as_str() {
                "" | "n" | "no" => {
                    // A Windows credential can exist without a settings file.
                    // Restore only missing settings; do not rewrite the credential.
                    if !path.exists() {
                        config.save(path)?;
                    }
                    println!(" [OK] Existing API key preserved.");
                    return Ok(());
                }
                "y" | "yes" => break,
                _ => println!(" [!] Please enter y or n."),
            }
        }
    }
    println!(" Please enter your Gemini API key.");
    println!(" (Get a free key at: https://aistudio.google.com/apikey)");

    loop {
        print!(" Gemini API Key (hidden): ");
        io::stdout().flush()?;
        let Some(key) = read_secret()? else {
            bail!("Input closed before an API key was entered");
        };
        let key = key.trim();

        if key.is_empty() {
            println!(" [!] API key cannot be empty.");
            continue;
        }

        print!(" -> Verifying API key... ");
        io::stdout().flush()?;

        let test_result = {
            let cfg = config.clone();
            let k = key.to_string();
            tokio::runtime::Runtime::new()?.block_on(live::validate_key(&cfg, &k))
        };

        match test_result {
            Ok(()) => {
                println!("SUCCESS!");
                settings::save_verified(config.clone(), key.to_string(), path, |_, _| Ok(()))?;
                println!(" [OK] Settings saved successfully.");
                break;
            }
            Err(err) => {
                println!("FAILED!");
                println!(" [!] Connection failed: {err:#}");
                println!("     Please check your API key and try again.");
            }
        }
    }

    Ok(())
}

pub fn menu() -> Result<()> {
    let path = config::path()?;
    if !path.exists() {
        return first_run_setup();
    }
    let mut config = Config::load(&path)?;
    if config.key().is_err() {
        return first_run_setup();
    }

    loop {
        let running = background::running()?;
        let autostart = background::startup_enabled()?;
        let mic = config.input_device.as_deref().unwrap_or("System Default");

        println!();
        println!("=====================================================");
        println!(
            "                   AI DIKTE (v{})                    ",
            env!("CARGO_PKG_VERSION")
        );
        println!("=====================================================");
        println!(
            " [Status]     : Background service is {}",
            if running { "RUNNING" } else { "STOPPED" }
        );
        println!(
            " [Hotkey]     : {}",
            if cfg!(windows) {
                "Win+Z (Press to talk, press to finish)"
            } else {
                "Meta+Z"
            }
        );
        println!(" [Voice/Mode] : {} ({:?})", config.language, config.mode);
        println!(" [Microphone] : {mic}");
        println!("-----------------------------------------------------");
        println!(" [1] Enter / Update API Key");
        println!(" [2] System & Connection Diagnostics (Doctor)");
        println!(" [3] Writing Style (Smart / Verbatim) & Vocabulary");
        println!(
            " [4] Start at Sign-in: [{}]",
            if autostart { "ENABLED" } else { "DISABLED" }
        );
        println!(
            " [5] Background Service: [{}]",
            if running { "Stop" } else { "Start" }
        );
        println!(" [0] Exit");
        println!("-----------------------------------------------------");
        print!(" Choice [0-5]: ");
        io::stdout().flush()?;

        let Some(choice) = read_input()? else {
            println!("\n Input closed. Exiting AI Dikte menu.");
            break;
        };
        match choice.trim() {
            "1" => update_api_key(&mut config, &path)?,
            "2" => run_doctor(&path)?,
            "3" => update_style_and_vocabulary(&mut config, &path)?,
            "4" => toggle_autostart(autostart)?,
            "5" => toggle_daemon(running)?,
            "0" => {
                println!("\n Exiting AI Dikte menu.");
                break;
            }
            _ => println!(" [!] Invalid choice! Please enter a number between 0 and 5."),
        }
    }
    Ok(())
}

fn update_api_key(config: &mut Config, path: &Path) -> Result<()> {
    println!();
    println!("--- Update API Key ---");
    let current = config.key().unwrap_or_default();
    let masked = mask_key(&current);
    println!(" Current Key: {masked}");
    println!(" (Press Enter without typing to keep the current key)");
    print!(" New Gemini API Key (hidden): ");
    io::stdout().flush()?;

    let Some(new_key) = read_secret()? else {
        println!(" -> Input closed; no change made.");
        return Ok(());
    };
    let new_key = new_key.trim();

    if new_key.is_empty() {
        println!(" -> No change made.");
        return Ok(());
    }

    print!(" -> Verifying new key... ");
    io::stdout().flush()?;

    let test_result = {
        let cfg = config.clone();
        let k = new_key.to_string();
        tokio::runtime::Runtime::new()?.block_on(live::validate_key(&cfg, &k))
    };

    match test_result {
        Ok(()) => {
            println!("SUCCESS!");
            save_api_key(config, new_key.to_string(), path)?;
            println!(" [OK] New API key saved successfully.");
        }
        Err(err) => {
            println!("FAILED!");
            println!(" [!] Verification failed: {err:#}");
            println!("     Previous settings preserved.");
        }
    }
    Ok(())
}

fn save_api_key(config: &mut Config, key: String, path: &Path) -> Result<()> {
    settings::save_verified(config.clone(), key, path, |_, _| Ok(()))?;
    *config = Config::load(path)?;
    Ok(())
}

fn run_doctor(path: &Path) -> Result<()> {
    println!();
    println!("--- System & Connection Diagnostics ---");
    println!("{}", diagnostics::report());

    if let Ok(config) = Config::load(path)
        && let Ok(key) = config.key()
    {
        print!(" -> Testing live Gemini WebSocket connection... ");
        io::stdout().flush()?;
        match tokio::runtime::Runtime::new()?.block_on(live::validate_key(&config, &key)) {
            Ok(()) => println!("SUCCESS!"),
            Err(e) => println!("FAILED: {e:#}"),
        }
    }

    println!("\n Press Enter to return to menu...");
    let _ = read_input()?;
    Ok(())
}

fn update_style_and_vocabulary(config: &mut Config, path: &Path) -> Result<()> {
    loop {
        println!();
        println!("--- Language, Style & Vocabulary ---");
        println!(
            " [1] Spoken Language  : {} (Default: tr-TR)",
            config.language
        );
        println!(" [2] Writing Style    : {:?}", config.mode);
        println!(
            " [3] Custom Vocabulary: {} terms configured",
            config.custom_vocabulary.len()
        );
        println!(" [0] Back");
        print!(" Choice [0-3]: ");
        io::stdout().flush()?;

        let Some(choice) = read_input()? else {
            break;
        };
        match choice.trim() {
            "1" => {
                println!("\n Supported examples: tr-TR, en-US, de-DE, fr-FR");
                print!(" Enter language code [Enter to keep {}]: ", config.language);
                io::stdout().flush()?;
                let Some(lang) = read_input()? else {
                    break;
                };
                let lang = lang.trim();
                if !lang.is_empty() {
                    config.language = lang.to_string();
                    save_config_change(config, path)?;
                    println!(" [OK] Spoken language set to {}.", config.language);
                }
            }
            "2" => {
                config.mode = if config.mode == Mode::Smart {
                    Mode::Verbatim
                } else {
                    Mode::Smart
                };
                save_config_change(config, path)?;
                println!(" [OK] Writing style changed to {:?}.", config.mode);
                if config.mode == Mode::Smart {
                    println!("     (Smart: Cleans up filler words and adds punctuation)");
                } else {
                    println!("     (Verbatim: Types exact spoken words and repetitions)");
                }
            }
            "3" => {
                let words_display = if config.custom_vocabulary.is_empty() {
                    " (None)".to_string()
                } else {
                    config.custom_vocabulary.join(", ")
                };
                println!("\n Current Custom Vocabulary:\n {words_display}");
                println!(" Enter comma-separated terms (leave empty to keep current):");
                print!(" Terms: ");
                io::stdout().flush()?;
                let Some(words) = read_input()? else {
                    break;
                };
                let words = words.trim();
                if !words.is_empty() {
                    config.custom_vocabulary = parse_vocabulary(words);
                    save_config_change(config, path)?;
                    println!(" [OK] Custom vocabulary updated.");
                }
            }
            "0" => break,
            _ => println!(" [!] Invalid choice!"),
        }
    }
    Ok(())
}

fn save_config_change(config: &Config, path: &Path) -> Result<()> {
    let key = config.key()?;
    settings::save_verified(config.clone(), key, path, |_, _| Ok(()))
}

fn toggle_autostart(current: bool) -> Result<()> {
    let target = !current;
    background::set_startup(target)?;
    println!(
        "\n [OK] Start at sign-in: {}",
        if target { "ENABLED" } else { "DISABLED" }
    );
    Ok(())
}

fn toggle_daemon(running: bool) -> Result<()> {
    if running {
        print!("\n -> Stopping background service... ");
        io::stdout().flush()?;
        background::stop()?;
        println!("STOPPED.");
    } else {
        print!("\n -> Starting background service... ");
        io::stdout().flush()?;
        background::start()?;
        println!("STARTED.");
    }
    Ok(())
}

pub fn mask_key(key: &str) -> String {
    if key.is_empty() {
        "Not configured".to_string()
    } else {
        "Configured (hidden)".to_string()
    }
}

pub fn parse_vocabulary(input: &str) -> Vec<String> {
    input
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(windows))]
    #[test]
    fn style_save_preserves_updated_api_key() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.json");
        let mut config = Config {
            api_key: Some("old-key".into()),
            ..Config::default()
        };
        save_api_key(&mut config, "new-key".into(), &path).unwrap();
        config.mode = Mode::Verbatim;
        save_config_change(&config, &path).unwrap();
        let saved = Config::load(&path).unwrap();
        assert_eq!(saved.key().unwrap(), "new-key");
        assert_eq!(saved.mode, Mode::Verbatim);
    }

    #[test]
    fn test_mask_key() {
        assert_eq!(mask_key(""), "Not configured");
        assert_eq!(mask_key("12345"), "Configured (hidden)");
        assert_eq!(mask_key("1234567890"), "Configured (hidden)");
    }

    #[test]
    fn test_parse_vocabulary() {
        assert_eq!(parse_vocabulary(""), Vec::<String>::new());
        assert_eq!(
            parse_vocabulary("  Kubernetes , Rust, , DevOps "),
            vec![
                "Kubernetes".to_string(),
                "Rust".to_string(),
                "DevOps".to_string()
            ]
        );
    }
}
