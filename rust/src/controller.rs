//! A single owner serializes hotkey events. Finishing never starts a new recording.
use crate::{
    config::{self, Config},
    protocol, session,
};
use tokio::sync::{mpsc, oneshot};
#[derive(Clone, Copy)]
pub enum Command {
    Toggle,
    Quit,
}
pub async fn run(mut commands: mpsc::Receiver<Command>, report: impl Fn(&str)) {
    let report = |message: &str| {
        if let Err(error) = crate::diagnostics::append(message) {
            eprintln!("Cannot write session log: {error}");
        }
        report(message);
    };
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
        let (cancel, cancelled) = oneshot::channel();
        let recording = session::run_cancellable(config, stopped, cancelled);
        report("Recording — Win+Z to stop");
        if control_recording(&mut commands, recording, stop, cancel, &report).await {
            break;
        }
    }
}

// Returns true only when the controller must exit, after capture cleanup.
async fn control_recording(
    commands: &mut mpsc::Receiver<Command>,
    recording: impl Future<Output = anyhow::Result<()>>,
    stop: oneshot::Sender<()>,
    cancel: oneshot::Sender<()>,
    report: impl Fn(&str),
) -> bool {
    tokio::pin!(recording);
    let quitting = tokio::select! {
        biased;
        command = commands.recv() => matches!(command, None | Some(Command::Quit)),
        result = &mut recording => {
            report(&format_result(result));
            return false;
        }
    };
    if quitting {
        let _ = cancel.send(());
        drop(stop);
        if let Err(error) = recording.await {
            report(&format_result(Err(error)));
        }
        return true;
    }
    let _ = stop.send(());
    report("Finishing transcription…");
    loop {
        tokio::select! {
            biased;
            command = commands.recv() => {
                if matches!(command, None | Some(Command::Quit)) {
                    let _ = cancel.send(());
                    // Keep capture cleanup owned by this future on exit.
                    if let Err(error) = recording.await {
                        report(&format_result(Err(error)));
                    }
                    return true;
                }
                report("Still finishing; wait for Ready");
            }
            result = &mut recording => {
                report(&format_result(result));
                return false;
            }
        }
    }
}

fn format_result(result: anyhow::Result<()>) -> String {
    match result {
        Ok(()) => "Ready".into(),
        Err(e) if protocol::is_no_speech(&e) => "No speech detected".into(),
        Err(e) => format!("Error: {e:#}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, time::Duration};

    #[tokio::test]
    async fn quit_awaits_cleanup_without_finishing_or_success_feedback() {
        for finishing in [false, true] {
            let (send, mut commands) = mpsc::channel(2);
            let (stop, stopped) = oneshot::channel();
            let (cancel, cancelled) = oneshot::channel();
            let (release, cleanup) = oneshot::channel();
            let (cleaned, cleanup_done) = oneshot::channel();
            let reports = RefCell::new(Vec::new());
            if finishing {
                send.send(Command::Toggle).await.unwrap();
            } else {
                send.send(Command::Quit).await.unwrap();
            }
            let recording = async {
                if finishing {
                    stopped.await.unwrap();
                    send.send(Command::Quit).await.unwrap();
                }
                cancelled.await.unwrap();
                if finishing {
                    // Finishing feedback was valid before Quit arrived.
                    reports.borrow_mut().clear();
                } else {
                    assert!(reports.borrow().is_empty());
                }
                cleanup.await.unwrap();
                cleaned.send(()).unwrap();
                Ok(())
            };
            let request = async {
                tokio::task::yield_now().await;
                release.send(()).unwrap();
            };
            let (quitting, ()) = tokio::time::timeout(Duration::from_secs(1), async {
                tokio::join!(
                    control_recording(&mut commands, recording, stop, cancel, |message| {
                        reports.borrow_mut().push(message.to_owned());
                    }),
                    request
                )
            })
            .await
            .unwrap();
            assert!(quitting);
            cleanup_done.await.unwrap();
            assert!(reports.borrow().is_empty());
        }
    }

    #[test]
    fn ordinary_errors_that_mention_transcription_are_not_reclassified() {
        let err = anyhow::anyhow!("upstream returned no transcription metadata");
        assert_eq!(
            format_result(Err(err)),
            "Error: upstream returned no transcription metadata"
        );
    }
}
