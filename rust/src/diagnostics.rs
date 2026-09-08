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
