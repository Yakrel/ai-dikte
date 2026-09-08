use anyhow::Result;
/// Short, tapered PCM cue. A notification failure is reported independently;
/// it must not cause already typed text to be retried.
fn wave(finish: bool) -> Vec<u8> {
    let count = 2400u32;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + count * 2).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&16000u32.to_le_bytes());
    bytes.extend_from_slice(&32000u32.to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&(count * 2).to_le_bytes());
    let frequency = if finish { 660.0 } else { 880.0 };
    for i in 0..count {
        let taper = (i as f32 / 160.0).min((count - i) as f32 / 160.0).min(1.0);
        let sample = ((i as f32 * frequency * std::f32::consts::TAU / 16000.0).sin()
            * taper
            * 4000.0) as i16;
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    bytes
}
pub async fn play(finish: bool) -> Result<()> {
    let bytes = wave(finish);
    #[cfg(windows)]
    {
        tokio::task::spawn_blocking(move || {
            use windows_sys::Win32::Media::Audio::*;
            if unsafe {
                PlaySoundW(
                    bytes.as_ptr().cast(),
                    std::ptr::null_mut(),
                    SND_MEMORY | SND_SYNC | SND_NODEFAULT,
                )
            } == 0
            {
                anyhow::bail!("Could not play audio cue");
            }
            Ok(())
        })
        .await?
    }
    #[cfg(not(windows))]
    {
        use anyhow::Context;
        use std::{process::Stdio, time::Duration};
        use tokio::io::AsyncWriteExt;
        let mut child = tokio::process::Command::new("pw-play")
            .args(["--raw", "--rate=16000", "--channels=1", "--format=s16", "-"])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .context("Cannot start pw-play")?;
        tokio::time::timeout(Duration::from_secs(2), async {
            let mut stdin = child.stdin.take().context("Cannot open audio cue input")?;
            stdin.write_all(&bytes[44..]).await?;
            drop(stdin);
            if !child.wait().await?.success() {
                anyhow::bail!("Audio cue failed");
            }
            Ok::<_, anyhow::Error>(())
        })
        .await
        .context("Audio cue timed out")?
    }
}
