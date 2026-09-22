//! Explicit opt-in acceptance tests; credentials stay in the process environment.
//! Run: cargo test --locked --test live_api -- --ignored --nocapture --test-threads=1
use ai_dikte::{config::Config, live, protocol};
use std::time::Duration;
use tokio::{sync::mpsc, time::Instant};

fn api_key() -> String {
    std::env::var("GEMINI_API_KEY")
        .ok()
        .filter(|key| !key.trim().is_empty())
        .expect("Set GEMINI_API_KEY to run opt-in live tests")
}

#[tokio::test]
#[ignore = "Requires live network and GEMINI_API_KEY"]
async fn live_gemini_authentication() {
    live::validate_key(&Config::default(), &api_key())
        .await
        .unwrap();
}

async fn dictate(pcm: &[u8], config: &Config) -> (anyhow::Result<String>, Duration) {
    let key = api_key();
    let (tx, rx) = mpsc::channel(32);
    let send = async {
        for chunk in pcm.chunks(3200) {
            tx.send(Ok(chunk.to_vec())).await.unwrap();
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        let stopped = Instant::now();
        drop(tx);
        stopped
    };
    let (result, stopped) = tokio::join!(live::transcribe(config, &key, rx), send);
    (result, stopped.elapsed())
}

#[tokio::test]
#[ignore = "Requires live network and GEMINI_API_KEY"]
async fn accidental_empty_silent_and_noisy_recordings_finish_promptly() {
    let mut seed = 42_u32;
    let noise: Vec<_> = (0..32_000)
        .flat_map(|_| {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            ((seed >> 16) as i16 / 100).to_le_bytes()
        })
        .collect();
    for (label, pcm) in [
        ("empty", Vec::new()),
        ("100ms silence", vec![0; 3200]),
        ("100ms microphone noise", noise[..3200].to_vec()),
        ("2s silence after setup", vec![0; 64_000]),
        ("2s microphone noise after setup", noise),
    ] {
        let (result, elapsed) = dictate(&pcm, &Config::default()).await;
        let error = result.unwrap_err();
        assert!(protocol::is_no_speech(&error), "{label}: {error:#}");
        assert!(elapsed < Duration::from_secs(1), "{label}: {elapsed:?}");
        println!("{label}: NoSpeech, stop-to-result {elapsed:?}");
    }
}

#[tokio::test]
#[ignore = "Requires live network and GEMINI_API_KEY"]
async fn ordinary_and_quiet_speech_retain_transcribed_words() {
    let config = Config {
        language: "en-US".into(),
        ..Config::default()
    };
    let original: &[u8] = include_bytes!("fixtures/speech.pcm");
    let short: &[u8] = include_bytes!("fixtures/short-speech.pcm");
    for (divisor, samples, expected) in [
        (1, original, "dictation"),
        (16, original, "dictation"),
        (1, short, "hello"),
    ] {
        let pcm: Vec<_> = samples
            .as_chunks::<2>()
            .0
            .iter()
            .flat_map(|sample| (i16::from_le_bytes(*sample) / divisor).to_le_bytes())
            .collect();
        let (result, elapsed) = dictate(&pcm, &config).await;
        let text = result.unwrap();
        assert!(text.to_lowercase().contains(expected), "{text}");
        println!("speech at 1/{divisor} volume: {text:?}, stop-to-result {elapsed:?}");
    }
}
