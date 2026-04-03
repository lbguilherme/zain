pub(crate) mod chat;
pub(crate) mod client;
pub(crate) mod error;
pub(crate) mod message;
pub(crate) mod profile;
pub(crate) mod qr;
pub(crate) mod types;

pub use client::{WhatsAppClient, WhatsAppOptions, WhatsAppSession};
pub use error::{Result, WhatsappError};
pub use types::{ChatPreview, MessageType, RawMessage, UserProfile};

/// WhatsApp Web URL.
pub(crate) const WEB_URL: &str = "https://web.whatsapp.com";
