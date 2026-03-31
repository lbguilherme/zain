use thiserror::Error;

#[derive(Debug, Error)]
pub enum CdpError {
    #[error("websocket: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("http: {0}")]
    Http(#[from] reqwest::Error),

    #[error("json: {0}")]
    Json(#[from] serde_json::Error),

    #[error("cdp protocol error {code}: {message}")]
    Protocol { code: i64, message: String },

    #[error("connection closed")]
    ConnectionClosed,

    #[error("request timed out after {0:?}")]
    Timeout(std::time::Duration),

    #[error("process failed to start: {0}")]
    ProcessStart(std::io::Error),

    #[error("browser closed unexpectedly")]
    BrowserCrashed,

    #[error("session not found: {0}")]
    SessionNotFound(String),

    #[error("js exception: {0}")]
    JsException(String),
}

pub type Result<T> = std::result::Result<T, CdpError>;
