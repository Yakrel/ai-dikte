//! Optional on-demand live integration test with the real Gemini API.
//! Run with: GEMINI_API_KEY="your-key" cargo test --test live_api -- --ignored --nocapture
use ai_dikte::{config::Config, live};

#[tokio::test]
#[ignore = "Requires live network and GEMINI_API_KEY environment variable"]
async fn test_live_gemini_connection_with_env_key() {
    let Ok(key) = std::env::var("GEMINI_API_KEY") else {
        eprintln!("Skipping live test: GEMINI_API_KEY is not set.");
        return;
    };
    if key.trim().is_empty() {
        eprintln!("Skipping live test: GEMINI_API_KEY is empty.");
        return;
    }

    let config = Config::default();
    println!("Testing live WebSocket connection with Gemini API...");
    let result = live::validate_key(&config, &key).await;
    assert!(
        result.is_ok(),
        "Live Gemini validation failed: {:?}",
        result.err()
    );
    println!("Live Gemini validation succeeded!");
}
