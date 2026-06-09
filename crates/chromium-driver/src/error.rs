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

    /// A queried element/frame/resource did not exist (e.g. a selector matched
    /// nothing). Distinct from a protocol error so callers can match it.
    #[error("not found: {0}")]
    NotFound(String),

    /// Failed to decode data returned by the browser (base64, etc.).
    #[error("decode error: {0}")]
    Decode(String),

    /// The browser returned a response in an unexpected shape (missing field,
    /// wrong type, malformed payload) — not a protocol-level error.
    #[error("unexpected response: {0}")]
    Unexpected(String),

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
