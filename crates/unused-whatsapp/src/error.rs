use thiserror::Error;

#[derive(Debug, Error)]
pub enum WhatsappError {
    #[error("cdp: {0}")]
    Cdp(#[from] chromium_driver::CdpError),

    #[error("qr code not found on page")]
    QrCodeNotFound,

    #[error("qr code decode failed: {0}")]
    QrCodeDecode(String),

    #[error("selector not found: {0}")]
    SelectorNotFound(&'static str),

    #[error("timed out waiting for {0}")]
    Timeout(String),

    #[error("screenshot failed: {0}")]
    Screenshot(String),

    #[error("base64 decode: {0}")]
    Base64(#[from] base64::DecodeError),

    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, WhatsappError>;
