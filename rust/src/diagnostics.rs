use anyhow::Result;
use std::io::Write;
pub fn append(message: &str) -> Result<()> {
    let path = crate::config::path()?.with_file_name("session.log");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let rotate = std::fs::metadata(&path)
        .map(|m| m.len() > 128 * 1024)
        .unwrap_or(false);
    let mut options = std::fs::OpenOptions::new();
    options
        .create(true)
        .write(true)
        .append(!rotate)
        .truncate(rotate);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    let mut file = options.open(path)?;
    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    writeln!(file, "{time}: {message}")?;
    Ok(())
}
pub fn report() -> String {
    let mut lines = vec![
        format!("AI Dikte {}", env!("CARGO_PKG_VERSION")),
        format!("Platform: {}", std::env::consts::OS),
        format!("Model: {}", crate::protocol::MODEL),
    ];
    match crate::config::path().and_then(|path| crate::config::Config::load(&path)) {
        Ok(config) => {
            lines.push(format!("Language: {} / {:?}", config.language, config.mode));
            lines.push(if config.key().is_ok() {
                "API key: configured".into()
            } else {
                "API key: unavailable".into()
            });
        }
        Err(error) => lines.push(format!("Configuration: {error}")),
    }
    #[cfg(not(windows))]
    {
        lines.push(format!("Typing backend: {:?}", crate::output::driver()));
    }
    lines
        .push("Live API, microphone and target window have not been tested by this report.".into());
    lines.join("\n")
}
pub fn log() -> Result<String> {
    let path = crate::config::path()?.with_file_name("session.log");
    Ok(std::fs::read_to_string(path)?)
}

pub fn check_configuration() -> Result<()> {
    let config = crate::config::Config::load(&crate::config::path()?)?;
    config.key()?;
    #[cfg(windows)]
    {
        crate::audio::select_device(config.input_device.as_deref())?;
    }
    #[cfg(not(windows))]
    {
        if std::env::var_os("WAYLAND_DISPLAY").is_none() {
            anyhow::bail!("A Wayland desktop session is required");
        }
        for program in ["pw-record", "notify-send", crate::output::driver()?] {
            require_program(program)?;
        }
        if config.audio_cue {
            require_program("pw-play")?;
        }
    }
    Ok(())
}
#[cfg(not(windows))]
pub fn require_program(program: &str) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    if let Some(path) = std::env::var_os("PATH") {
        for directory in std::env::split_paths(&path) {
            if let Ok(meta) = std::fs::metadata(directory.join(program))
                && meta.is_file()
                && meta.permissions().mode() & 0o111 != 0
            {
                return Ok(());
            }
        }
    }
    anyhow::bail!("Required program is missing: {program}")
}
pub fn self_test() -> Result<()> {
    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../../ai-dikte.png"))?;
    anyhow::ensure!(
        icon.width > 0 && icon.height > 0,
        "Invalid application icon"
    );
    let mut transcript = crate::protocol::Transcript::default();
    for _ in 0..2 {
        transcript.receive(
            &serde_json::json!({"serverContent":{"inputTranscription":{"text":"Evet."}}}),
            std::time::Duration::ZERO,
        )?;
    }
    anyhow::ensure!(
        transcript.finish()? == "Evet. Evet.",
        "Transcript self-test failed"
    );
    crate::protocol::audio(&[0, 0])?;
    println!(
        "AI Dikte {} Rust runtime self-test passed",
        env!("CARGO_PKG_VERSION")
    );
    Ok(())
}
