//! A single owner serializes hotkey events. Finishing never starts a new recording.
use crate::{
    config::{self, Config},
    session,
};
use tokio::sync::{mpsc, oneshot};
#[derive(Clone, Copy)]
pub enum Command {
    Toggle,
    Quit,
}
pub async fn run(mut commands: mpsc::Receiver<Command>, report: impl Fn(&str)) {
    while let Some(command) = commands.recv().await {
        if matches!(command, Command::Quit) {
            break;
        }
        let config = match config::path().and_then(|path| Config::load(&path)) {
            Ok(config) => config,
            Err(error) => {
                report(&format!("Error: {error:#}"));
                continue;
            }
        };
        let (stop, stopped) = oneshot::channel();
        let mut recording = Box::pin(session::run(config, stopped));
        report("Recording — Win+Z to stop");
        let quitting = tokio::select! {
            result = &mut recording => {
                report(&match result { Ok(()) => "Ready".into(), Err(e) => format!("Error: {e:#}") });
                continue;
            }
            command = commands.recv() => matches!(command, None | Some(Command::Quit)),
        };
        let _ = stop.send(());
        report("Finishing transcription…");
        loop {
            tokio::select! {
                result = &mut recording => {
                    report(&match result { Ok(()) => "Ready".into(), Err(e) => format!("Error: {e:#}") });
                    break;
                }
                command = commands.recv(), if !quitting => {
                    if matches!(command, None | Some(Command::Quit)) {
                        // Keep capture cleanup owned by this future on exit.
                        let _ = recording.await;
                        return;
                    }
                    report("Still finishing; wait for Ready");
                }
            }
        }
        if quitting {
            break;
        }
    }
}
