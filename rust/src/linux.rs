use crate::controller::{self, Command};
use anyhow::{Context, Result, bail};
use std::{
    fs::{File, OpenOptions},
    os::unix::{
        fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
        io::AsRawFd,
    },
    path::PathBuf,
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{UnixListener, UnixStream},
    sync::mpsc,
};
fn runtime_dir() -> Result<PathBuf> {
    let root =
        PathBuf::from(std::env::var_os("XDG_RUNTIME_DIR").context("XDG_RUNTIME_DIR is required")?);
    if !root.is_absolute() {
        bail!("XDG_RUNTIME_DIR must be absolute");
    }
    let path = root.join("ai-dikte-rust");
    match std::fs::create_dir(&path) {
        Ok(()) => std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700))?,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
        Err(e) => return Err(e.into()),
    }
    let meta = std::fs::symlink_metadata(&path)?;
    if !meta.is_dir()
        || meta.uid() != unsafe { libc::geteuid() }
        || meta.permissions().mode() & 0o077 != 0
    {
        bail!("Insecure runtime directory");
    }
    Ok(path)
}
struct SocketGuard {
    path: PathBuf,
    _lock: File,
}
impl Drop for SocketGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}
pub fn running() -> Result<bool> {
    let path = runtime_dir()?.join("daemon.lock");
    let lock = match OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
    {
        Ok(lock) => lock,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(e.into()),
    };
    if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
        return Ok(false);
    }
    let error = std::io::Error::last_os_error();
    if error.kind() == std::io::ErrorKind::WouldBlock {
        Ok(true)
    } else {
        Err(error.into())
    }
}

pub async fn toggle() -> Result<()> {
    let path = runtime_dir()?.join("control.sock");
    tokio::time::timeout(Duration::from_secs(2), async {
        let mut socket = UnixStream::connect(path)
            .await
            .context("Rust daemon is not running; start ai-dikte daemon")?;
        socket.write_all(b"T").await?;
        if socket.read_u8().await? != b'K' {
            bail!("Daemon did not accept toggle");
        }
        Ok::<_, anyhow::Error>(())
    })
    .await
    .context("Daemon toggle timed out")?
}
pub async fn daemon() -> Result<()> {
    let dir = runtime_dir()?;
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(dir.join("daemon.lock"))?;
    if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        bail!("Rust daemon is already running");
    }
    let path = dir.join("control.sock");
    match std::fs::remove_file(&path) {
        Ok(()) => (),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
        Err(e) => return Err(e.into()),
    }
    let listener = UnixListener::bind(&path)?;
    let _guard = SocketGuard { path, _lock: lock };
    let (tx, rx) = mpsc::channel(16);
    let worker = tokio::spawn(controller::run(rx, |message| {
        eprintln!("AI Dikte: {message}");
        let enabled = crate::config::path()
            .and_then(|path| crate::config::Config::load(&path))
            .map(|config| config.notify_mode == crate::config::Notifications::All)
            .unwrap_or(true);
        if enabled || message.starts_with("Error:") {
            let message = message.to_owned();
            tokio::spawn(async move {
                let mut command = tokio::process::Command::new("notify-send");
                command
                    .args(["-a", "AI Dikte", "AI Dikte", &message])
                    .kill_on_drop(true);
                match tokio::time::timeout(Duration::from_secs(3), command.status()).await {
                    Ok(Ok(status)) if status.success() => (),
                    _ => eprintln!("AI Dikte: notification failed"),
                }
            });
        }
    }));
    let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => break,
            _ = term.recv() => break,
            incoming = listener.accept() => {
                let (mut socket, _) = incoming?;
                if socket.peer_cred()?.uid() != unsafe { libc::geteuid() } { continue; }
                let tx = tx.clone();
                // One short request at a time; a client cannot allocate unbounded tasks.
                let result = tokio::time::timeout(Duration::from_secs(1), async {
                    if socket.read_u8().await? != b'T' { bail!("Invalid daemon command"); }
                    tx.try_send(Command::Toggle).context("Daemon command queue is full")?;
                    socket.write_all(b"K").await?;
                    Ok::<_, anyhow::Error>(())
                }).await;
                if !matches!(result, Ok(Ok(()))) { eprintln!("AI Dikte: control request rejected"); }
            }
        }
    }
    tx.send(Command::Quit).await?;
    worker.await?;
    Ok(())
}
