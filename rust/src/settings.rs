#[cfg(windows)]
use crate::config;
use crate::config::Config;
use anyhow::Result;
use std::path::Path;

// Every save must pass live validation, even if the key or settings are unchanged.
// Validation runs before either the credential store or the config file is touched.
pub fn save_verified(
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
    {
        let previous = config::credentials::read_optional()?;
        save_with_credential(
            &config,
            &key,
            path,
            previous.as_deref(),
            config::credentials::restore,
        )
    }
    #[cfg(not(windows))]
    {
        config.api_key = Some(key);
        config.save(path)
    }
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
