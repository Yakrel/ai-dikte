//! Conservative local signal meter used only to short-circuit obvious silence.
//!
//! This is intentionally not a general-purpose VAD. Ambiguous or noisy audio is
//! always sent through the normal Gemini finalization path so quiet speech is not
//! discarded by a local heuristic.
use anyhow::{Result, bail};

const RMS_SILENCE_THRESHOLD: f64 = 0.0032; // roughly -50 dBFS
const PEAK_SILENCE_THRESHOLD: f64 = 0.010; // -40 dBFS

#[derive(Debug, Default)]
pub struct AudioActivity {
    samples: u64,
    sum_squares: f64,
    peak: f64,
}

impl AudioActivity {
    pub fn observe_pcm16(&mut self, bytes: &[u8]) -> Result<()> {
        if !bytes.len().is_multiple_of(2) {
            bail!("Audio must contain complete signed 16-bit PCM samples");
        }

        for sample in bytes.as_chunks::<2>().0 {
            let value = i16::from_le_bytes(*sample) as f64 / 32768.0;
            self.samples += 1;
            self.sum_squares += value * value;
            self.peak = self.peak.max(value.abs());
        }
        Ok(())
    }

    /// Returns true only for a confidently silent signal. Anything near the
    /// threshold is deliberately treated as possible speech and finalized by
    /// Gemini instead.
    pub fn clearly_silent(&self) -> bool {
        if self.samples == 0 {
            return false;
        }
        let rms = (self.sum_squares / self.samples as f64).sqrt();
        rms <= RMS_SILENCE_THRESHOLD && self.peak <= PEAK_SILENCE_THRESHOLD
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pcm(samples: impl IntoIterator<Item = i16>) -> Vec<u8> {
        samples
            .into_iter()
            .flat_map(i16::to_le_bytes)
            .collect::<Vec<_>>()
    }

    #[test]
    fn one_second_of_digital_silence_is_detected_locally() {
        let mut activity = AudioActivity::default();
        activity.observe_pcm16(&pcm(vec![0; 16_000])).unwrap();
        assert!(activity.clearly_silent());
    }

    #[test]
    fn low_level_microphone_floor_is_detected_as_silence() {
        let mut activity = AudioActivity::default();
        let samples = (0..16_000).map(|index| if index % 2 == 0 { 64 } else { -64 });
        activity.observe_pcm16(&pcm(samples)).unwrap();
        assert!(activity.clearly_silent());
    }

    #[test]
    fn quiet_speech_like_peak_falls_back_to_remote_finalization() {
        let mut activity = AudioActivity::default();
        let mut samples = vec![0; 16_000];
        samples[8_000] = 400;
        activity.observe_pcm16(&pcm(samples)).unwrap();
        assert!(!activity.clearly_silent());
    }

    #[test]
    fn speech_like_burst_is_never_short_circuited() {
        let mut activity = AudioActivity::default();
        let samples = (0..16_000).map(|index| {
            if (4_000..5_600).contains(&index) {
                if index % 2 == 0 { 2_000 } else { -2_000 }
            } else {
                0
            }
        });
        activity.observe_pcm16(&pcm(samples)).unwrap();
        assert!(!activity.clearly_silent());
    }

    #[test]
    fn transient_peak_falls_back_to_remote_finalization() {
        let mut activity = AudioActivity::default();
        let mut samples = vec![0; 16_000];
        samples[8_000] = 1_000;
        activity.observe_pcm16(&pcm(samples)).unwrap();
        assert!(!activity.clearly_silent());
    }

    #[test]
    fn malformed_pcm_is_rejected() {
        let mut activity = AudioActivity::default();
        assert!(activity.observe_pcm16(&[1]).is_err());
        assert!(!activity.clearly_silent());
    }
}
