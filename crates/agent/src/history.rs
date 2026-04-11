use chrono::{DateTime, Utc};
use cubos_sql::sql;
use deadpool_postgres::Pool;
use serde_json::Value;

use crate::dispatch::Models;

/// Mensagem do histórico de conversa do WhatsApp.
pub struct ConversationMessage {
    pub from_me: bool,
    pub text: String,
    pub timestamp: Option<DateTime<Utc>>,
    /// Imagens anexadas a esta mensagem. Cada imagem tem um ID (do
    /// WhatsApp) que é mencionado no `text` para o modelo correlacionar
    /// o anexo com a parte que virá depois da user message.
    pub images: Vec<HistoryImage>,
}

/// Imagem anexada a uma [`ConversationMessage`]. Os bytes já foram
/// baixados (e cacheados em disco) pelo `fetch_history`.
pub struct HistoryImage {
    pub id: String,
    pub mime_type: String,
    pub bytes: Vec<u8>,
}

pub async fn fetch_history(
    pool: &Pool,
    chat_id: &str,
    history_starts_at: Option<DateTime<Utc>>,
    ai: &ai::Client,
    models: &Models,
) -> anyhow::Result<(Vec<ConversationMessage>, Option<DateTime<Utc>>)> {
    struct Row {
        from_me: bool,
        msg_type: String,
        text_body: Option<String>,
        voice: Option<Value>,
        image: Option<Value>,
        timestamp: DateTime<Utc>,
    }

    let rows: Vec<Row> = sql!(
        pool,
        "SELECT from_me, msg_type, text_body, voice, image, \"timestamp\"
         FROM whatsapp.messages
         WHERE chat_id = $chat_id
           AND \"timestamp\" >= COALESCE($history_starts_at?, 'epoch'::timestamptz)
         ORDER BY \"timestamp\" DESC
         LIMIT 60"
    )
    .fetch_all()
    .await?
    .iter()
    .map(|r| Row {
        from_me: r.from_me,
        msg_type: r.msg_type.clone(),
        text_body: r.text_body.clone(),
        voice: r.voice.clone(),
        image: r.image.clone(),
        timestamp: r.timestamp,
    })
    .collect();

    let mut messages = Vec::new();
    let mut total_chars = 0usize;
    let max_ts: Option<DateTime<Utc>> = rows.iter().find(|r| !r.from_me).map(|r| r.timestamp);

    for row in &rows {
        let mut images: Vec<HistoryImage> = Vec::new();
        let text: String = match row.msg_type.as_str() {
            "text" => row.text_body.clone().unwrap_or_default(),
            "voice" => {
                let voice_id = row
                    .voice
                    .as_ref()
                    .and_then(|v| v.get("id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let link = row
                    .voice
                    .as_ref()
                    .and_then(|v| v.get("link"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if !voice_id.is_empty() {
                    match transcribe_voice(pool, ai, &models.transcription, voice_id, link).await {
                        Ok(t) => format!("[áudio transcrito]: {t}"),
                        Err(e) => {
                            tracing::warn!(voice_id, "Falha ao transcrever áudio: {e:#}");
                            "[áudio não transcrito]".into()
                        }
                    }
                } else {
                    "[áudio]".into()
                }
            }
            "image" => {
                let img_meta = row.image.as_ref();
                let id = img_meta
                    .and_then(|v| v.get("id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let link = img_meta
                    .and_then(|v| v.get("link"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let mime = img_meta
                    .and_then(|v| v.get("mime_type"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("image/jpeg");
                let caption = img_meta
                    .and_then(|v| v.get("caption"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_owned();

                if !id.is_empty() && !link.is_empty() {
                    match fetch_image_cached(id, mime, link).await {
                        Ok(bytes) => {
                            images.push(HistoryImage {
                                id: id.to_owned(),
                                mime_type: mime.to_owned(),
                                bytes,
                            });
                            if caption.is_empty() {
                                "[imagem]".into()
                            } else {
                                format!("[imagem] {caption}")
                            }
                        }
                        Err(e) => {
                            tracing::warn!(image_id = id, "Falha ao baixar imagem: {e:#}");
                            if caption.is_empty() {
                                "[imagem não carregada]".into()
                            } else {
                                format!("[imagem não carregada] {caption}")
                            }
                        }
                    }
                } else {
                    "[imagem sem metadados]".into()
                }
            }
            other => {
                tracing::debug!(msg_type = other, "Tipo de mensagem ignorado no histórico");
                continue;
            }
        };

        // Orçamento de caracteres: limita o histórico para não estourar
        // contexto. Imagens não contam aqui (bytes ≠ tokens) — o limite
        // efetivo de imagens vem do próprio LIMIT 60 da query.
        total_chars += text.len();
        if total_chars > 10_000 && !messages.is_empty() {
            break;
        }
        messages.push(ConversationMessage {
            from_me: row.from_me,
            text,
            timestamp: Some(row.timestamp),
            images,
        });
    }

    messages.reverse();
    Ok((messages, max_ts))
}

/// Baixa uma imagem do WhatsApp e cacheia em disco por ID. Retorna os
/// bytes crus. O cache é best-effort: falhas de escrita/leitura são
/// silenciosas e caem no download direto.
async fn fetch_image_cached(id: &str, mime_type: &str, link: &str) -> anyhow::Result<Vec<u8>> {
    let cache_dir = std::env::temp_dir().join("zain-images");
    let _ = std::fs::create_dir_all(&cache_dir);

    let ext = mime_type
        .rsplit('/')
        .next()
        .filter(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric()))
        .unwrap_or("bin");
    let safe_id: String = id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let cache_path = cache_dir.join(format!("{safe_id}.{ext}"));

    if let Ok(bytes) = std::fs::read(&cache_path) {
        tracing::debug!(image_id = id, "imagem lida do cache");
        return Ok(bytes);
    }

    let bytes = reqwest::Client::new()
        .get(link)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?
        .to_vec();

    let _ = std::fs::write(&cache_path, &bytes);
    tracing::debug!(image_id = id, size = bytes.len(), "imagem baixada");
    Ok(bytes)
}

async fn transcribe_voice(
    pool: &Pool,
    ai: &ai::Client,
    transcription_model: &str,
    voice_id: &str,
    download_link: &str,
) -> anyhow::Result<String> {
    let cached: Option<String> = sql!(
        pool,
        "SELECT transcription FROM zain.audio_transcriptions WHERE id = $voice_id"
    )
    .fetch_optional()
    .await?
    .map(|r| r.transcription);

    if let Some(transcription) = cached {
        return Ok(transcription);
    }

    let audio_bytes = reqwest::Client::new()
        .get(download_link)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;

    let transcription = ai
        .transcribe(
            transcription_model,
            audio_bytes.to_vec(),
            "audio.ogg",
            "audio/ogg",
        )
        .await?;

    let transcription_ref = &transcription;
    sql!(
        pool,
        "INSERT INTO zain.audio_transcriptions (id, transcription)
         VALUES ($voice_id, $transcription_ref)
         ON CONFLICT (id) DO NOTHING"
    )
    .execute()
    .await?;

    Ok(transcription)
}
