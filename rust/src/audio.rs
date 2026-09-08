use crate::config::Config;
use anyhow::Result;
use tokio::sync::{mpsc, oneshot};
pub const BUFFER_CHUNKS: usize = 2048; // Bounded preconnection buffer; never silently discard audio.
pub struct Capture {
    pub audio: mpsc::Receiver<Result<Vec<u8>>>,
    pub stop: oneshot::Sender<()>,
    pub task: tokio::task::JoinHandle<Result<()>>,
}
#[cfg(not(windows))]
pub fn start(config: &Config) -> Result<Capture> {
    use anyhow::Context;
    use std::{process::Stdio, time::Duration};
    use tokio::{io::AsyncReadExt, process::Command};
    if config.input_device.is_some() {
        anyhow::bail!("Linux uses the PipeWire default source; choose it in system audio settings");
    }
    let mut child = Command::new("pw-record")
        .args(["--raw", "--rate=16000", "--channels=1", "--format=s16", "-"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .context("Cannot start pw-record; install PipeWire")?;
    let mut stdout = child
        .stdout
        .take()
        .context("Recorder stdout is unavailable")?;
    let (tx, audio) = mpsc::channel(BUFFER_CHUNKS);
    let (stop, mut stopping) = oneshot::channel();
    let task = tokio::spawn(async move {
        let mut buffer = [0u8; 3200];
        let mut pending_byte = None;
        loop {
            let size = tokio::select! {
                _ = &mut stopping => break,
                result = stdout.read(&mut buffer) => result.context("Cannot read PipeWire audio")?,
            };
            if size == 0 {
                anyhow::bail!("PipeWire recorder exited unexpectedly");
            }
            let mut bytes = Vec::with_capacity(size + 1);
            if let Some(byte) = pending_byte.take() {
                bytes.push(byte);
            }
            bytes.extend_from_slice(&buffer[..size]);
            if !bytes.len().is_multiple_of(2) {
                pending_byte = bytes.pop();
            }
            if !bytes.is_empty() && tx.try_send(Ok(bytes)).is_err() {
                anyhow::bail!("Audio buffer overflow or consumer stopped; recording aborted");
            }
        }
        // SIGINT asks PipeWire to flush its final samples. Drain concurrently,
        // otherwise a full stdout pipe can deadlock process shutdown.
        if let Some(pid) = child.id()
            && unsafe { libc::kill(pid as i32, libc::SIGINT) } != 0
        {
            anyhow::bail!("Cannot stop PipeWire recorder");
        }
        let drain = async {
            loop {
                let size = stdout.read(&mut buffer).await?;
                if size == 0 {
                    break;
                }
                let mut bytes = Vec::new();
                if let Some(byte) = pending_byte.take() {
                    bytes.push(byte);
                }
                bytes.extend_from_slice(&buffer[..size]);
                if !bytes.len().is_multiple_of(2) {
                    pending_byte = bytes.pop();
                }
                if !bytes.is_empty() {
                    tx.send(Ok(bytes)).await.context("Audio consumer stopped")?;
                }
            }
            child.wait().await?;
            if pending_byte.is_some() {
                anyhow::bail!("Recorder ended with an incomplete PCM sample");
            }
            Ok::<_, anyhow::Error>(())
        };
        tokio::time::timeout(Duration::from_secs(2), drain)
            .await
            .context("PipeWire shutdown timed out")??;
        Ok(())
    });
    Ok(Capture { audio, stop, task })
}
#[cfg(windows)]
pub fn start(config: &Config) -> Result<Capture> {
    use anyhow::Context;
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    let (tx, audio) = mpsc::channel(BUFFER_CHUNKS);
    let (stop, mut stopping) = oneshot::channel();
    let index = config.input_device;
    // WASAPI stream is created, owned and destroyed on the same dedicated thread.
    let task = tokio::task::spawn_blocking(move || {
        let host = cpal::default_host();
        let device = match index {
            Some(index) => host
                .input_devices()?
                .nth(index as usize)
                .context("Selected microphone no longer exists")?,
            None => host
                .default_input_device()
                .context("No default microphone")?,
        };
        let supported = device.default_input_config()?;
        let config: cpal::StreamConfig = supported.clone().into();
        let rate = config.sample_rate.0;
        let channels = config.channels as usize;
        let failed = Arc::new(AtomicBool::new(false));
        let overflow = failed.clone();
        let device_failed = failed.clone();
        let mut pcm = Pcm16::new(rate, channels)?;
        let mut emit = move |samples: &[f32]| {
            let bytes = pcm.convert(samples);
            if !bytes.is_empty() && tx.try_send(Ok(bytes)).is_err() {
                overflow.store(true, Ordering::Release);
            }
        };
        let on_error = move |_| {
            device_failed.store(true, Ordering::Release);
        };
        let stream = match supported.sample_format() {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &config,
                move |data: &[f32], _| emit(data),
                on_error,
                None,
            )?,
            cpal::SampleFormat::I16 => device.build_input_stream(
                &config,
                move |data: &[i16], _| {
                    emit(&data.iter().map(|v| *v as f32 / 32768.0).collect::<Vec<_>>())
                },
                on_error,
                None,
            )?,
            cpal::SampleFormat::U16 => device.build_input_stream(
                &config,
                move |data: &[u16], _| {
                    emit(
                        &data
                            .iter()
                            .map(|v| (*v as f32 - 32768.0) / 32768.0)
                            .collect::<Vec<_>>(),
                    )
                },
                on_error,
                None,
            )?,
            format => anyhow::bail!("Unsupported microphone sample format {format:?}"),
        };
        stream.play()?;
        loop {
            if failed.load(Ordering::Acquire) {
                anyhow::bail!("Microphone failed or audio buffer overflowed; recording aborted");
            }
            match stopping.try_recv() {
                Ok(()) | Err(oneshot::error::TryRecvError::Closed) => break,
                Err(oneshot::error::TryRecvError::Empty) => (),
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        drop(stream);
        Ok(())
    });
    Ok(Capture { audio, stop, task })
}
/// Streaming mono conversion with a box low-pass when downsampling.
/// Phase survives callbacks, so 44.1/48 kHz sources cannot drift or be mislabeled.
#[cfg(any(windows, test))]
struct Pcm16 {
    rate: u32,
    channels: usize,
    phase: u64,
    sum: f64,
    count: u32,
}
#[cfg(any(windows, test))]
impl Pcm16 {
    fn new(rate: u32, channels: usize) -> Result<Self> {
        if rate < 16000 || channels == 0 {
            anyhow::bail!("Microphone must support at least 16 kHz and one channel");
        }
        Ok(Self {
            rate,
            channels,
            phase: 0,
            sum: 0.0,
            count: 0,
        })
    }
    fn convert(&mut self, samples: &[f32]) -> Vec<u8> {
        let mut bytes = Vec::new();
        for frame in samples.chunks_exact(self.channels) {
            self.sum += frame.iter().map(|v| *v as f64).sum::<f64>() / self.channels as f64;
            self.count += 1;
            self.phase += 16000;
            if self.phase >= self.rate as u64 {
                let value = (self.sum / self.count as f64).clamp(-1.0, 1.0);
                bytes.extend_from_slice(&((value * 32767.0).round() as i16).to_le_bytes());
                self.phase -= self.rate as u64;
                self.sum = 0.0;
                self.count = 0;
            }
        }
        bytes
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resampling_keeps_exact_duration_across_callbacks() {
        for rate in [16000, 44100, 48000] {
            let mut pcm = Pcm16::new(rate, 2).unwrap();
            let input = vec![0.5; rate as usize * 2];
            let mut output = Vec::new();
            for chunk in input.chunks(254) {
                output.extend(pcm.convert(chunk));
            }
            assert_eq!(output.len(), 32000);
            assert!(
                output
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .all(|b| i16::from_le_bytes([b[0], b[1]]) == 16384)
            );
        }
    }
}
