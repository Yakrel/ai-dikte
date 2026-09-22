//! Update only an explicitly marked shortcut block; restore on reload failure.
use anyhow::{Context, Result, bail};
use std::{io::Write, path::Path};
const START: &str = ">>> ai-dikte >>>";
const END: &str = "<<< ai-dikte <<<";
fn without_block(text: &str) -> Result<String> {
    let mut result = String::new();
    let mut inside = false;
    for line in text.split_inclusive('\n') {
        let marker = line.trim_end_matches(['\r', '\n']);
        if marker.trim_start_matches(['#', '-', ' ']) == START {
            if inside {
                bail!("Nested AI Dikte shortcut markers");
            }
            inside = true;
        } else if marker.trim_start_matches(['#', '-', ' ']) == END {
            if !inside {
                bail!("Unmatched AI Dikte shortcut end marker");
            }
            inside = false;
        } else if !inside {
            result.push_str(line);
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
    update_files(&dir, install, reload)
}

fn update_files(dir: &Path, install: bool, mut reload: impl FnMut() -> Result<()>) -> Result<()> {
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
    let target_path = if install {
        Some(
            dir.join(target)
                .canonicalize()
                .context("Selected Hyprland bindings file is missing; create/source it first")?,
        )
    } else {
        None
    };
    let mut visited = Vec::new();
    for name in candidates {
        let path = dir.join(name);
        match std::fs::symlink_metadata(&path) {
            Ok(_) => (),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        }
        // Replace the resolved file atomically, never the user's symlink.
        let path = path.canonicalize()?;
        if visited.contains(&path) {
            continue;
        }
        visited.push(path.clone());
        let old = std::fs::read_to_string(&path)?;
        let mut new = without_block(&old)?;
        if target_path.as_ref() == Some(&path) {
            let (prefix, binding) = if target.ends_with(".lua") {
                (
                    "--",
                    "o.bind(\"SUPER + Z\", \"AI Dikte\", \"ai-dikte-toggle\")",
                )
            } else {
                ("#", "bind = SUPER, Z, exec, ai-dikte-toggle")
            };
            if !new.is_empty() && !new.ends_with('\n') {
                new.push('\n');
            }
            new.push_str(&format!("{prefix} {START}\n{binding}\n{prefix} {END}\n"));
        }
        if new != old {
            edits.push((path, old, new));
        }
    }
    if edits.is_empty() {
        return Ok(());
    }
    let mut written = 0;
    let result = (|| {
        for (path, _, new) in &edits {
            write(path, new)?;
            written += 1;
        }
        reload()
    })();
    if let Err(error) = result {
        for (path, old, _) in &edits[..written] {
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

    #[test]
    #[cfg(unix)]
    fn unchanged_files_keep_their_bytes_and_inodes() {
        use std::os::unix::fs::MetadataExt;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hyprland.conf");
        let original = "first\r\nlast without newline";
        std::fs::write(&path, original).unwrap();
        let before = std::fs::metadata(&path).unwrap();
        update_files(dir.path(), false, || panic!("No-op must not reload")).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
        assert_eq!(std::fs::metadata(&path).unwrap().ino(), before.ino());

        let bindings = dir.path().join("bindings.conf");
        std::fs::write(&bindings, "# User bindings\n").unwrap();
        update_files(dir.path(), true, || Ok(())).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
        assert_eq!(std::fs::metadata(&path).unwrap().ino(), before.ino());
        let installed = std::fs::read_to_string(&bindings).unwrap();
        let before = std::fs::metadata(&bindings).unwrap();
        update_files(dir.path(), true, || panic!("No-op must not reload")).unwrap();
        assert_eq!(std::fs::read_to_string(&bindings).unwrap(), installed);
        assert_eq!(std::fs::metadata(&bindings).unwrap().ino(), before.ino());
    }

    #[test]
    #[cfg(unix)]
    fn updates_symlink_target_and_restores_it_after_reload_failure() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("tracked.conf");
        let link = dir.path().join("hyprland.conf");
        let original = "# User configuration\r\n";
        std::fs::write(&target, original).unwrap();
        symlink("tracked.conf", &link).unwrap();
        let mut first_reload = true;
        let result = update_files(dir.path(), true, || {
            if first_reload {
                first_reload = false;
                bail!("Rejected new configuration");
            }
            Ok(())
        });
        assert!(result.is_err());
        assert_eq!(
            std::fs::read_link(&link).unwrap(),
            Path::new("tracked.conf")
        );
        assert_eq!(std::fs::read_to_string(&target).unwrap(), original);

        update_files(dir.path(), true, || Ok(())).unwrap();
        assert_eq!(
            std::fs::read_link(&link).unwrap(),
            Path::new("tracked.conf")
        );
        let installed = std::fs::read_to_string(&target).unwrap();
        assert!(installed.starts_with(original));
        assert!(installed.contains("bind = SUPER, Z, exec, ai-dikte-toggle"));
        update_files(dir.path(), false, || Ok(())).unwrap();
        assert_eq!(
            std::fs::read_link(&link).unwrap(),
            Path::new("tracked.conf")
        );
        assert_eq!(std::fs::read_to_string(&target).unwrap(), original);
    }

    #[test]
    fn invalid_markers_abort_before_any_files_change() {
        let dir = tempfile::tempdir().unwrap();
        let bindings = dir.path().join("bindings.conf");
        let config = dir.path().join("hyprland.conf");
        let original = "# User bindings\n";
        let malformed = "# >>> ai-dikte >>>\nuser data";
        std::fs::write(&bindings, original).unwrap();
        std::fs::write(&config, malformed).unwrap();
        assert!(
            update_files(dir.path(), true, || panic!(
                "Invalid config must not reload"
            ))
            .is_err()
        );
        assert_eq!(std::fs::read_to_string(&bindings).unwrap(), original);
        assert_eq!(std::fs::read_to_string(&config).unwrap(), malformed);
    }

    #[test]
    #[cfg(unix)]
    fn aliased_candidates_do_not_remove_the_installed_shortcut() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::tempdir().unwrap();
        let bindings = dir.path().join("bindings.conf");
        let config = dir.path().join("hyprland.conf");
        std::fs::write(&bindings, "# User bindings\n").unwrap();
        symlink("bindings.conf", &config).unwrap();
        update_files(dir.path(), true, || Ok(())).unwrap();
        let installed = std::fs::read_to_string(&bindings).unwrap();
        update_files(dir.path(), true, || panic!("No-op must not reload")).unwrap();
        assert_eq!(std::fs::read_to_string(&bindings).unwrap(), installed);
        assert_eq!(
            std::fs::read_link(&config).unwrap(),
            Path::new("bindings.conf")
        );
    }
}
