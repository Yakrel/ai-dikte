//! Update only an explicitly marked shortcut block; restore on reload failure.
use anyhow::{Context, Result, bail};
use std::{io::Write, path::Path};
const START: &str = ">>> ai-dikte >>>";
const END: &str = "<<< ai-dikte <<<";
fn without_block(text: &str) -> Result<String> {
    let mut result = String::new();
    let mut inside = false;
    for line in text.lines() {
        if line.trim_start_matches(['#', '-', ' ']) == START {
            if inside {
                bail!("Nested AI Dikte shortcut markers");
            }
            inside = true;
        } else if line.trim_start_matches(['#', '-', ' ']) == END {
            if !inside {
                bail!("Unmatched AI Dikte shortcut end marker");
            }
            inside = false;
        } else if !inside {
            result.push_str(line);
            result.push('\n');
        }
    }
    if inside {
        bail!("Unterminated AI Dikte shortcut block; refusing to remove user configuration");
    }
    Ok(result)
}
fn write(path: &Path, text: &str) -> Result<()> {
    let parent = path.parent().context("Shortcut path has no parent")?;
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    file.write_all(text.as_bytes())?;
    file.as_file().sync_all()?;
    file.persist(path)?;
    Ok(())
}
fn reload() -> Result<()> {
    for argument in ["reload", "configerrors"] {
        let output = std::process::Command::new("hyprctl")
            .arg(argument)
            .output()?;
        if !output.status.success() {
            bail!("hyprctl {argument} failed");
        }
        let text = String::from_utf8_lossy(&output.stdout);
        if argument == "configerrors"
            && !text.trim().is_empty()
            && !text.to_lowercase().contains("no errors")
        {
            bail!("Hyprland reports configuration errors: {text}");
        }
    }
    Ok(())
}
pub fn update(install: bool) -> Result<()> {
    if crate::output::driver()? != "wtype" {
        bail!("Shortcut management requires a Hyprland session");
    }
    crate::diagnostics::require_program("hyprctl")?;
    let home = std::env::var_os("HOME").context("HOME missing")?;
    let dir = std::path::PathBuf::from(home).join(".config/hypr");
    let candidates = ["bindings.lua", "bindings.conf", "hyprland.conf"];
    let target = if dir.join("hyprland.lua").exists() || dir.join("bindings.lua").exists() {
        "bindings.lua"
    } else if dir.join("bindings.conf").exists() {
        "bindings.conf"
    } else if dir.join("hyprland.conf").exists() {
        "hyprland.conf"
    } else {
        bail!("Cannot find an existing Hyprland config; add the Win+Z binding manually");
    };
    let mut edits = Vec::new();
    for name in candidates {
        let path = dir.join(name);
        if !path.exists() {
            continue;
        }
        let old = std::fs::read_to_string(&path)?;
        let mut new = without_block(&old)?;
        if name == target && install {
            let (prefix, binding) = if name.ends_with(".lua") {
                (
                    "--",
                    "o.bind(\"SUPER + Z\", \"AI Dikte\", \"ai-dikte-toggle\")",
                )
            } else {
                ("#", "bind = SUPER, Z, exec, ai-dikte-toggle")
            };
            new.push_str(&format!("{prefix} {START}\n{binding}\n{prefix} {END}\n"));
        }
        edits.push((path, old, new));
    }
    if install
        && !edits
            .iter()
            .any(|(path, _, _)| path.file_name().is_some_and(|name| name == target))
    {
        bail!("Selected Hyprland bindings file is missing; create/source it first");
    }
    let result = (|| {
        for (path, _, new) in &edits {
            write(path, new)?;
        }
        reload()
    })();
    if let Err(error) = result {
        for (path, old, _) in &edits {
            write(path, old).context("Could not restore Hyprland configuration")?;
        }
        reload().context("Original Hyprland config restored, but reload failed")?;
        return Err(error);
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_unrelated_bindings() {
        assert_eq!(
            without_block("before\n# >>> ai-dikte >>>\nbind\n# <<< ai-dikte <<<\nafter\n").unwrap(),
            "before\nafter\n"
        );
    }
    #[test]
    fn incomplete_markers_do_not_delete_user_text() {
        assert!(without_block("# >>> ai-dikte >>>\nuser data").is_err());
    }
}
