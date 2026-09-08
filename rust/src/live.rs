use crate::{
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
    let mut url = url::Url::parse(ENDPOINT)?;
    url.query_pairs_mut().append_pair("key", key);
    // Never attach the transport error: it can contain the authenticated URL.
    let (mut socket, _) = timeout(IO_TIMEOUT, tokio_tungstenite::connect_async(url.as_str()))
        .await
        .context("Gemini connection timed out")?
        .map_err(|_| anyhow::anyhow!("Cannot connect to Gemini; check network and API access"))?;
    send(&mut socket, protocol::setup(config)).await?;
    timeout(IO_TIMEOUT, async {
        loop {
            let message = receive(&mut socket).await?;
            protocol::check_error(&message)?;
            if message.get("setupComplete").is_some() {
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
    mut audio: mpsc::Receiver<Result<Vec<u8>>>,
) -> Result<String> {
    let mut socket = connect(config, key).await?;
    send(&mut socket, json!({"realtimeInput":{"activityStart":{}}})).await?;
    let start = Instant::now();
    let mut transcript = Transcript::default();
    let mut has_audio = false;
    loop {
        tokio::select! {
            message = receive(&mut socket) => transcript.receive(&message?, start.elapsed())?,
            chunk = audio.recv() => match chunk {
                Some(chunk) => { send(&mut socket, protocol::audio(&chunk?)?).await?; has_audio = true; }
                None => break,
            }
        }
    }
    if !has_audio {
        bail!("No audio captured; check microphone");
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
