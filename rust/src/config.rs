use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum Mode {
    #[default]
    Smart,
    Verbatim,
}
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Notifications {
    #[default]
    All,
    None,
}

// Never derive Debug: Linux configuration contains the API key.
#[derive(Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub language: String,
    pub mode: Mode,
    pub custom_vocabulary: Vec<String>,
    pub audio_cue: bool,
    pub notify_mode: Notifications,
    pub input_device: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            language: "tr-TR".into(),
            mode: Mode::Smart,
            custom_vocabulary: vec![],
            audio_cue: false,
            notify_mode: Notifications::All,
            input_device: None,
            api_key: None,
        }
    }
}
impl Config {
    pub fn validate(&mut self) -> Result<()> {
        self.language = self.language.trim().to_owned();
        if self.language.is_empty() {
            bail!("Language cannot be empty");
        }
        let mut unique = Vec::new();
        for term in &self.custom_vocabulary {
            let term = term.trim();
            if !term.is_empty() && !unique.iter().any(|s| s == term) {
                unique.push(term.to_owned());
            }
        }
        if unique.len() > 1000 {
            bail!("At most 1000 vocabulary terms are supported");
        }
        self.custom_vocabulary = unique;
        Ok(())
    }
    pub fn load(path: &Path) -> Result<Self> {
        let raw =
            std::fs::read_to_string(path).context("Cannot read configuration; run setup first")?;
        // Do not include JSON parser errors: an invalid enum could contain a secret.
        let mut config: Self = serde_json::from_str(&raw)
            .map_err(|_| anyhow::anyhow!("Invalid configuration JSON or unsupported setting"))?;
        config.validate()?;
        Ok(config)
    }
    pub fn save(&self, path: &Path) -> Result<()> {
        let mut checked = self.clone();
        checked.validate()?;
        #[cfg(windows)]
        {
            checked.api_key = None;
        }
        let parent = path.parent().context("Config path has no parent")?;
        std::fs::create_dir_all(parent)?;
        let mut file = tempfile::NamedTempFile::new_in(parent)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            file.as_file()
                .set_permissions(std::fs::Permissions::from_mode(0o600))?;
        }
        serde_json::to_writer_pretty(&mut file, &checked)?;
        file.write_all(b"\n")?;
        file.as_file().sync_all()?;
        file.persist(path)
            .context("Cannot atomically replace configuration")?;
        #[cfg(unix)]
        {
            std::fs::File::open(parent)?.sync_all()?;
        }
        Ok(())
    }
    pub fn key(&self) -> Result<String> {
        #[cfg(windows)]
        let key = credentials::read()?;
        #[cfg(not(windows))]
        let key = self.api_key.clone().unwrap_or_default();
        let key = key.trim().to_owned();
        if key.is_empty() {
            bail!("API key missing; run setup first");
        }
        Ok(key)
    }
}
pub fn path() -> Result<PathBuf> {
    #[cfg(windows)]
    let root = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .context("APPDATA is missing")?;
    #[cfg(not(windows))]
    let root = if let Some(root) = std::env::var_os("XDG_CONFIG_HOME") {
        PathBuf::from(root)
    } else {
        PathBuf::from(std::env::var_os("HOME").context("HOME is missing")?).join(".config")
    };
    if !root.is_absolute() {
        bail!("Configuration directory must be absolute");
    }
    Ok(root.join("ai-dikte/config.json"))
}

#[cfg(windows)]
pub mod credentials {
    use anyhow::{Result, bail};
    use windows_sys::Win32::{Foundation::GetLastError, Security::Credentials::*};
    fn target() -> Vec<u16> {
        "Yakrel/AI-Dikte/GoogleAI\0".encode_utf16().collect()
    }
    pub fn read() -> Result<String> {
        read_optional()?.ok_or_else(|| anyhow::anyhow!("API key missing; run setup first"))
    }
    pub fn read_optional() -> Result<Option<String>> {
        let target = target();
        let mut cred = std::ptr::null_mut();
        unsafe {
            if CredReadW(target.as_ptr(), CRED_TYPE_GENERIC, 0, &mut cred) == 0 {
                if GetLastError() == windows_sys::Win32::Foundation::ERROR_NOT_FOUND {
                    return Ok(None);
                }
                bail!(
                    "Cannot read API key from Credential Manager (error {})",
                    GetLastError()
                );
            }
            if (*cred).CredentialBlobSize == 0 || (*cred).CredentialBlob.is_null() {
                CredFree(cred.cast());
                bail!("API key credential is empty");
            }
            let bytes = std::slice::from_raw_parts(
                (*cred).CredentialBlob,
                (*cred).CredentialBlobSize as usize,
            )
            .to_vec();
            CredFree(cred.cast());
            String::from_utf8(bytes)
                .map(Some)
                .map_err(|_| anyhow::anyhow!("Invalid credential encoding"))
        }
    }
    pub fn restore(key: Option<&str>) -> Result<()> {
        if let Some(key) = key {
            return write(key);
        }
        if unsafe { CredDeleteW(target().as_ptr(), CRED_TYPE_GENERIC, 0) } == 0
            && unsafe { GetLastError() } != windows_sys::Win32::Foundation::ERROR_NOT_FOUND
        {
            bail!("Cannot remove API key from Credential Manager");
        }
        Ok(())
    }
    pub fn write(key: &str) -> Result<()> {
        let mut target = target();
        let mut bytes: Vec<u8> = key.as_bytes().to_vec();
        if bytes.is_empty() || bytes.len() > 2560 {
            bail!("Invalid API key length");
        }
        let mut cred: CREDENTIALW = unsafe { std::mem::zeroed() };
        cred.Type = CRED_TYPE_GENERIC;
        cred.TargetName = target.as_mut_ptr();
        cred.CredentialBlobSize = bytes.len() as u32;
        cred.CredentialBlob = bytes.as_mut_ptr();
        cred.Persist = CRED_PERSIST_LOCAL_MACHINE;
        if unsafe { CredWriteW(&cred, 0) } == 0 {
            bail!("Cannot save API key in Credential Manager");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_invalid_types_and_unknown_settings() {
        for input in [
            r#"{"audio_cue":"false"}"#,
            r#"{"mode":"unknown"}"#,
            r#"{"input_device":-1}"#,
            r#"{"fallback":true}"#,
        ] {
            assert!(serde_json::from_str::<Config>(input).is_err());
        }
    }
    #[test]
    fn vocabulary_preserves_order_and_case() {
        let mut c = Config {
            custom_vocabulary: vec![" Rust ".into(), "".into(), "Rust".into(), "rust".into()],
            ..Config::default()
        };
        c.validate().unwrap();
        assert_eq!(c.custom_vocabulary, ["Rust", "rust"]);
    }
    #[test]
    fn atomic_replace_and_private_permissions() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        Config::default().save(&path).unwrap();
        let mut c = Config::load(&path).unwrap();
        c.language = "en-US".into();
        c.save(&path).unwrap();
        assert_eq!(Config::load(&path).unwrap().language, "en-US");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }
}
