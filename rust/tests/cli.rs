use std::process::Command;

#[test]
fn test_cli_self_test() {
    let output = Command::new(env!("CARGO_BIN_EXE_ai-dikte"))
        .arg("--self-test")
        .output()
        .expect("Failed to execute ai-dikte --self-test");
    assert!(output.status.success());
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
    assert!(!stdout.contains("existing-secret"));
    assert_eq!(std::fs::read_to_string(path).unwrap(), original);
}

#[cfg(target_os = "linux")]
#[test]
fn interrupted_hidden_input_restores_terminal() {
    use std::{
        fs::File,
        io::Write,
        os::fd::{AsRawFd, FromRawFd},
        process::{Child, Stdio},
        time::{Duration, Instant},
    };

    struct ChildGuard(Option<Child>);
    impl Drop for ChildGuard {
        fn drop(&mut self) {
            if let Some(child) = self.0.as_mut() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }

    for signal in [libc::SIGINT, libc::SIGTERM] {
        let temp = tempfile::tempdir().unwrap();
        let mut master = -1;
        let mut slave = -1;
        assert_eq!(
            unsafe {
                libc::openpty(
                    &mut master,
                    &mut slave,
                    std::ptr::null_mut(),
                    std::ptr::null(),
                    std::ptr::null(),
                )
            },
            0
        );
        let mut master = unsafe { File::from_raw_fd(master) };
        let slave = unsafe { File::from_raw_fd(slave) };
        let fd = slave.as_raw_fd();
        let mut original = unsafe { std::mem::zeroed::<libc::termios>() };
        assert_eq!(unsafe { libc::tcgetattr(fd, &mut original) }, 0);
        original.c_lflag |= libc::ECHO | libc::ECHONL;
        assert_eq!(unsafe { libc::tcsetattr(fd, libc::TCSANOW, &original) }, 0);
        let original_flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        let mut child = ChildGuard(Some(
            Command::new(env!("CARGO_BIN_EXE_ai-dikte"))
                .arg("setup")
                .env("XDG_CONFIG_HOME", temp.path())
                .env("PATH", temp.path())
                .stdin(Stdio::from(slave.try_clone().unwrap()))
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap(),
        ));
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let mut current = unsafe { std::mem::zeroed::<libc::termios>() };
            assert_eq!(unsafe { libc::tcgetattr(fd, &mut current) }, 0);
            if current.c_lflag & (libc::ECHO | libc::ECHONL) == 0 {
                break;
            }
            assert!(child.0.as_mut().unwrap().try_wait().unwrap().is_none());
            assert!(Instant::now() < deadline, "Hidden prompt did not open");
            std::thread::sleep(Duration::from_millis(10));
        }
        master.write_all(b"partial-secret").unwrap();
        assert_eq!(
            unsafe { libc::kill(child.0.as_ref().unwrap().id() as i32, signal) },
            0
        );
        let deadline = Instant::now() + Duration::from_secs(5);
        while child.0.as_mut().unwrap().try_wait().unwrap().is_none() {
            assert!(Instant::now() < deadline, "Interrupted prompt did not exit");
            std::thread::sleep(Duration::from_millis(10));
        }
        let output = child.0.take().unwrap().wait_with_output().unwrap();
        assert!(!output.status.success());
        let mut restored = unsafe { std::mem::zeroed::<libc::termios>() };
        assert_eq!(unsafe { libc::tcgetattr(fd, &mut restored) }, 0);
        assert_eq!(restored.c_lflag, original.c_lflag);
        assert_eq!(restored.c_iflag, original.c_iflag);
        assert_eq!(restored.c_oflag, original.c_oflag);
        assert_eq!(restored.c_cflag, original.c_cflag);
        assert_eq!(restored.c_cc, original.c_cc);
        assert_eq!(unsafe { libc::fcntl(fd, libc::F_GETFL) }, original_flags);
        assert!(!temp.path().join("ai-dikte/config.json").exists());
        assert!(!String::from_utf8_lossy(&output.stdout).contains("partial-secret"));
        assert!(!String::from_utf8_lossy(&output.stderr).contains("partial-secret"));
    }
}
