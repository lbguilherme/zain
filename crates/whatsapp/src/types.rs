use serde::{Deserialize, Serialize};

// ── Chat types ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ChatsList {
    pub chats: Vec<Chat>,
    pub count: i32,
    pub total: i32,
    #[serde(default)]
    pub offset: i32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Chat {
    pub id: String,
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub chat_type: Option<String>,
    pub timestamp: Option<i64>,
    pub chat_pic: Option<String>,
    pub pin: Option<bool>,
    pub mute: Option<bool>,
    pub archive: Option<bool>,
    pub unread: Option<i32>,
    pub read_only: Option<bool>,
    pub last_message: Option<Message>,
}

// ── Message types ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct MessagesList {
    pub messages: Vec<Message>,
    pub count: i32,
    pub total: i32,
    #[serde(default)]
    pub offset: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Message {
    pub id: String,
    #[serde(rename = "type")]
    pub msg_type: String,
    pub subtype: Option<String>,
    pub chat_id: String,
    pub from: Option<String>,
    pub from_me: bool,
    pub from_name: Option<String>,
    pub timestamp: Option<f64>,
    pub source: Option<String>,
    pub status: Option<String>,
    pub text: Option<TextBody>,
    pub image: Option<MediaBody>,
    pub video: Option<MediaBody>,
    pub audio: Option<MediaBody>,
    pub voice: Option<MediaBody>,
    pub document: Option<DocumentBody>,
    pub sticker: Option<MediaBody>,
    pub location: Option<serde_json::Value>,
    pub contact: Option<serde_json::Value>,
    pub poll: Option<serde_json::Value>,
    pub context: Option<MessageContext>,
    pub reactions: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TextBody {
    pub body: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MediaBody {
    pub id: Option<String>,
    pub link: Option<String>,
    pub mime_type: Option<String>,
    pub file_size: Option<i64>,
    pub caption: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DocumentBody {
    pub id: Option<String>,
    pub link: Option<String>,
    pub mime_type: Option<String>,
    pub file_size: Option<i64>,
    pub filename: Option<String>,
    pub caption: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MessageContext {
    pub forwarded: Option<bool>,
    pub quoted_id: Option<String>,
}

impl Message {
    /// Extrai o corpo de texto principal da mensagem, independente do tipo.
    pub fn text_body(&self) -> Option<&str> {
        if let Some(t) = &self.text {
            return t.body.as_deref();
        }
        if let Some(m) = &self.image {
            return m.caption.as_deref();
        }
        if let Some(m) = &self.video {
            return m.caption.as_deref();
        }
        if let Some(d) = &self.document {
            return d.caption.as_deref();
        }
        None
    }

    /// Verifica se a mensagem contém algum tipo de mídia.
    pub fn has_media(&self) -> bool {
        self.image.is_some()
            || self.video.is_some()
            || self.audio.is_some()
            || self.voice.is_some()
            || self.document.is_some()
            || self.sticker.is_some()
    }

    /// Retorna o mime_type da mídia, se houver.
    pub fn media_mime(&self) -> Option<&str> {
        self.image
            .as_ref()
            .and_then(|m| m.mime_type.as_deref())
            .or_else(|| self.video.as_ref().and_then(|m| m.mime_type.as_deref()))
            .or_else(|| self.audio.as_ref().and_then(|m| m.mime_type.as_deref()))
            .or_else(|| self.voice.as_ref().and_then(|m| m.mime_type.as_deref()))
            .or_else(|| self.document.as_ref().and_then(|d| d.mime_type.as_deref()))
            .or_else(|| self.sticker.as_ref().and_then(|m| m.mime_type.as_deref()))
    }

    /// Retorna o link de download da mídia, se houver.
    pub fn media_url(&self) -> Option<&str> {
        self.image
            .as_ref()
            .and_then(|m| m.link.as_deref())
            .or_else(|| self.video.as_ref().and_then(|m| m.link.as_deref()))
            .or_else(|| self.audio.as_ref().and_then(|m| m.link.as_deref()))
            .or_else(|| self.voice.as_ref().and_then(|m| m.link.as_deref()))
            .or_else(|| self.document.as_ref().and_then(|d| d.link.as_deref()))
            .or_else(|| self.sticker.as_ref().and_then(|m| m.link.as_deref()))
    }
}
