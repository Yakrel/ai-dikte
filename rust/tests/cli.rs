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
    let output = Command::new(env!("CARGO_BIN_EXE_ai-dikte"))
        .arg("status")
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
