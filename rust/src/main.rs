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
    #[arg(long)]
    self_test: bool,
}
#[derive(Subcommand)]
enum Command {
    Daemon,
    #[cfg(not(windows))]
    Toggle,
    Setup,
    Doctor,
    Logs,
    SelfTest,
    CheckConfig,
    #[cfg(not(windows))]
    ShortcutInstall,
    #[cfg(not(windows))]
    ShortcutRemove,
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
            if !matches!(
                std::env::args().nth(1).as_deref(),
                Some("check-config" | "self-test" | "--self-test")
            ) {
                unsafe {
                    MessageBoxW(
                        std::ptr::null_mut(),
                        message.as_ptr(),
                        title.as_ptr(),
                        MB_ICONERROR,
                    );
                }
            }
        }
        std::process::exit(1);
    }
}
fn run() -> Result<()> {
    let args = Args::parse();
    if args.self_test {
        return ai_dikte::diagnostics::self_test();
    }
    #[cfg(windows)]
    let command = match args.command {
        None => {
            if !config::path()?.exists() {
                ui::setup()?;
                if !config::path()?.exists() {
                    return Ok(());
                }
            }
            Command::Daemon
        }
        Some(command) => command,
    };
    #[cfg(not(windows))]
    let command = args.command.unwrap_or(Command::Setup);
    match command {
        Command::SelfTest => ai_dikte::diagnostics::self_test(),
        Command::CheckConfig => ai_dikte::diagnostics::check_configuration(),
        #[cfg(not(windows))]
        Command::ShortcutInstall => ai_dikte::shortcut::update(true),
        #[cfg(not(windows))]
        Command::ShortcutRemove => ai_dikte::shortcut::update(false),
        Command::Setup => ui::setup(),
        #[cfg(windows)]
        Command::Daemon => {
            ai_dikte::diagnostics::check_configuration()?;
            ai_dikte::windows::daemon()
        }
        #[cfg(not(windows))]
        Command::Daemon => {
            ai_dikte::diagnostics::check_configuration()?;
            tokio::runtime::Runtime::new()?.block_on(ai_dikte::linux::daemon())
        }
        #[cfg(not(windows))]
        Command::Toggle => tokio::runtime::Runtime::new()?.block_on(ai_dikte::linux::toggle()),
        Command::Doctor => {
            let report = ai_dikte::diagnostics::report();
            #[cfg(windows)]
            {
                ui::text_window("AI Dikte — Diagnostics", report)
            }
            #[cfg(not(windows))]
            {
                println!("{report}");
                Ok(())
            }
        }
        Command::Logs => ui::text_window("AI Dikte — Session log", ai_dikte::diagnostics::log()?),
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
