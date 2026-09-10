use crate::{
    activity::AudioActivity,
    config::Config,
    protocol::{self, Transcript},
};
use anyhow::{Context, Result, bail};
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use std::time::Duration;
use tokio::{
    net::TcpStream,
    sync::mpsc,
    time::{Instant, timeout},
};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, tungstenite::Message};
type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;
const ENDPOINT: &str = "wss://generativelanguage.googleapis.com/ws/google.ai.generativelanguage.v1beta.GenerativeService.BidiGenerateContent";
const IO_TIMEOUT: Duration = Duration::from_secs(10);
const SILENT_CLOSE_TIMEOUT: Duration = Duration::from_millis(250);
async fn send(socket: &mut Socket, value: Value) -> Result<()> {
    timeout(
        IO_TIMEOUT,
        socket.send(Message::Text(value.to_string().into())),
    )
    .await
    .context("Gemini send timed out")?
    .map_err(|_| anyhow::anyhow!("Gemini connection failed while sending"))
}
async fn receive(socket: &mut Socket) -> Result<Value> {
    loop {
        match socket.next().await {
            Some(Ok(Message::Text(text))) => {
                return serde_json::from_str(&text)
                    .map_err(|_| anyhow::anyhow!("Invalid Gemini JSON message"));
            }
            Some(Ok(Message::Binary(bytes))) => {
                return serde_json::from_slice(&bytes)
                    .map_err(|_| anyhow::anyhow!("Invalid Gemini JSON message"));
            }
            Some(Ok(Message::Ping(_))) => {
                socket
                    .flush()
                    .await
                    .context("Cannot respond to Gemini ping")?;
            }
            Some(Ok(Message::Pong(_))) => (),
            Some(Ok(Message::Frame(_))) => (),
            _ => bail!("Gemini connection ended before transcription was complete"),
        }
    }
}
async fn connect(config: &Config, key: &str) -> Result<Socket> {
    connect_at(config, key, ENDPOINT).await
}
async fn connect_at(config: &Config, key: &str, endpoint: &str) -> Result<Socket> {
    let mut url = url::Url::parse(endpoint)?;
    url.query_pairs_mut().append_pair("key", key);
    // Never attach the transport error: it can contain the authenticated URL.
    let (mut socket, _) = timeout(IO_TIMEOUT, tokio_tungstenite::connect_async(url.as_str()))
        .await
        .context("Gemini connection timed out")?
        .map_err(|error| match error {
            tokio_tungstenite::tungstenite::Error::Http(response) => anyhow::anyhow!(
                "Gemini rejected the connection (HTTP {}). Check your API key and model access.",
                response.status().as_u16()
            ),
            _ => anyhow::anyhow!("Cannot connect to Gemini; check network and API access"),
        })?;
    send(&mut socket, protocol::setup(config)).await?;
    timeout(IO_TIMEOUT, async {
        loop {
            let message = receive(&mut socket).await?;
            protocol::check_error(&message)?;
            if let Some(acknowledgement) = message.get("setupComplete") {
                if !acknowledgement.is_object() {
                    bail!("Gemini returned an invalid setup acknowledgement");
                }
                return Ok::<_, anyhow::Error>(());
            }
        }
    })
    .await
    .context("Gemini setup timed out")??;
    Ok(socket)
}
pub async fn validate_key(config: &Config, key: &str) -> Result<()> {
    let mut socket = connect(config, key).await?;
    timeout(Duration::from_secs(2), socket.close(None))
        .await
        .context("Gemini close timed out")?
        .map_err(|_| anyhow::anyhow!("Gemini validation connection failed to close"))?;
    Ok(())
}
/// Audio channel is bounded by the recorder. Closure means capture has stopped
/// and all queued samples have drained; errors abort without injecting text.
pub async fn transcribe(
    config: &Config,
    key: &str,
    audio: mpsc::Receiver<Result<Vec<u8>>>,
) -> Result<String> {
    let socket = connect(config, key).await?;
    stream(socket, audio).await
}
async fn close_silent(socket: &mut Socket) {
    // Silence is already a committed local outcome. Do not make the user wait
    // for Gemini's normal finalization timeout just to complete a close handshake.
    let _ = timeout(SILENT_CLOSE_TIMEOUT, socket.close(None)).await;
}
async fn stream(mut socket: Socket, mut audio: mpsc::Receiver<Result<Vec<u8>>>) -> Result<String> {
    send(&mut socket, json!({"realtimeInput":{"activityStart":{}}})).await?;
    let start = Instant::now();
    let mut transcript = Transcript::default();
    let mut activity = AudioActivity::default();
    let mut has_audio = false;
    loop {
        tokio::select! {
            message = receive(&mut socket) => transcript.receive(&message?, start.elapsed())?,
            chunk = audio.recv() => match chunk {
                Some(chunk) => {
                    let chunk = chunk?;
                    activity.observe_pcm16(&chunk)?;
                    send(&mut socket, protocol::audio(&chunk)?).await?;
                    has_audio = true;
                }
                None => break,
            }
        }
    }
    if !has_audio {
        close_silent(&mut socket).await;
        return Err(protocol::NoSpeech.into());
    }
    // The local meter is deliberately conservative. It only bypasses remote
    // finalization when the signal is clearly silent and Gemini has not already
    // observed any transcript text. Ambiguous/quiet audio keeps the old path.
    if activity.clearly_silent() && !transcript.has_text() {
        close_silent(&mut socket).await;
        return Err(protocol::NoSpeech.into());
    }
    send(&mut socket, json!({"realtimeInput":{"activityEnd":{}}})).await?;
    let stopped_at = start.elapsed();
    let deadline = Instant::now() + protocol::FINAL_TIMEOUT;
    loop {
        if transcript.ready(start.elapsed(), stopped_at) {
            break;
        }
        if Instant::now() >= deadline {
            return transcript.finish();
        }
        tokio::select! {
            message = receive(&mut socket) => transcript.receive(&message?, start.elapsed())?,
            _ = tokio::time::sleep(Duration::from_millis(25)) => (),
        }
    }
    let result = transcript.finish();
    // The final text is already committed; a close handshake failure cannot
    // change that result. Always bound the handshake and drop the socket.
    let _ = timeout(Duration::from_secs(2), socket.close(None)).await;
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;
    async fn mock(outcome: &'static str) -> (String, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("ws://{}", listener.local_addr().unwrap());
        let task = tokio::spawn(async move {
            let (tcp, _) = listener.accept().await.unwrap();
            let mut socket = tokio_tungstenite::accept_async(tcp).await.unwrap();
            let setup: Value =
                serde_json::from_str(socket.next().await.unwrap().unwrap().to_text().unwrap())
                    .unwrap();
            assert_eq!(
                setup["setup"]["model"],
                format!("models/{}", protocol::MODEL)
            );
            if outcome == "bad_ack" {
                socket
                    .send(Message::Text(
                        json!({"setupComplete":null}).to_string().into(),
                    ))
                    .await
                    .unwrap();
                let _ = socket.close(None).await;
                return;
            }
            if outcome == "auth" {
                socket
                    .send(Message::Text(
                        json!({"error":{"code":403,"message":"secret-api-key"}})
                            .to_string()
                            .into(),
                    ))
                    .await
                    .unwrap();
                return;
            }
            socket
                .send(Message::Text(
                    json!({"setupComplete":{}}).to_string().into(),
                ))
                .await
                .unwrap();
            for field in ["activityStart", "audio"] {
                let message: Value =
                    serde_json::from_str(socket.next().await.unwrap().unwrap().to_text().unwrap())
                        .unwrap();
                assert!(!message["realtimeInput"][field].is_null());
            }
            if outcome == "silence" {
                // The client intentionally closes instead of waiting up to the
                // normal finalization timeout for an empty transcript.
                let _ = socket.next().await;
                return;
            }
            let message: Value =
                serde_json::from_str(socket.next().await.unwrap().unwrap().to_text().unwrap())
                    .unwrap();
            assert!(!message["realtimeInput"]["activityEnd"].is_null());
            if outcome == "disconnect" {
                socket.close(None).await.unwrap();
                return;
            }
            for text in ["Evet.", "Evet."] {
                socket
                    .send(Message::Text(
                        json!({"serverContent":{"inputTranscription":{"text":text}}})
                            .to_string()
                            .into(),
                    ))
                    .await
                    .unwrap();
            }
            if outcome == "interim" {
                socket.send(Message::Text(json!({"serverContent":{"interimInputTranscription":{"text":"unfinished"}}}).to_string().into())).await.unwrap();
                socket.close(None).await.unwrap();
                return;
            }
            // Deliberately no turnComplete; client must still finalize.
            let _ = socket.next().await;
        });
        (url, task)
    }
    fn voiced_chunk() -> Vec<u8> {
        (0..1600)
            .flat_map(|index| {
                let sample: i16 = if index % 2 == 0 { 2_000 } else { -2_000 };
                sample.to_le_bytes()
            })
            .collect()
    }
    async fn exercise(outcome: &'static str) -> Result<String> {
        let (url, server) = mock(outcome).await;
        let socket = connect_at(&Config::default(), "test-only", &url).await?;
        let (tx, rx) = mpsc::channel(2);
        let chunk = if outcome == "silence" {
            vec![0; 3200]
        } else {
            voiced_chunk()
        };
        tx.send(Ok(chunk)).await.unwrap();
        drop(tx);
        let result = tokio::time::timeout(Duration::from_secs(3), stream(socket, rx))
            .await
            .unwrap();
        server.await.unwrap();
        result
    }
    #[tokio::test]
    async fn wire_session_preserves_repeated_finals_without_turn_complete() {
        assert_eq!(exercise("success").await.unwrap(), "Evet. Evet.");
    }
    #[tokio::test]
    async fn obvious_silence_finishes_without_remote_finalization_wait() {
        let started = Instant::now();
        let error = exercise("silence").await.unwrap_err();
        assert!(protocol::is_no_speech(&error));
        assert!(started.elapsed() < Duration::from_secs(1));
    }
    #[tokio::test]
    async fn disconnect_never_returns_partial_text() {
        assert!(exercise("disconnect").await.is_err());
    }
    #[tokio::test]
    async fn interim_disconnect_never_returns_committed_prefix() {
        assert!(exercise("interim").await.is_err());
    }
    #[tokio::test]
    async fn malformed_setup_acknowledgement_is_not_validation_success() {
        let (url, server) = mock("bad_ack").await;
        assert!(
            connect_at(&Config::default(), "test-only", &url)
                .await
                .is_err()
        );
        server.await.unwrap();
    }
    #[tokio::test]
    async fn authentication_error_does_not_expose_key() {
        let (url, server) = mock("auth").await;
        let error = connect_at(&Config::default(), "secret-api-key", &url)
            .await
            .err()
            .unwrap();
        assert!(!format!("{error:#}").contains("secret-api-key"));
        server.await.unwrap();
    }
}
