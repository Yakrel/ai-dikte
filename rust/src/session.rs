use crate::{audio, config::Config, live, output};
use anyhow::Result;
use tokio::sync::oneshot;
/// No injection until both the recorder and protocol complete successfully.
pub async fn run(config: Config, stopped: oneshot::Receiver<()>) -> Result<()> {
    let key = config.key()?;
    if config.audio_cue {
        crate::cue::play(false).await?;
    }
    let audio::Capture {
        audio,
        stop,
        mut task,
    } = audio::start(&config)?;
    let mut transcription = Box::pin(live::transcribe(&config, &key, audio));
    tokio::select! {
        result = &mut transcription => {
            let _ = stop.send(());
            // Wait for native capture to release the microphone even on network failure.
            let _ = task.await;
            result?;
            anyhow::bail!("Transcription ended while recording was active");
        }
        result = &mut task => {
            result??;
            anyhow::bail!("Recorder ended before stop was requested");
        }
        _ = stopped => { let _ = stop.send(()); }
    }
    // Poll concurrently: the capture shutdown can still send buffered audio.
    let (recorded, text) = tokio::join!(task, transcription);
    recorded??;
    output::type_text(&text?).await?;
    if config.audio_cue
        && let Err(error) = crate::cue::play(true).await
    {
        eprintln!("Text typed, but audio cue failed: {error}");
    }
    Ok(())
}
