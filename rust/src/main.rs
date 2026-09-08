#![cfg_attr(windows, windows_subsystem = "windows")]
use ai_dikte::{
    config::{self, Config},
    session, ui,
};
use anyhow::Result;
use clap::{Parser, Subcommand};
#[derive(Parser)]
#[command(version, about = "AI Dikte Rust development build")]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,
}
#[derive(Subcommand)]
enum Command {
    Daemon,
    #[cfg(not(windows))]
    Toggle,
    Setup,
    Doctor,
    /// Record until Ctrl+C, then type finalized text.
    Record,
}
fn main() {
    #[cfg(windows)]
    unsafe {
        windows_sys::Win32::System::Console::AttachConsole(u32::MAX);
    }
    if let Err(error) = run() {
        eprintln!("AI Dikte: {error:#}");
        #[cfg(windows)]
        {
            use windows_sys::Win32::UI::WindowsAndMessaging::*;
            let message: Vec<u16> = format!("AI Dikte: {error:#}\0").encode_utf16().collect();
            let title: Vec<u16> = "AI Dikte\0".encode_utf16().collect();
            unsafe {
                MessageBoxW(
                    std::ptr::null_mut(),
                    message.as_ptr(),
                    title.as_ptr(),
                    MB_ICONERROR,
                );
            }
        }
        std::process::exit(1);
    }
}
fn run() -> Result<()> {
    #[cfg(windows)]
    let default = if config::path()?.exists() {
        Command::Daemon
    } else {
        Command::Setup
    };
    #[cfg(not(windows))]
    let default = Command::Setup;
    match Args::parse().command.unwrap_or(default) {
        Command::Setup => ui::setup(),
        #[cfg(windows)]
        Command::Daemon => ai_dikte::windows::daemon(),
        #[cfg(not(windows))]
        Command::Daemon => tokio::runtime::Runtime::new()?.block_on(ai_dikte::linux::daemon()),
        #[cfg(not(windows))]
        Command::Toggle => tokio::runtime::Runtime::new()?.block_on(ai_dikte::linux::toggle()),
        Command::Doctor => {
            let path = config::path()?;
            let config = Config::load(&path)?;
            config.key()?;
            #[cfg(not(windows))]
            println!("Typing backend: {}", ai_dikte::output::driver()?);
            println!(
                "Configuration valid. Model: {}. Live API and microphone not tested.",
                ai_dikte::protocol::MODEL
            );
            Ok(())
        }
        Command::Record => tokio::runtime::Runtime::new()?.block_on(async {
            let config = Config::load(&config::path()?)?;
            let (stop, stopped) = tokio::sync::oneshot::channel();
            let interrupt = tokio::spawn(async {
                tokio::signal::ctrl_c().await?;
                let _ = stop.send(());
                Ok::<_, std::io::Error>(())
            });
            let result = session::run(config, stopped).await;
            interrupt.abort();
            result
        }),
    }
}
