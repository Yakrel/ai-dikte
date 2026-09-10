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
    Menu,
    Status,
    Diagnostics,
    LogView,
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
    if let Err(error) = run() {
        eprintln!("AI Dikte: {error:#}");
        std::process::exit(1);
    }
}
fn run() -> Result<()> {
    let args = Args::parse();
    if args.self_test {
        return ai_dikte::diagnostics::self_test();
    }
    #[cfg(windows)]
    let default_launch = args.command.is_none();
    let command = args.command.unwrap_or(Command::Menu);
    let result = match command {
        Command::SelfTest => ai_dikte::diagnostics::self_test(),
        Command::CheckConfig => ai_dikte::diagnostics::check_configuration(),
        #[cfg(not(windows))]
        Command::ShortcutInstall => ai_dikte::shortcut::update(true),
        #[cfg(not(windows))]
        Command::ShortcutRemove => ai_dikte::shortcut::update(false),
        Command::Setup => ui::setup(),
        Command::Menu => ui::menu(),
        Command::Diagnostics => ui::diagnostics_menu(),
        Command::LogView => ui::logs_menu(),
        Command::Status => {
            println!(
                "Background app: {}",
                if ai_dikte::background::running()? {
                    "running"
                } else {
                    "not running"
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
    };
    #[cfg(windows)]
    if result.is_ok() && default_launch && config::path()?.exists() {
        ai_dikte::diagnostics::check_configuration()?;
        ai_dikte::windows::ensure_background()?;
    }
    result
}
