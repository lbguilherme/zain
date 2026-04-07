use serde::{Deserialize, Serialize};

// ── Conteudo de mensagem: Imagem ───────────────────────────────────────
// MediaFile + MessagePropsImageOrVideo + ActionButtons + ViewOnce

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImageContent {
    pub id: String,
    #[serde(default)]
    pub link: Option<String>,
    pub mime_type: String,
    pub file_size: i64,
    #[serde(default)]
    pub file_name: Option<String>,
    #[serde(default)]
    pub sha256: Option<String>,
    #[serde(default)]
    pub timestamp: Option<f64>,
    #[serde(default)]
    pub caption: Option<String>,
    #[serde(default)]
    pub preview: Option<String>,
    #[serde(default)]
    pub width: Option<i32>,
    #[serde(default)]
    pub height: Option<i32>,
    #[serde(default)]
    pub view_once: Option<bool>,
    #[serde(default)]
    pub buttons: Option<Vec<serde_json::Value>>,
}

// ── Conteudo de mensagem: Video / Short / GIF ──────────────────────────
// MediaFile + MessagePropsImageOrVideo + {seconds, autoplay} + ActionButtons + ViewOnce

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VideoContent {
    pub id: String,
    #[serde(default)]
    pub link: Option<String>,
    pub mime_type: String,
    pub file_size: i64,
    #[serde(default)]
    pub file_name: Option<String>,
    #[serde(default)]
    pub sha256: Option<String>,
    #[serde(default)]
    pub timestamp: Option<f64>,
    #[serde(default)]
    pub caption: Option<String>,
    #[serde(default)]
    pub preview: Option<String>,
    #[serde(default)]
    pub width: Option<i32>,
    #[serde(default)]
    pub height: Option<i32>,
    #[serde(default)]
    pub seconds: Option<i32>,
    #[serde(default)]
    pub autoplay: Option<bool>,
    #[serde(default)]
    pub view_once: Option<bool>,
    #[serde(default)]
    pub buttons: Option<Vec<serde_json::Value>>,
}

// ── Conteudo de mensagem: Audio ────────────────────────────────────────
// MediaFile + MessagePropsAudio{seconds} + ViewOnce

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioContent {
    pub id: String,
    #[serde(default)]
    pub link: Option<String>,
    pub mime_type: String,
    pub file_size: i64,
    #[serde(default)]
    pub file_name: Option<String>,
    #[serde(default)]
    pub sha256: Option<String>,
    #[serde(default)]
    pub timestamp: Option<f64>,
    #[serde(default)]
    pub seconds: Option<i32>,
    #[serde(default)]
    pub view_once: Option<bool>,
}

// ── Conteudo de mensagem: Voice ────────────────────────────────────────
// MediaFile + MessagePropsVoice{seconds, recording_time, waveform} + ViewOnce

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VoiceContent {
    pub id: String,
    #[serde(default)]
    pub link: Option<String>,
    pub mime_type: String,
    pub file_size: i64,
    #[serde(default)]
    pub file_name: Option<String>,
    #[serde(default)]
    pub sha256: Option<String>,
    #[serde(default)]
    pub timestamp: Option<f64>,
    #[serde(default)]
    pub seconds: Option<i32>,
    #[serde(default)]
    pub recording_time: Option<f64>,
    #[serde(default)]
    pub waveform: Option<String>,
    #[serde(default)]
    pub view_once: Option<bool>,
}

// ── Conteudo de mensagem: Documento ────────────────────────────────────
// MediaFile + MessagePropsDocument{caption, filename} + {page_count, preview} + ActionButtons + ViewOnce

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentContent {
    pub id: String,
    #[serde(default)]
    pub link: Option<String>,
    pub mime_type: String,
    pub file_size: i64,
    #[serde(default)]
    pub file_name: Option<String>,
    #[serde(default)]
    pub sha256: Option<String>,
    #[serde(default)]
    pub timestamp: Option<f64>,
    #[serde(default)]
    pub caption: Option<String>,
    #[serde(default)]
    pub filename: Option<String>,
    #[serde(default)]
    pub page_count: Option<i32>,
    #[serde(default)]
    pub preview: Option<String>,
    #[serde(default)]
    pub view_once: Option<bool>,
    #[serde(default)]
    pub buttons: Option<Vec<serde_json::Value>>,
}

// ── Conteudo de mensagem: Sticker ──────────────────────────────────────
// MediaFile + MessagePropsSticker{animated} + {preview} + Size + ViewOnce

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StickerContent {
    pub id: String,
    #[serde(default)]
    pub link: Option<String>,
    pub mime_type: String,
    pub file_size: i64,
    #[serde(default)]
    pub file_name: Option<String>,
    #[serde(default)]
    pub sha256: Option<String>,
    #[serde(default)]
    pub timestamp: Option<f64>,
    #[serde(default)]
    pub animated: Option<bool>,
    #[serde(default)]
    pub preview: Option<String>,
    #[serde(default)]
    pub width: Option<i32>,
    #[serde(default)]
    pub height: Option<i32>,
    #[serde(default)]
    pub view_once: Option<bool>,
}

// ── Conteudo de mensagem: Location ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocationContent {
    pub latitude: f64,
    pub longitude: f64,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub preview: Option<String>,
    #[serde(default)]
    pub accuracy: Option<i32>,
    #[serde(default)]
    pub speed: Option<i32>,
    #[serde(default)]
    pub degrees: Option<i32>,
    #[serde(default)]
    pub comment: Option<String>,
    #[serde(default)]
    pub view_once: Option<bool>,
}

// ── Conteudo de mensagem: Live Location ────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiveLocationContent {
    pub latitude: f64,
    pub longitude: f64,
    #[serde(default)]
    pub accuracy: Option<i32>,
    #[serde(default)]
    pub speed: Option<i32>,
    #[serde(default)]
    pub degrees: Option<i32>,
    #[serde(default)]
    pub caption: Option<String>,
    #[serde(default)]
    pub sequence_number: Option<i64>,
    #[serde(default)]
    pub time_offset: Option<f64>,
    #[serde(default)]
    pub preview: Option<String>,
    #[serde(default)]
    pub view_once: Option<bool>,
}

// ── Conteudo de mensagem: Contact (VCard) ──────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContactMsgContent {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub vcard: Option<String>,
}

// ── Conteudo de mensagem: Contact List ─────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContactListContent {
    pub list: Vec<ContactMsgContent>,
}

// ── Conteudo de mensagem: Link Preview ─────────────────────────────────
// Usado tambem por group_invite, newsletter_invite, catalog

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinkPreviewContent {
    pub body: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub link: Option<String>,
    #[serde(default)]
    pub sha256: Option<String>,
    #[serde(default)]
    pub catalog_id: Option<String>,
    #[serde(default)]
    pub newsletter_id: Option<String>,
    #[serde(default)]
    pub invite_code: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub canonical_url: Option<String>,
    #[serde(default, rename = "type")]
    pub preview_type: Option<String>,
    #[serde(default)]
    pub preview: Option<String>,
    #[serde(default)]
    pub thumbnail: Option<String>,
    #[serde(default)]
    pub view_once: Option<bool>,
}

// ── Metadado: Message Context ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MessageContext {
    #[serde(default)]
    pub forwarded: Option<bool>,
    #[serde(default)]
    pub forwarding_score: Option<i32>,
    #[serde(default)]
    pub mentions: Option<Vec<String>>,
    #[serde(default)]
    pub ad: Option<serde_json::Value>,
    #[serde(default)]
    pub conversion: Option<serde_json::Value>,
    #[serde(default)]
    pub quoted_id: Option<String>,
    #[serde(default)]
    pub quoted_type: Option<String>,
    #[serde(default)]
    pub quoted_content: Option<serde_json::Value>,
    #[serde(default)]
    pub quoted_author: Option<String>,
    #[serde(default)]
    pub ephemeral: Option<i32>,
}

// ── Metadado: Message Action ───────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MessageAction {
    #[serde(rename = "type")]
    pub action_type: String,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub emoji: Option<String>,
    #[serde(default)]
    pub ephemeral: Option<i32>,
    #[serde(default)]
    pub edited_type: Option<String>,
    #[serde(default)]
    pub edited_content: Option<serde_json::Value>,
    #[serde(default)]
    pub votes: Option<Vec<String>>,
    #[serde(default)]
    pub comment: Option<String>,
    #[serde(default)]
    pub event_response: Option<serde_json::Value>,
}

// ── Canal: Health ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HealthData {
    pub status: HealthStatus,
    pub uptime: i64,
    pub start_at: i64,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub core_version: Option<String>,
    #[serde(default)]
    pub api_version: Option<String>,
    #[serde(default)]
    pub device_id: Option<f64>,
    #[serde(default)]
    pub ip: Option<String>,
    #[serde(default)]
    pub channel_id: Option<String>,
    #[serde(default)]
    pub user: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HealthStatus {
    pub code: i32,
    pub text: String,
}

// ── Canal: QR ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QrData {
    pub status: String,
    #[serde(default)]
    pub base64: Option<String>,
    #[serde(default)]
    pub rowdata: Option<String>,
    #[serde(default)]
    pub expire: Option<i32>,
}
