use crate::{audio, config::Config, live, output};
use anyhow::Result;
use futures_util::{
    FutureExt,
    future::{Fuse, FusedFuture},
};
use tokio::sync::oneshot;
/// No injection until both the recorder and protocol complete successfully.
pub async fn run(config: Config, stopped: oneshot::Receiver<()>) -> Result<()> {
    let (_cancel, cancelled) = oneshot::channel();
    run_cancellable(config, stopped, cancelled).await
}

pub async fn run_cancellable(
    config: Config,
    stopped: oneshot::Receiver<()>,
    mut cancelled: oneshot::Receiver<()>,
) -> Result<()> {
    let key = config.key()?;
    let audio::Capture { audio, stop, task } = audio::start(&config)?;
    let transcription = live::transcribe(&config, &key, audio);
    let Some(text) =
        collect_cancellable(stop, task, transcription, stopped, &mut cancelled).await?
    else {
        return Ok(());
    };
    tokio::select! {
        biased;
        _ = &mut cancelled => return Ok(()),
        result = output::type_text(&text) => result?,
    }
    Ok(())
}

async fn collect_cancellable(
    stop: oneshot::Sender<()>,
    task: tokio::task::JoinHandle<Result<()>>,
    transcription: impl std::future::Future<Output = Result<String>>,
    stopped: oneshot::Receiver<()>,
    cancelled: &mut oneshot::Receiver<()>,
) -> Result<Option<String>> {
    let mut task = task.fuse();
    // Dropping collect releases its stop sender and audio receiver. Retain the
    // capture handle here so cancellation also waits for microphone cleanup.
    tokio::select! {
        biased;
        _ = cancelled => (),
        result = collect(stop, &mut task, transcription, stopped) => return result.map(Some),
    }
    if !task.is_terminated() {
        task.await??;
    }
    Ok(None)
}

async fn collect(
    stop: oneshot::Sender<()>,
    mut task: &mut Fuse<tokio::task::JoinHandle<Result<()>>>,
    transcription: impl std::future::Future<Output = Result<String>>,
    stopped: oneshot::Receiver<()>,
) -> Result<String> {
    tokio::pin!(transcription);
    tokio::select! {
        biased;
        _ = stopped => { let _ = stop.send(()); }
        result = &mut transcription => {
            let _ = stop.send(());
            let _ = task.await;
            result?;
            anyhow::bail!("Transcription ended while recording was active");
        }
        result = &mut task => {
            result??;
            anyhow::bail!("Recorder ended before stop was requested");
        }
    }
    let (recorded, text) = tokio::join!(task, transcription);
    recorded??;
    text
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    #[tokio::test]
    async fn cancellation_releases_capture_without_returning_text() {
        for finishing in [false, true] {
            let (stop, stop_capture) = oneshot::channel();
            let (finish, stopped) = oneshot::channel();
            if finishing {
                finish.send(()).unwrap();
            }
            let (cancel, mut cancelled) = oneshot::channel();
            let (released, cleanup) = oneshot::channel();
            let capture = tokio::spawn(async move {
                let _ = stop_capture.await;
                released.send(()).unwrap();
                Ok(())
            });
            let operation = collect_cancellable(
                stop,
                capture,
                std::future::pending(),
                stopped,
                &mut cancelled,
            );
            let request = async move {
                tokio::task::yield_now().await;
                cancel.send(()).unwrap();
            };
            let (result, ()) = tokio::time::timeout(Duration::from_secs(1), async {
                tokio::join!(operation, request)
            })
            .await
            .unwrap();
            assert!(result.unwrap().is_none());
            cleanup.await.unwrap();
        }
    }
    #[tokio::test]
    async fn cancellation_after_capture_finished_does_not_poll_join_handle_twice() {
        let (stop, stop_capture) = oneshot::channel();
        let (finish, stopped) = oneshot::channel();
        finish.send(()).unwrap();
        let (cancel, mut cancelled) = oneshot::channel();
        let capture = tokio::spawn(async move {
            stop_capture.await?;
            Ok(())
        });
        let transcript = async move {
            tokio::time::sleep(Duration::from_millis(20)).await;
            cancel.send(()).unwrap();
            std::future::pending::<Result<String>>().await
        };
        let result = tokio::time::timeout(
            Duration::from_secs(1),
            collect_cancellable(stop, capture, transcript, stopped, &mut cancelled),
        )
        .await
        .unwrap();
        assert!(result.unwrap().is_none());
    }
    #[tokio::test]
    async fn immediate_stop_drains_capture_before_returning_text() {
        let (stop, stop_capture) = oneshot::channel();
        let (finish, stopped) = oneshot::channel();
        finish.send(()).unwrap();
        let (audio, mut received) = tokio::sync::mpsc::channel(1);
        let capture = tokio::spawn(async move {
            stop_capture.await?;
            audio.send("tail").await?;
            Ok(())
        });
        let mut capture = capture.fuse();
        let transcript = async move { Ok(received.recv().await.unwrap().to_owned()) };
        assert_eq!(
            collect(stop, &mut capture, transcript, stopped)
                .await
                .unwrap(),
            "tail"
        );
    }
    #[tokio::test]
    async fn recorder_failure_never_returns_a_successful_transcript() {
        let (stop, stop_capture) = oneshot::channel();
        let (finish, stopped) = oneshot::channel();
        finish.send(()).unwrap();
        let capture = tokio::spawn(async move {
            stop_capture.await?;
            anyhow::bail!("device disconnected");
        });
        let mut capture = capture.fuse();
        assert!(
            collect(stop, &mut capture, async { Ok("partial".into()) }, stopped)
                .await
                .is_err()
        );
    }
    #[tokio::test]
    async fn network_failure_releases_capture() {
        let (stop, stop_capture) = oneshot::channel();
        let (_finish, stopped) = oneshot::channel();
        let capture = tokio::spawn(async move {
            stop_capture.await?;
            Ok(())
        });
        let mut capture = capture.fuse();
        let result = tokio::time::timeout(
            Duration::from_secs(1),
            collect(
                stop,
                &mut capture,
                async { anyhow::bail!("network failed") },
                stopped,
            ),
        )
        .await
        .unwrap();
        assert!(result.is_err());
    }
}
