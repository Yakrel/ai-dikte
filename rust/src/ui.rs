use crate::{
    config::{self, Config, Mode, Notifications},
    live,
};
use anyhow::Result;
use eframe::egui::{self, Color32, RichText};
use std::{
    path::{Path, PathBuf},
    sync::mpsc,
};

const ACCENT: Color32 = Color32::from_rgb(105, 205, 190);
const MUTED: Color32 = Color32::from_rgb(167, 177, 192);

pub fn setup() -> Result<()> {
    let path = config::path()?;
    let config = if path.exists() {
        Config::load(&path)?
    } else {
        Config::default()
    };
    #[cfg(windows)]
    let key = String::new();
    #[cfg(not(windows))]
    let key = config.api_key.clone().unwrap_or_default();
    let vocabulary = config.custom_vocabulary.join("\n");
    #[cfg(windows)]
    let devices = crate::audio::input_devices()?;
    let app = Settings {
        config,
        key,
        vocabulary,
        #[cfg(windows)]
        devices,
        path,
        status: String::new(),
        failed: false,
        show_key: false,
        pending: None,
    };
    eframe::run_native(
        "AI Dikte — Settings",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([700.0, 880.0])
                .with_min_inner_size([460.0, 520.0])
                .with_icon(eframe::icon_data::from_png_bytes(include_bytes!(
                    "../../ai-dikte.png"
                ))?),
            ..Default::default()
        },
        Box::new(|cc| {
            let mut style = (*cc.egui_ctx.style()).clone();
            style.visuals = egui::Visuals::dark();
            style.visuals.panel_fill = Color32::from_rgb(18, 22, 29);
            style.visuals.override_text_color = Some(Color32::from_rgb(233, 237, 244));
            style.visuals.selection.bg_fill = Color32::from_rgb(37, 91, 87);
            style.visuals.selection.stroke = egui::Stroke::new(1.0_f32, ACCENT);
            style.visuals.extreme_bg_color = Color32::from_rgb(16, 20, 27);
            style.spacing.item_spacing = egui::vec2(8.0, 7.0);
            style.spacing.button_padding = egui::vec2(12.0, 7.0);
            style.spacing.interact_size.y = 30.0;
            style
                .text_styles
                .insert(egui::TextStyle::Body, egui::FontId::proportional(15.0));
            style
                .text_styles
                .insert(egui::TextStyle::Button, egui::FontId::proportional(14.0));
            style
                .text_styles
                .insert(egui::TextStyle::Small, egui::FontId::proportional(12.5));
            cc.egui_ctx.set_style(style);
            Ok(Box::new(app))
        }),
    )
    .map_err(|_| anyhow::anyhow!("Cannot open settings window; check graphical session"))
}

struct Settings {
    config: Config,
    key: String,
    vocabulary: String,
    #[cfg(windows)]
    devices: Vec<String>,
    path: PathBuf,
    status: String,
    failed: bool,
    show_key: bool,
    pending: Option<mpsc::Receiver<Result<()>>>,
}

// Every save must pass live validation, even if the key or settings are unchanged.
// Validation runs before either the credential store or the config file is touched.
fn save_verified(
    mut config: Config,
    key: String,
    path: &Path,
    validate: impl FnOnce(&Config, &str) -> Result<()>,
) -> Result<()> {
    config.validate()?;
    if key.trim().is_empty() {
        anyhow::bail!("Enter a Google AI API key");
    }
    validate(&config, &key)?;
    #[cfg(windows)]
    config::credentials::write(&key)?;
    #[cfg(not(windows))]
    {
        config.api_key = Some(key);
    }
    config.save(path)
}

impl Settings {
    fn save(&mut self) -> Result<()> {
        let mut config = self.config.clone();
        config.custom_vocabulary = self.vocabulary.lines().map(str::to_owned).collect();
        config.validate()?;
        #[cfg(not(windows))]
        if config.input_device.is_some() {
            anyhow::bail!("Select the default microphone in your system's PipeWire audio settings");
        }
        #[cfg(windows)]
        let key = if self.key.trim().is_empty() {
            config.key()?
        } else {
            self.key.trim().to_owned()
        };
        #[cfg(not(windows))]
        let key = self.key.trim().to_owned();
        if key.is_empty() {
            anyhow::bail!("Enter a Google AI API key");
        }
        let path = self.path.clone();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let result = save_verified(config, key, &path, |config, key| {
                tokio::runtime::Runtime::new()?.block_on(live::validate_key(config, key))
            });
            let _ = tx.send(result);
        });
        self.pending = Some(rx);
        self.failed = false;
        self.status = "Connecting to Gemini and checking API access…".into();
        Ok(())
    }
    fn error(&mut self, error: impl std::fmt::Display) {
        self.failed = true;
        self.status = format!("Not saved. {error}");
    }
}

fn hint(ui: &mut egui::Ui, text: &str) {
    ui.add(egui::Label::new(RichText::new(text).small().color(MUTED)).wrap());
}
fn card(ui: &mut egui::Ui, title: &str, content: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(Color32::from_rgb(27, 33, 43))
        .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgb(44, 53, 67)))
        .corner_radius(10)
        .inner_margin(16)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(title).size(17.0).strong());
            ui.add_space(3.0);
            content(ui);
        });
    ui.add_space(7.0);
}

fn writing_style(ui: &mut egui::Ui, selected: &mut Mode, choice: Mode) {
    let (label, description) = match choice {
        Mode::Smart => (
            "Clean up speech (Smart)",
            "Remove filler words and false starts; add punctuation and readable formatting.",
        ),
        Mode::Verbatim => (
            "Word for word (Verbatim)",
            "Keep the words you say, including repetitions and fillers such as ‘um’.",
        ),
    };
    ui.radio_value(selected, choice, label);
    hint(ui, description);
}

impl eframe::App for Settings {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        if let Some(rx) = &self.pending {
            match rx.try_recv() {
                Ok(result) => {
                    self.pending = None;
                    match result {
                        Ok(()) => {
                            self.failed = false;
                            self.status =
                                "Connection verified. Settings saved for your next recording."
                                    .into();
                        }
                        Err(e) => self.error(format!("{e:#}")),
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.pending = None;
                    self.error("The save operation stopped unexpectedly. Please try again.");
                }
                Err(mpsc::TryRecvError::Empty) => {
                    ctx.request_repaint_after(std::time::Duration::from_millis(100));
                }
            }
        }
        egui::TopBottomPanel::top("header")
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(18, 22, 29))
                    .inner_margin(20),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("AI Dikte").size(27.0).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new("WIN + Z").small().color(ACCENT));
                    });
                });
                hint(
                    ui,
                    "Voice dictation, your way. Choose how your speech becomes text.",
                );
            });
        egui::TopBottomPanel::bottom("save_footer")
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(18, 22, 29))
                    .inner_margin(16),
            )
            .show(ctx, |ui| {
                let pending = self.pending.is_some();
                ui.horizontal(|ui| {
                    let button = egui::Button::new(
                        RichText::new("Test connection & save")
                            .color(Color32::from_rgb(13, 32, 31))
                            .strong(),
                    )
                    .fill(ACCENT)
                    .min_size(egui::vec2(210.0, 36.0));
                    if ui.add_enabled(!pending, button).clicked() {
                        if let Err(e) = self.save() {
                            self.error(format!("{e:#}"));
                        }
                        ctx.request_repaint();
                    }
                    if pending {
                        ui.spinner();
                    }
                });
                if self.status.is_empty() {
                    hint(
                        ui,
                        "Every save checks Gemini access. No microphone audio is sent.",
                    );
                } else {
                    let color = if self.failed {
                        Color32::from_rgb(255, 151, 151)
                    } else {
                        ACCENT
                    };
                    ui.add(
                        egui::Label::new(RichText::new(&self.status).small().color(color)).wrap(),
                    );
                }
            });
        egui::CentralPanel::default().frame(egui::Frame::new().fill(Color32::from_rgb(18, 22, 29)).inner_margin(16)).show(ctx, |ui| {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                ui.add_enabled_ui(self.pending.is_none(), |ui| {
                    card(ui, "Connection", |ui| {
                        ui.label("Google AI API key");
                        ui.horizontal(|ui| {
                            let width = (ui.available_width() - 80.0).max(100.0);
                            ui.add_sized([width, 30.0], egui::TextEdit::singleline(&mut self.key)
                                .password(!self.show_key).hint_text("Enter your API key"));
                            ui.toggle_value(&mut self.show_key, "Show");
                        });
                        #[cfg(windows)]
                        hint(ui, "Leave blank to test and reuse your saved key in Windows Credential Manager.");
                        #[cfg(not(windows))]
                        hint(ui, "Stored in your private configuration file. Tested with Gemini before saving.");
                    });
                    card(ui, "Transcription", |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Spoken language");
                            ui.add_sized([100.0, 30.0], egui::TextEdit::singleline(&mut self.config.language));
                            hint(ui, "e.g. tr-TR or en-US");
                        });
                        ui.add_space(4.0);
                        ui.label(RichText::new("Writing style").strong());
                        if ui.available_width() >= 500.0 {
                            ui.columns(2, |columns| {
                                writing_style(&mut columns[0], &mut self.config.mode, Mode::Smart);
                                writing_style(&mut columns[1], &mut self.config.mode, Mode::Verbatim);
                            });
                        } else {
                            writing_style(ui, &mut self.config.mode, Mode::Smart);
                            writing_style(ui, &mut self.config.mode, Mode::Verbatim);
                        }
                        ui.add_space(5.0);
                        ui.label("Custom vocabulary");
                        hint(ui, "Names or technical terms Gemini should recognize. One entry per line.");
                        ui.add(egui::TextEdit::multiline(&mut self.vocabulary)
                            .hint_text("AI Dikte\nKubernetes")
                            .desired_rows(3).desired_width(f32::INFINITY));
                    });
                    card(ui, "Recording & feedback", |ui| {
                        #[cfg(windows)]
                        {
                            ui.label("Microphone");
                            ui.horizontal(|ui| {
                                egui::ComboBox::from_id_salt("microphone")
                                    .width((ui.available_width() - 110.0).max(120.0))
                                    .selected_text(self.config.input_device.as_deref().unwrap_or("System default"))
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut self.config.input_device, None, "System default");
                                        for name in &self.devices {
                                            ui.selectable_value(&mut self.config.input_device, Some(name.clone()), name);
                                        }
                                    });
                                if ui.button("Refresh").clicked() {
                                    match crate::audio::input_devices() {
                                        Ok(devices) => self.devices = devices,
                                        Err(error) => self.error(error),
                                    }
                                }
                            });
                        }
                        #[cfg(not(windows))]
                        {
                            ui.label("Microphone: system default (PipeWire)");
                            hint(ui, "Change the input device in your system audio settings.");
                        }
                        ui.checkbox(&mut self.config.audio_cue, "Play sounds when recording starts and finishes");
                        let mut notifications = self.config.notify_mode == Notifications::All;
                        ui.checkbox(&mut notifications, "Show recording status notifications");
                        self.config.notify_mode = if notifications { Notifications::All } else { Notifications::None };
                        hint(ui, "Errors are always shown.");
                    });
                });
            });
        });
    }
}

pub fn text_window(title: &str, mut text: String) -> Result<()> {
    eframe::run_simple_native(title, eframe::NativeOptions::default(), move |ctx, _| {
        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.button("Copy").clicked() {
                ctx.copy_text(text.clone());
            }
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut text)
                        .desired_width(f32::INFINITY)
                        .interactive(false),
                );
            });
        });
    })
    .map_err(|_| anyhow::anyhow!("Cannot open diagnostic window"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unchanged_invalid_key_is_rechecked_and_does_not_overwrite_settings() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let config = Config {
            api_key: Some("12345".into()),
            ..Config::default()
        };
        config.save(&path).unwrap();
        let original = std::fs::read(&path).unwrap();
        let mut checked = false;
        let result = save_verified(config, "12345".into(), &path, |_, key| {
            checked = true;
            assert_eq!(key, "12345");
            anyhow::bail!("Gemini rejected the request (code 400)");
        });
        assert!(checked);
        assert!(result.is_err());
        assert_eq!(std::fs::read(&path).unwrap(), original);
    }
    #[test]
    fn failed_first_validation_does_not_create_config() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        assert!(
            save_verified(Config::default(), "12345".into(), &path, |_, _| {
                anyhow::bail!("Cannot connect to Gemini");
            })
            .is_err()
        );
        assert!(!path.exists());
    }
}
