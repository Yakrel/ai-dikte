//! Local speech detection at the wire's 16 kHz mono PCM rate.
//! Use the detector's standard speech threshold; never impose a minimum utterance length.
use anyhow::{Result, bail};

const FRAME_SAMPLES: usize = 256;
const SPEECH_THRESHOLD: f32 = 0.5;

pub struct AudioActivity {
    detector: earshot::Detector<earshot::DefaultPredictor>,
    frame: [i16; FRAME_SAMPLES],
    filled: usize,
    speech: bool,
}

impl Default for AudioActivity {
    fn default() -> Self {
        Self {
            detector: earshot::Detector::default(),
            frame: [0; FRAME_SAMPLES],
            filled: 0,
            speech: false,
        }
    }
}

impl AudioActivity {
    pub fn observe_pcm16(&mut self, bytes: &[u8]) -> Result<()> {
        if !bytes.len().is_multiple_of(2) {
            bail!("Audio must contain complete signed 16-bit PCM samples");
        }
        if self.speech {
            return Ok(());
        }
        for sample in bytes.as_chunks::<2>().0 {
            self.frame[self.filled] = i16::from_le_bytes(*sample);
            self.filled += 1;
            if self.filled == FRAME_SAMPLES {
                self.filled = 0;
                if self.detector.predict_i16(&self.frame) >= SPEECH_THRESHOLD {
                    self.speech = true;
                    break;
                }
            }
        }
        Ok(())
    }

    /// Finalize only after capture ends; zero-pad the last incomplete VAD frame.
    pub fn clearly_silent(&mut self) -> bool {
        if !self.speech && self.filled != 0 {
            self.frame[self.filled..].fill(0);
            self.speech = self.detector.predict_i16(&self.frame) >= SPEECH_THRESHOLD;
            self.filled = 0;
        }
        !self.speech
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_silence_and_short_microphone_noise_are_not_speech() {
        for pcm in [Vec::new(), vec![0; 3200], noise()] {
            let mut activity = AudioActivity::default();
            activity.observe_pcm16(&pcm).unwrap();
            assert!(activity.clearly_silent());
        }
    }

    fn noise() -> Vec<u8> {
        let mut state = 42_u32;
        (0..1600)
            .flat_map(|_| {
                state = state.wrapping_mul(1664525).wrapping_add(1013904223);
                ((state >> 16) as i16 / 100).to_le_bytes()
            })
            .collect()
    }

    #[test]
    fn real_speech_survives_quiet_levels_and_chunk_boundaries() {
        let original: &[u8] = include_bytes!("../tests/fixtures/speech.pcm");
        let short: &[u8] = include_bytes!("../tests/fixtures/short-speech.pcm");
        for (divisor, samples) in [(1, original), (16, original), (16, short)] {
            let pcm: Vec<_> = samples
                .as_chunks::<2>()
                .0
                .iter()
                .flat_map(|sample| (i16::from_le_bytes(*sample) / divisor).to_le_bytes())
                .collect();
            let mut activity = AudioActivity::default();
            for chunk in pcm.chunks(314) {
                activity.observe_pcm16(chunk).unwrap();
            }
            assert!(
                !activity.clearly_silent(),
                "speech at 1/{divisor} volume lost"
            );
        }
    }

    #[test]
    fn malformed_pcm_is_rejected_even_after_speech() {
        let mut activity = AudioActivity::default();
        activity
            .observe_pcm16(include_bytes!("../tests/fixtures/speech.pcm"))
            .unwrap();
        assert!(activity.observe_pcm16(&[1]).is_err());
    }
}
