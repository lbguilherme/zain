use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::sync::{broadcast, mpsc, oneshot, Mutex};
use tokio_tungstenite::tungstenite::Message;

use crate::error::{CdpError, Result};

#[derive(Debug, Clone)]
pub struct CdpEvent {
    pub method: String,
    pub session_id: Option<String>,
    pub params: Value,
}

struct SendCommand {
    id: u64,
    message: String,
    response_tx: oneshot::Sender<Result<Value>>,
}

pub(crate) struct Transport {
    cmd_tx: mpsc::Sender<SendCommand>,
    event_tx: broadcast::Sender<CdpEvent>,
    next_id: AtomicU64,
}

impl Transport {
    pub async fn connect(url: &str) -> Result<Arc<Self>> {
        let (ws, _) = tokio_tungstenite::connect_async(url).await?;
        let (ws_sink, ws_stream) = ws.split();

        let (cmd_tx, cmd_rx) = mpsc::channel::<SendCommand>(64);
        let (event_tx, _) = broadcast::channel::<CdpEvent>(256);

        let pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Value>>>>> =
            Arc::new(Mutex::new(HashMap::new()));

        // Writer task
        let pending_for_writer = pending.clone();
        tokio::spawn(Self::writer_loop(cmd_rx, ws_sink, pending_for_writer));

        // Reader task
        let event_tx_clone = event_tx.clone();
        tokio::spawn(Self::reader_loop(ws_stream, pending, event_tx_clone));

        Ok(Arc::new(Self {
            cmd_tx,
            event_tx,
            next_id: AtomicU64::new(1),
        }))
    }

    pub async fn send(
        &self,
        method: &str,
        params: Value,
        session_id: Option<&str>,
    ) -> Result<Value> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let mut msg = serde_json::json!({
            "id": id,
            "method": method,
            "params": params,
        });

        if let Some(sid) = session_id {
            msg["sessionId"] = Value::String(sid.to_owned());
        }

        let message = serde_json::to_string(&msg)?;
        let (response_tx, response_rx) = oneshot::channel();

        self.cmd_tx
            .send(SendCommand {
                id,
                message,
                response_tx,
            })
            .await
            .map_err(|_| CdpError::ConnectionClosed)?;

        let timeout = tokio::time::Duration::from_secs(30);
        match tokio::time::timeout(timeout, response_rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(CdpError::ConnectionClosed),
            Err(_) => Err(CdpError::Timeout(timeout)),
        }
    }

    pub fn events(&self) -> broadcast::Receiver<CdpEvent> {
        self.event_tx.subscribe()
    }

    async fn writer_loop(
        mut cmd_rx: mpsc::Receiver<SendCommand>,
        mut sink: impl SinkExt<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
        pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Value>>>>>,
    ) {
        while let Some(cmd) = cmd_rx.recv().await {
            {
                let mut map = pending.lock().await;
                map.insert(cmd.id, cmd.response_tx);
            }

            if let Err(e) = sink.send(Message::Text(cmd.message.into())).await {
                let mut map = pending.lock().await;
                if let Some(tx) = map.remove(&cmd.id) {
                    let _ = tx.send(Err(CdpError::WebSocket(e)));
                }
                break;
            }
        }
    }

    async fn reader_loop(
        mut stream: impl StreamExt<Item = std::result::Result<Message, tokio_tungstenite::tungstenite::Error>>
            + Unpin,
        pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Value>>>>>,
        event_tx: broadcast::Sender<CdpEvent>,
    ) {
        while let Some(msg) = stream.next().await {
            let text = match msg {
                Ok(Message::Text(t)) => t,
                Ok(Message::Close(_)) => break,
                Ok(_) => continue,
                Err(_) => break,
            };

            let value: Value = match serde_json::from_str(&text) {
                Ok(v) => v,
                Err(_) => continue,
            };

            // Response to a command (has "id")
            if let Some(id) = value.get("id").and_then(|v| v.as_u64()) {
                let mut map = pending.lock().await;
                if let Some(tx) = map.remove(&id) {
                    let result = if let Some(error) = value.get("error") {
                        let code = error.get("code").and_then(|c| c.as_i64()).unwrap_or(-1);
                        let message = error
                            .get("message")
                            .and_then(|m| m.as_str())
                            .unwrap_or("unknown error")
                            .to_owned();
                        Err(CdpError::Protocol { code, message })
                    } else {
                        Ok(value.get("result").cloned().unwrap_or(Value::Object(Default::default())))
                    };
                    let _ = tx.send(result);
                }
                continue;
            }

            // Event (has "method" but no "id")
            if let Some(method) = value.get("method").and_then(|m| m.as_str()) {
                let session_id = value
                    .get("sessionId")
                    .and_then(|s| s.as_str())
                    .map(String::from);
                let params = value.get("params").cloned().unwrap_or(Value::Object(Default::default()));

                let _ = event_tx.send(CdpEvent {
                    method: method.to_owned(),
                    session_id,
                    params,
                });
            }
        }

        // Connection closed: notify all pending requests
        let mut map = pending.lock().await;
        for (_, tx) in map.drain() {
            let _ = tx.send(Err(CdpError::ConnectionClosed));
        }
    }
}
