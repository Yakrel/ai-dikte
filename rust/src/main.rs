#![cfg_attr(windows, windows_subsystem = "windows")]

use ai_dikte::{
    config::{self, Config},
    session, ui,
};
use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about = "AI Dikte - Minimal Dictation with Gemini")]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,
    #[arg(long)]
    self_test: bool,
}

#[derive(Subcommand)]
enum Command {
    /// Background hotkey listener daemon
    Daemon,
    /// (Linux) Start or stop active recording
    #[cfg(not(windows))]
    Toggle,
    /// First-run setup wizard (API key entry)
    Setup,
    /// Interactive console menu
    Menu,
    /// Show service status and configuration
    Status,
    /// System diagnostics report
    Diagnostics,
    /// Local configuration and environment report (live API test: menu option 2)
    Doctor,
    /// Show session log
    Logs,
    /// Isolated runtime self-test without network or credentials
    SelfTest,
    /// Check local configuration and system devices
    CheckConfig,
    #[cfg(not(windows))]
    ShortcutInstall,
    #[cfg(not(windows))]
    ShortcutRemove,
    /// Record from microphone until Ctrl+C, then type text
    Record,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("AI Dikte: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args = match Args::try_parse() {
        Ok(args) => args,
        Err(error) => {
            #[cfg(windows)]
            ai_dikte::windows::ensure_console();
            error.exit();
        }
    };
    if args.self_test {
        #[cfg(windows)]
        ai_dikte::windows::ensure_console();
        return ai_dikte::diagnostics::self_test();
    }

    let is_daemon = matches!(args.command, Some(Command::Daemon));
    if !is_daemon {
        #[cfg(windows)]
        ai_dikte::windows::ensure_console();
    }

    let command = match args.command {
        Some(cmd) => cmd,
        None => {
            let path = config::path()?;
            if !path.exists() {
                return ui::first_run_setup();
            }
            let config = Config::load(&path)?;
            if config.key().is_err() {
                return ui::first_run_setup();
            }
            Command::Menu
        }
    };

    match command {
        Command::SelfTest => ai_dikte::diagnostics::self_test(),
        Command::CheckConfig => ai_dikte::diagnostics::check_configuration(),
        #[cfg(not(windows))]
        Command::ShortcutInstall => ai_dikte::shortcut::update(true),
        #[cfg(not(windows))]
        Command::ShortcutRemove => ai_dikte::shortcut::update(false),
        // Explicit setup is intentionally one-shot. Installers can wait for it
        // without being trapped inside the interactive menu afterwards.
        Command::Setup => ui::setup(),
        Command::Menu => ui::menu(),
        Command::Diagnostics => ui::diagnostics_menu(),
        Command::Status => {
            println!(
                "Background service: {}",
                if ai_dikte::background::running()? {
                    "RUNNING"
                } else {
                    "NOT RUNNING"
                }
            );
            println!("{}", ai_dikte::diagnostics::report());
            Ok(())
        }
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
            println!("{}", ai_dikte::diagnostics::report());
            Ok(())
        }
        Command::Logs => {
            println!("{}", ai_dikte::diagnostics::log()?);
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
