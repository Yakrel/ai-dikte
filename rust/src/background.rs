//! Desktop-session integration; never run dictation as a privileged system service.
use anyhow::Result;

pub fn running() -> Result<bool> {
    #[cfg(windows)]
    {
        crate::windows::running()
    }
    #[cfg(not(windows))]
    {
        crate::linux::running()
    }
}

pub fn start() -> Result<()> {
    #[cfg(windows)]
    {
        crate::windows::ensure_background()
    }
    #[cfg(not(windows))]
    {
        let output = systemctl(&["start", "ai-dikte.service"])?;
        anyhow::ensure!(
            output.status.success(),
            "Cannot start user service; check systemctl --user status ai-dikte.service"
        );
        Ok(())
    }
}

pub fn stop() -> Result<()> {
    #[cfg(windows)]
    {
        crate::windows::stop_daemon()
    }
    #[cfg(not(windows))]
    {
        let output = systemctl(&["stop", "ai-dikte.service"])?;
        anyhow::ensure!(
            output.status.success(),
            "Cannot stop user service; check systemctl --user status ai-dikte.service"
        );
        Ok(())
    }
}

pub fn startup_enabled() -> Result<bool> {
    #[cfg(windows)]
    {
        crate::windows::startup_enabled()
    }
    #[cfg(not(windows))]
    {
        let output = systemctl(&["is-enabled", "ai-dikte.service"])?;
        let state = String::from_utf8_lossy(&output.stdout);
        match state.trim() {
            "enabled" | "enabled-runtime" => Ok(true),
            "disabled" => Ok(false),
            other => anyhow::bail!(
                "Cannot manage startup (service state: {other}); install the user service first"
            ),
        }
    }
}

pub fn set_startup(enabled: bool) -> Result<()> {
    #[cfg(windows)]
    {
        crate::windows::set_startup(enabled)
    }
    #[cfg(not(windows))]
    {
        let output = systemctl(&[
            if enabled { "enable" } else { "disable" },
            "ai-dikte.service",
        ])?;
        anyhow::ensure!(
            output.status.success(),
            "Cannot change user-service startup; check systemctl --user status ai-dikte.service"
        );
        Ok(())
    }
}

#[cfg(not(windows))]
fn systemctl(args: &[&str]) -> Result<std::process::Output> {
    use anyhow::Context;
    tokio::runtime::Runtime::new()?.block_on(async {
        tokio::time::timeout(
            std::time::Duration::from_secs(3),
            tokio::process::Command::new("systemctl")
                .arg("--user")
                .args(args)
                .kill_on_drop(true)
                .output(),
        )
        .await
        .context("User service manager timed out")?
        .context("Cannot run systemctl --user")
    })
}
