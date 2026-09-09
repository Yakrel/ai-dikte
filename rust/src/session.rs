use crate::{audio, config::Config, live, output};
use anyhow::Result;
use tokio::sync::oneshot;
/// No injection until both the recorder and protocol complete successfully.
pub async fn run(config: Config, stopped: oneshot::Receiver<()>) -> Result<()> {
    let key = config.key()?;
    if config.audio_cue {
        crate::cue::play(false).await?;
    }
    let audio::Capture { audio, stop, task } = audio::start(&config)?;
    let transcription = live::transcribe(&config, &key, audio);
    let text = collect(stop, task, transcription, stopped).await?;
    output::type_text(&text).await?;
    if config.audio_cue
        && let Err(error) = crate::cue::play(true).await
    {
        eprintln!("Text typed, but audio cue failed: {error}");
    }
    Ok(())
}

async fn collect(
    stop: oneshot::Sender<()>,
    mut task: tokio::task::JoinHandle<Result<()>>,
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
        let transcript = async move { Ok(received.recv().await.unwrap().to_owned()) };
        assert_eq!(
            collect(stop, capture, transcript, stopped).await.unwrap(),
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
        assert!(
            collect(stop, capture, async { Ok("partial".into()) }, stopped)
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
        let result = tokio::time::timeout(
            Duration::from_secs(1),
            collect(
                stop,
                capture,
                async { anyhow::bail!("network failed") },
                stopped,
            ),
        )
        .await
        .unwrap();
        assert!(result.is_err());
    }
}
