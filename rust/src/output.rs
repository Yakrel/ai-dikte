use anyhow::Result;
#[cfg(not(windows))]
pub fn driver() -> Result<&'static str> {
    let desktop = [
        "XDG_CURRENT_DESKTOP",
        "XDG_SESSION_DESKTOP",
        "DESKTOP_SESSION",
    ]
    .iter()
    .filter_map(|key| std::env::var(key).ok())
    .collect::<Vec<_>>()
    .join(" ")
    .to_lowercase();
    select_driver(
        &desktop,
        std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some(),
        std::env::var_os("KDE_FULL_SESSION").is_some(),
    )
}
#[cfg(not(windows))]
fn select_driver(desktop: &str, hyprland: bool, kde: bool) -> Result<&'static str> {
    if hyprland || desktop.contains("hyprland") {
        Ok("wtype")
    } else if kde || desktop.contains("kde") || desktop.contains("plasma") {
        Ok("kwtype")
    } else {
        anyhow::bail!("Unsupported desktop: KDE Plasma or Hyprland Wayland is required")
    }
}
#[cfg(not(windows))]
pub async fn type_text(text: &str) -> Result<()> {
    use anyhow::Context;
    if text.is_empty() {
        return Ok(());
    }
    let driver = driver()?;
    let mut command = tokio::process::Command::new(driver);
    command.args(["--", text]).kill_on_drop(true);
    let output = tokio::time::timeout(std::time::Duration::from_secs(15), command.output())
        .await
        .context("Typing timed out")?
        .with_context(|| format!("Cannot start required backend {driver}"))?;
    if !output.status.success() {
        anyhow::bail!(
            "{driver} failed with {}; text may be partially typed",
            output.status
        );
    }
    Ok(())
}
#[cfg(windows)]
pub async fn type_text(text: &str) -> Result<()> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
    // Do not type while a physical modifier is held after Win+Z.
    for _ in 0..100 {
        let held = [VK_LWIN, VK_RWIN, VK_CONTROL, VK_MENU, VK_SHIFT]
            .iter()
            .any(|key| unsafe { GetAsyncKeyState(*key as i32) } < 0);
        if !held {
            return inject(text);
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    anyhow::bail!("Release keyboard modifiers; text was not typed")
}
#[cfg(windows)]
fn inject(text: &str) -> Result<()> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
    let mut events = Vec::new();
    for unit in text.encode_utf16() {
        for flags in [KEYEVENTF_UNICODE, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP] {
            events.push(INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: 0,
                        wScan: unit,
                        dwFlags: flags,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            });
        }
    }
    for batch in events.chunks(256) {
        let sent = unsafe {
            SendInput(
                batch.len() as u32,
                batch.as_ptr(),
                std::mem::size_of::<INPUT>() as i32,
            )
        };
        if sent as usize != batch.len() {
            anyhow::bail!(
                "SendInput injected {sent}/{} events; text may be partially typed. Check target elevation.",
                batch.len()
            );
        }
    }
    Ok(())
}
#[cfg(all(test, not(windows)))]
mod tests {
    use super::*;
    #[test]
    fn desktop_selection_is_explicit() {
        assert_eq!(select_driver("kde", false, false).unwrap(), "kwtype");
        assert_eq!(select_driver("hyprland", false, false).unwrap(), "wtype");
        assert!(select_driver("gnome", false, false).is_err());
    }
}
