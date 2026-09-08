use crate::{
    config::{self, Config, Mode, Notifications},
    live,
};
use anyhow::Result;
use eframe::egui;
use std::{path::PathBuf, sync::mpsc};
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
    let device = config
        .input_device
        .map(|i| i.to_string())
        .unwrap_or_default();
    let app = Settings {
        config,
        key,
        vocabulary,
        device,
        path,
        status: String::new(),
        pending: None,
    };
    eframe::run_native(
        "AI Dikte — Ayarlar",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([540.0, 650.0])
                .with_min_inner_size([430.0, 450.0]),
            ..Default::default()
        },
        Box::new(|_| Ok(Box::new(app))),
    )
    .map_err(|_| anyhow::anyhow!("Cannot open settings window; check graphical session"))
}
struct Settings {
    config: Config,
    key: String,
    vocabulary: String,
    device: String,
    path: PathBuf,
    status: String,
    pending: Option<mpsc::Receiver<Result<()>>>,
}
impl Settings {
    fn save(&mut self) -> Result<()> {
        let mut config = self.config.clone();
        config.custom_vocabulary = self.vocabulary.lines().map(str::to_owned).collect();
        config.input_device = if self.device.trim().is_empty() {
            None
        } else {
            Some(
                self.device
                    .trim()
                    .parse()
                    .map_err(|_| anyhow::anyhow!("Mikrofon numarası geçersiz"))?,
            )
        };
        config.validate()?;
        #[cfg(not(windows))]
        if config.input_device.is_some() {
            anyhow::bail!("Linux'ta mikrofonu sistemin PipeWire ayarlarından seçin");
        }
        let mut key = self.key.trim().to_owned();
        #[cfg(windows)]
        if key.is_empty() {
            key = config.key()?;
        }
        if key.is_empty() {
            anyhow::bail!("API anahtarı boş olamaz");
        }
        let path = self.path.clone();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let result = (|| {
                tokio::runtime::Runtime::new()?.block_on(live::validate_key(&config, &key))?;
                #[cfg(windows)]
                {
                    config::credentials::write(&key)?;
                }
                #[cfg(not(windows))]
                {
                    config.api_key = Some(std::mem::take(&mut key));
                }
                config.save(&path)
            })();
            let _ = tx.send(result);
        });
        self.pending = Some(rx);
        self.status = "Gemini bağlantısı doğrulanıyor…".into();
        Ok(())
    }
}
impl eframe::App for Settings {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        if let Some(rx) = &self.pending {
            match rx.try_recv() {
                Ok(result) => {
                    self.status = match result {
                        Ok(()) => "Ayarlar kaydedildi. Yeni kayıt oturumunda kullanılacak.".into(),
                        Err(e) => format!("Hata: {e:#}"),
                    };
                    self.pending = None;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.status = "Ayar kaydetme işlemi beklenmedik şekilde sonlandı.".into();
                    self.pending = None;
                }
                Err(mpsc::TryRecvError::Empty) => {
                    ctx.request_repaint_after(std::time::Duration::from_millis(100));
                }
            }
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("AI Dikte");
                ui.label("Gemini ile sesli yazma");
                ui.separator();
                ui.add_enabled_ui(self.pending.is_none(), |ui| {
                    ui.label("Google AI API anahtarı");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.key)
                            .password(true)
                            .desired_width(f32::INFINITY),
                    );
                    #[cfg(windows)]
                    ui.small("Boş bırakırsanız Credential Manager'daki mevcut anahtar kullanılır.");
                    ui.label("Dil");
                    ui.text_edit_singleline(&mut self.config.language);
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut self.config.mode, Mode::Smart, "Akıllı düzenleme");
                        ui.selectable_value(&mut self.config.mode, Mode::Verbatim, "Olduğu gibi");
                    });
                    ui.label("Özel kelimeler — her satıra bir kelime");
                    ui.add(
                        egui::TextEdit::multiline(&mut self.vocabulary)
                            .desired_rows(7)
                            .desired_width(f32::INFINITY),
                    );
                    #[cfg(windows)]
                    {
                        ui.label("Mikrofon numarası (varsayılan için boş)");
                        ui.text_edit_singleline(&mut self.device);
                    }
                    #[cfg(not(windows))]
                    ui.label("Mikrofon: PipeWire varsayılan giriş aygıtı");
                    ui.checkbox(&mut self.config.audio_cue, "Sesli bildirim");
                    ui.horizontal(|ui| {
                        ui.label("Bildirimler");
                        ui.selectable_value(
                            &mut self.config.notify_mode,
                            Notifications::All,
                            "Açık",
                        );
                        ui.selectable_value(
                            &mut self.config.notify_mode,
                            Notifications::None,
                            "Kapalı",
                        );
                    });
                    ui.add_space(12.0);
                    if ui.button("Doğrula ve kaydet").clicked()
                        && let Err(e) = self.save()
                    {
                        self.status = format!("Hata: {e:#}");
                    }
                });
                ui.add_space(10.0);
                ui.label(&self.status);
            });
        });
    }
}
