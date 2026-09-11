use std::process::Command;

#[test]
fn test_cli_version() {
    let output = Command::new(env!("CARGO_BIN_EXE_ai-dikte"))
        .arg("--version")
        .output()
        .expect("Failed to execute ai-dikte --version");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.to_lowercase().contains("ai-dikte"));
}

#[test]
fn test_cli_self_test() {
    let output = Command::new(env!("CARGO_BIN_EXE_ai-dikte"))
        .arg("--self-test")
        .output()
        .expect("Failed to execute ai-dikte --self-test");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("passed"));
}

#[test]
fn test_cli_help() {
    let output = Command::new(env!("CARGO_BIN_EXE_ai-dikte"))
        .arg("--help")
        .output()
        .expect("Failed to execute ai-dikte --help");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("daemon"));
    assert!(stdout.contains("status"));
    assert!(stdout.contains("setup"));
    assert!(stdout.contains("menu"));
}

#[test]
fn test_cli_doctor() {
    let output = Command::new(env!("CARGO_BIN_EXE_ai-dikte"))
        .arg("doctor")
        .output()
        .expect("Failed to execute ai-dikte doctor");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("AI Dikte"));
    assert!(stdout.contains("Platform:"));
}

#[test]
fn test_cli_status() {
    let runtime = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_ai-dikte"))
        .arg("status")
        .env("XDG_RUNTIME_DIR", runtime.path())
        .output()
        .expect("Failed to execute ai-dikte status");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Background service:"));
}

#[test]
fn test_cli_check_config_fails_when_unconfigured() {
    let temp = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_ai-dikte"))
        .arg("check-config")
        .env("XDG_CONFIG_HOME", temp.path())
        .env("APPDATA", temp.path())
        .output()
        .expect("Failed to execute ai-dikte check-config");
    assert!(!output.status.success());
}

#[test]
fn malformed_configuration_is_not_replaced_by_onboarding() {
    let temp = tempfile::tempdir().unwrap();
    let directory = temp.path().join("ai-dikte");
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("config.json");
    let original = r#"{"language": "tr-TR", "unsupported": "preserve-me"}"#;
    std::fs::write(&path, original).unwrap();
    for command in [None, Some("menu"), Some("setup")] {
        let mut process = Command::new(env!("CARGO_BIN_EXE_ai-dikte"));
        if let Some(command) = command {
            process.arg(command);
        }
        let output = process
            .env("XDG_CONFIG_HOME", temp.path())
            .env("APPDATA", temp.path())
            .stdin(std::process::Stdio::null())
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("Invalid configuration"));
        assert!(!String::from_utf8_lossy(&output.stdout).contains("Gemini API Key"));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
    }
}

#[cfg(unix)]
#[test]
fn status_rejects_insecure_runtime_directory() {
    use std::os::unix::fs::PermissionsExt;
    let runtime = tempfile::tempdir().unwrap();
    let directory = runtime.path().join("ai-dikte-rust");
    std::fs::create_dir(&directory).unwrap();
    std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o777)).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_ai-dikte"))
        .arg("status")
        .env("XDG_RUNTIME_DIR", runtime.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Insecure runtime directory"));
    assert!(!String::from_utf8_lossy(&output.stdout).contains("NOT RUNNING"));
}

#[cfg(unix)]
#[test]
fn setup_keeps_existing_key_by_default_without_rewriting_settings() {
    use std::io::Write;
    use std::process::Stdio;
    let temp = tempfile::tempdir().unwrap();
    let directory = temp.path().join("ai-dikte");
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("config.json");
    let original = r#"{"api_key":"existing-secret","mode":"VERBATIM","language":"tr-TR"}"#;
    std::fs::write(&path, original).unwrap();
    let mut process = Command::new(env!("CARGO_BIN_EXE_ai-dikte"))
        .arg("setup")
        .env("XDG_CONFIG_HOME", temp.path())
        // Do not start or modify the user's service during this test.
        .env("PATH", temp.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    process
        .stdin
        .take()
        .unwrap()
        .write_all(b"maybe\n\n")
        .unwrap();
    let output = process.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Existing API key preserved"));
    assert!(!stdout.contains("Gemini API Key (hidden):"));
    assert!(!stdout.contains("existing-secret"));
    assert_eq!(std::fs::read_to_string(path).unwrap(), original);
}

#[cfg(unix)]
#[test]
fn cancelling_an_explicit_key_replacement_preserves_existing_settings() {
    use std::io::Write;
    use std::process::Stdio;
    let temp = tempfile::tempdir().unwrap();
    let directory = temp.path().join("ai-dikte");
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("config.json");
    let original = r#"{"api_key":"existing-secret"}"#;
    std::fs::write(&path, original).unwrap();
    let mut process = Command::new(env!("CARGO_BIN_EXE_ai-dikte"))
        .arg("setup")
        .env("XDG_CONFIG_HOME", temp.path())
        .env("PATH", temp.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    process.stdin.take().unwrap().write_all(b"y\n").unwrap();
    let output = process.wait_with_output().unwrap();
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Gemini API Key (hidden):"));
    assert!(!stdout.contains("existing-secret"));
    assert_eq!(std::fs::read_to_string(path).unwrap(), original);
}
