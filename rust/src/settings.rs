#[cfg(windows)]
use crate::config;
use crate::config::Config;
use anyhow::Result;
use std::path::Path;

/// A key accepted by the live API. The private field prevents callers from
/// bypassing verification with a no-op callback.
pub struct VerifiedKey(String);

impl VerifiedKey {
    pub fn verify(config: &Config, key: String) -> Result<Self> {
        let mut checked = config.clone();
        checked.validate()?;
        let key = key.trim().to_owned();
        if key.is_empty() {
            anyhow::bail!("Enter a Google AI API key");
        }
        tokio::runtime::Runtime::new()?.block_on(crate::live::validate_key(&checked, &key))?;
        Ok(Self(key))
    }
}

/// Commit a verified key, updating the caller only after persistence succeeds.
/// Ordinary settings changes use Config::save and never rewrite credentials.
pub fn save_api_key(config: &mut Config, key: VerifiedKey, path: &Path) -> Result<()> {
    let mut updated = config.clone();
    updated.validate()?;
    #[cfg(windows)]
    {
        updated.api_key = None;
        let previous = config::credentials::read_optional()?;
        save_with_credential(
            &updated,
            &key.0,
            path,
            previous.as_deref(),
            config::credentials::restore,
        )?;
    }
    #[cfg(not(windows))]
    {
        updated.api_key = Some(key.0);
        updated.save(path)?;
    }
    *config = updated;
    Ok(())
}

// Credential Manager and the JSON file are separate stores. If replacing the
// settings fails, restore the prior key (or remove a newly created credential).
#[cfg(any(windows, test))]
fn save_with_credential(
    config: &Config,
    key: &str,
    path: &Path,
    previous: Option<&str>,
    mut write_key: impl FnMut(Option<&str>) -> Result<()>,
) -> Result<()> {
    write_key(Some(key))?;
    if let Err(error) = config.save(path) {
        if let Err(rollback) = write_key(previous) {
            return Err(error.context(format!("API key rollback also failed: {rollback:#}")));
        }
        return Err(error);
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn config_write_failure_restores_previous_credential_or_removes_new_one() {
        let dir = tempfile::tempdir().unwrap();
        // A directory cannot be atomically replaced with a config file.
        for previous in [None, Some("old-key")] {
            let mut saved = previous.map(str::to_owned);
            let result =
                save_with_credential(&Config::default(), "new-key", dir.path(), previous, |key| {
                    saved = key.map(str::to_owned);
                    Ok(())
                });
            assert!(result.is_err());
            assert_eq!(saved.as_deref(), previous);
        }
    }
    #[test]
    fn credential_write_failure_does_not_replace_config() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let original = b"original settings";
        std::fs::write(&path, original).unwrap();
        assert!(
            save_with_credential(
                &Config::default(),
                "new-key",
                &path,
                Some("old-key"),
                |_| {
                    anyhow::bail!("Credential store unavailable");
                }
            )
            .is_err()
        );
        assert_eq!(std::fs::read(path).unwrap(), original);
    }

    #[cfg(not(windows))]
    #[test]
    fn style_save_preserves_updated_api_key() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.json");
        let mut config = Config {
            api_key: Some("old-key".into()),
            ..Config::default()
        };
        save_api_key(&mut config, VerifiedKey("new-key".into()), &path).unwrap();
        config.mode = crate::config::Mode::Verbatim;
        config.save(&path).unwrap();
        let saved = Config::load(&path).unwrap();
        assert_eq!(saved.key().unwrap(), "new-key");
        assert_eq!(saved.mode, crate::config::Mode::Verbatim);
    }

    #[cfg(not(windows))]
    #[test]
    fn failed_key_save_preserves_in_memory_credential() {
        let directory = tempfile::tempdir().unwrap();
        let mut config = Config {
            api_key: Some("old-key".into()),
            ..Config::default()
        };
        assert!(
            save_api_key(&mut config, VerifiedKey("new-key".into()), directory.path()).is_err()
        );
        assert_eq!(config.key().unwrap(), "old-key");
    }
}
