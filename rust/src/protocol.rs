//! Pure Gemini wire contract and transcript reducer; independent of devices and UI.
use crate::config::Config;
use anyhow::{Result, bail};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use std::time::Duration;
pub const MODEL: &str = "gemini-3.5-transcribe-live";
pub const RATE: u32 = 16000;
pub const SETTLE: Duration = Duration::from_millis(500);
pub const FINAL_TIMEOUT: Duration = Duration::from_secs(10);
pub fn setup(config: &Config) -> Value {
    let mut transcription = json!({"languageCodes": [config.language], "mode": config.mode});
    if !config.custom_vocabulary.is_empty() {
        transcription["customVocabulary"] = json!(config.custom_vocabulary);
    }
    json!({"setup": {"model": format!("models/{MODEL}"), "generationConfig": {"responseModalities": ["TEXT"]}, "realtimeInputConfig": {"automaticActivityDetection": {"disabled": true}}, "inputAudioTranscription": transcription}})
}
pub fn audio(bytes: &[u8]) -> Result<Value> {
    if bytes.is_empty() || !bytes.len().is_multiple_of(2) {
        bail!("Audio must contain complete signed 16-bit PCM samples");
    }
    Ok(
        json!({"realtimeInput": {"audio": {"data": STANDARD.encode(bytes), "mimeType": "audio/pcm;rate=16000"}}}),
    )
}
pub fn check_error(message: &Value) -> Result<()> {
    if let Some(error) = message.get("error") {
        // Server error text is deliberately omitted: it can echo request credentials.
        bail!(
            "Gemini rejected the request (code {}). Check API key, quota and model access.",
            error.get("code").and_then(Value::as_i64).unwrap_or(0)
        );
    }
    Ok(())
}
#[derive(Default)]
pub struct Transcript {
    segments: Vec<String>,
    pending_interim: bool,
    complete: bool,
    last_update: Duration,
}
impl Transcript {
    pub fn receive(&mut self, message: &Value, now: Duration) -> Result<()> {
        check_error(message)?;
        let content = &message["serverContent"];
        let final_text = content["inputTranscription"]["text"]
            .as_str()
            .unwrap_or("")
            .trim();
        let interim = content["interimInputTranscription"]["text"]
            .as_str()
            .unwrap_or("")
            .trim();
        if !final_text.is_empty() {
            // Repeated finals are distinct spoken segments, never deduplicate.
            self.segments.push(final_text.to_owned());
            self.pending_interim = false;
            self.last_update = now;
        }
        if !interim.is_empty() {
            self.pending_interim = true;
            self.last_update = now;
        }
        if content["turnComplete"].as_bool() == Some(true) {
            self.complete = true;
            self.last_update = now;
        }
        Ok(())
    }
    pub fn ready(&self, now: Duration, stopped_at: Duration) -> bool {
        !self.pending_interim
            && (self.complete || !self.segments.is_empty())
            && now.saturating_sub(self.last_update.max(stopped_at)) >= SETTLE
    }
    pub fn finish(&self) -> Result<String> {
        if self.pending_interim {
            bail!("Transcription is incomplete; text was not typed");
        }
        if self.segments.is_empty() {
            bail!("Gemini returned no transcription");
        }
        Ok(self.segments.join(" "))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn final_message(text: &str) -> Value {
        json!({"serverContent":{"inputTranscription":{"text":text}}})
    }
    #[test]
    fn repeated_finals_are_not_lost() {
        let mut t = Transcript::default();
        t.receive(&final_message("Evet."), Duration::ZERO).unwrap();
        t.receive(&final_message("Evet."), Duration::ZERO).unwrap();
        assert_eq!(t.finish().unwrap(), "Evet. Evet.");
    }
    #[test]
    fn final_without_turn_complete_settles_after_stop() {
        let mut t = Transcript::default();
        t.receive(&final_message("Merhaba"), Duration::ZERO)
            .unwrap();
        assert!(!t.ready(Duration::from_secs(2), Duration::from_secs(2)));
        assert!(t.ready(Duration::from_millis(2500), Duration::from_secs(2)));
    }
    #[test]
    fn pending_interim_blocks_partial_output_even_after_completion() {
        let mut t = Transcript::default();
        t.receive(&final_message("Bir"), Duration::ZERO).unwrap();
        t.receive(&json!({"serverContent":{"interimInputTranscription":{"text":"iki"},"turnComplete":true}}), Duration::ZERO).unwrap();
        assert!(!t.ready(Duration::from_secs(20), Duration::ZERO));
        assert!(t.finish().is_err());
        t.receive(&final_message("iki"), Duration::from_secs(20))
            .unwrap();
        assert_eq!(t.finish().unwrap(), "Bir iki");
    }
    #[test]
    fn final_and_interim_in_same_message_keep_the_tail_pending() {
        let mut transcript = Transcript::default();
        transcript
            .receive(
                &json!({"serverContent": {
                    "inputTranscription": {"text": "Bir"},
                    "interimInputTranscription": {"text": "iki"},
                    "turnComplete": true
                }}),
                Duration::ZERO,
            )
            .unwrap();
        assert!(!transcript.ready(Duration::from_secs(20), Duration::ZERO));
        assert!(transcript.finish().is_err());
        transcript
            .receive(&final_message("iki"), Duration::from_secs(20))
            .unwrap();
        assert_eq!(transcript.finish().unwrap(), "Bir iki");
    }
    #[test]
    fn delayed_final_restarts_settle_window() {
        let mut t = Transcript::default();
        t.receive(
            &json!({"serverContent":{"turnComplete":true}}),
            Duration::ZERO,
        )
        .unwrap();
        t.receive(&final_message("geç"), Duration::from_millis(400))
            .unwrap();
        assert!(!t.ready(Duration::from_millis(500), Duration::ZERO));
        assert!(t.ready(Duration::from_millis(900), Duration::ZERO));
    }
    #[test]
    fn errors_do_not_echo_secrets() {
        let error = check_error(&json!({"error":{"code":403,"message":"key=secret"}})).unwrap_err();
        assert!(!error.to_string().contains("secret"));
    }
    #[test]
    fn pcm_contract() {
        let message = audio(&[0, 128, 255, 127]).unwrap();
        assert_eq!(message["realtimeInput"]["audio"]["data"], "AID/fw==");
        assert!(audio(&[0]).is_err());
        assert!(audio(&[]).is_err());
        let setup = setup(&Config::default());
        assert_eq!(setup["setup"]["inputAudioTranscription"]["mode"], "SMART");
        assert_eq!(
            setup["setup"]["realtimeInputConfig"]["automaticActivityDetection"]["disabled"],
            true
        );
    }
}
