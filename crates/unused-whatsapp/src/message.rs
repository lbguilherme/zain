//! Message reading and media downloads for an open chat.

use std::path::Path;
use std::time::Duration;

use chrono::NaiveDateTime;
use sha2::{Digest, Sha256};

use chromium_driver::PageSession;

use crate::error::{Result, WhatsappError};
use crate::types::{DataId, MessageType, RawMessage};

/// Scrollable container for messages inside an open chat.
pub(crate) const MSG_SCROLL_CONTAINER: &str =
    r#"div[data-scrolltracepolicy="wa.web.conversation.messages"]"#;

// ── Read messages ────────────────────────────────────────────────────────

/// Reads all visible messages in the currently open chat.
///
/// A single JS call extracts all message metadata from the DOM.
/// Media (stickers, images) are downloaded via separate interactions.
pub(crate) async fn read_visible_messages(
    page: &PageSession,
    media_dir: &Path,
) -> Result<Vec<RawMessage>> {
    let js_msgs: Vec<JsMsgData> = serde_json::from_value(
        page.eval_value(
            r#"(() => {
                const SYSTEM_TEXTS = ['secure service from Meta'];
                const msgs = document.querySelectorAll('div[data-id]:not(div[data-id] div[data-id])');
                return Array.from(msgs).map(el => {
                    const dataId = el.getAttribute('data-id');
                    if (!dataId) return null;
                    const hasDir = !!el.querySelector('div[class*="message-in"], div[class*="message-out"]');
                    let msgType = 'unknown';
                    if (!hasDir) {
                        const t = el.textContent || '';
                        for (const p of SYSTEM_TEXTS) { if (t.includes(p)) { msgType = 'system'; break; } }
                    } else if (el.querySelector('[label^="Sticker with"]') || el.querySelector('img[alt^="Sticker with"]')) {
                        msgType = 'sticker';
                    } else if (el.querySelector('[aria-label="Open picture"]')) {
                        msgType = 'image';
                    } else if (el.querySelector('[aria-label="Voice message"]')) {
                        msgType = 'voice';
                    } else if (el.querySelector('[data-icon="ic-videocam"]')) {
                        msgType = 'video';
                    } else {
                        const te = el.querySelector('span[data-testid="selectable-text"]');
                        if (te && te.textContent.trim()) msgType = 'text';
                    }
                    let text = null;
                    if (['text','image','video'].includes(msgType)) {
                        const te = el.querySelector('span[data-testid="selectable-text"]');
                        if (te) { const t = te.textContent.trim(); if (t) text = t; }
                    } else if (msgType === 'sticker') {
                        const s = el.querySelector('[label^="Sticker with"]');
                        if (s) text = s.getAttribute('label');
                        if (!text) { const img = el.querySelector('img[alt^="Sticker with"]'); if (img) text = img.getAttribute('alt'); }
                    } else if (msgType === 'system') {
                        const t = (el.textContent || '').trim(); if (t) text = t;
                    }
                    const copyable = el.querySelector('div[data-pre-plain-text]');
                    const prePlainText = copyable ? copyable.getAttribute('data-pre-plain-text') : null;
                    let stickerBlobUrl = null;
                    if (msgType === 'sticker') {
                        const img = el.querySelector('img[alt^="Sticker with"]');
                        if (img && img.src && img.src.startsWith('blob:')) stickerBlobUrl = img.src;
                    }
                    return { dataId, msgType, text, prePlainText, stickerBlobUrl };
                }).filter(r => r !== null);
            })()"#,
        )
        .await?,
    )
    .unwrap_or_default();

    let mut messages = Vec::new();
    let mut skipped = 0u32;

    for js_msg in &js_msgs {
        let Some(data_id) = DataId::parse(&js_msg.data_id) else {
            tracing::debug!(raw_id = %js_msg.data_id, "Could not parse data-id");
            skipped += 1;
            continue;
        };

        let msg_type = match js_msg.msg_type.as_str() {
            "text" => MessageType::Text,
            "image" => MessageType::Image,
            "sticker" => MessageType::Sticker,
            "voice" => MessageType::Voice,
            "video" => MessageType::Video,
            "system" => MessageType::System,
            _ => {
                skipped += 1;
                continue;
            }
        };

        let sender_jid = if data_id.outgoing {
            None
        } else {
            Some(
                data_id
                    .sender_lid
                    .clone()
                    .unwrap_or_else(|| data_id.chat_jid.clone()),
            )
        };

        let (timestamp, sender_name) = js_msg
            .pre_plain_text
            .as_deref()
            .map(parse_pre_plain_text)
            .unwrap_or((None, None));

        let sticker_media = if msg_type == MessageType::Sticker {
            if let Some(blob_url) = &js_msg.sticker_blob_url {
                save_media(page, media_dir, blob_url, "sticker")
                    .await
                    .unwrap_or_else(|e| {
                        tracing::warn!("Failed to download sticker: {e:#}");
                        None
                    })
            } else {
                None
            }
        } else {
            None
        };

        let image_media = if msg_type == MessageType::Image {
            download_image(page, media_dir, &js_msg.data_id)
                .await
                .unwrap_or_else(|e| {
                    tracing::warn!("Failed to download image: {e:#}");
                    None
                })
        } else {
            None
        };

        messages.push(RawMessage {
            data_id,
            msg_type,
            text: js_msg.text.clone(),
            sender_jid,
            sender_name,
            timestamp,
            sticker_media,
            image_media,
        });
    }

    if skipped > 0 {
        tracing::debug!(parsed = messages.len(), skipped, "read_visible_messages");
    }

    Ok(messages)
}

#[derive(serde::Deserialize)]
struct JsMsgData {
    #[serde(rename = "dataId")]
    data_id: String,
    #[serde(rename = "msgType")]
    msg_type: String,
    text: Option<String>,
    #[serde(rename = "prePlainText")]
    pre_plain_text: Option<String>,
    #[serde(rename = "stickerBlobUrl")]
    sticker_blob_url: Option<String>,
}

// ── Timestamp parsing ────────────────────────────────────────────────────

/// Parses the `data-pre-plain-text` attribute value.
///
/// Format: `[14:14, 3/30/2026] Guilherme Bernal: `
fn parse_pre_plain_text(s: &str) -> (Option<NaiveDateTime>, Option<String>) {
    let s = s.trim();

    let Some(bracket_start) = s.find('[') else {
        return (None, None);
    };
    let Some(bracket_end) = s.find(']') else {
        return (None, None);
    };
    let inside = &s[bracket_start + 1..bracket_end];

    let timestamp = if let Some((time_part, date_part)) = inside.split_once(", ") {
        let time_part = time_part.trim();
        let date_part = date_part.trim();
        let datetime_str = format!("{date_part} {time_part}");
        NaiveDateTime::parse_from_str(&datetime_str, "%-m/%-d/%Y %H:%M")
            .or_else(|_| NaiveDateTime::parse_from_str(&datetime_str, "%d/%m/%Y %H:%M"))
            .ok()
    } else {
        None
    };

    let after_bracket = s[bracket_end + 1..].trim();
    let sender = if let Some(colon_pos) = after_bracket.rfind(':') {
        let name = after_bracket[..colon_pos].trim();
        if name.is_empty() {
            None
        } else {
            Some(name.to_owned())
        }
    } else {
        None
    };

    (timestamp, sender)
}

// ── Media downloads ──────────────────────────────────────────────────────

/// Fetches a blob URL and saves it to `media_dir/{prefix}_{sha256}.{ext}`.
///
/// The file extension is derived from the blob's MIME type.
async fn save_media(
    page: &PageSession,
    media_dir: &Path,
    blob_url: &str,
    prefix: &str,
) -> Result<Option<String>> {
    let (bytes, mime) = page.fetch_blob_url_typed(blob_url).await?;
    if bytes.is_empty() {
        return Ok(None);
    }
    let ext = mime_to_ext(&mime);
    let hash = Sha256::digest(&bytes);
    let filename = format!("{prefix}_{:x}.{ext}", hash);
    let path = media_dir.join(&filename);
    if !path.exists() {
        std::fs::write(&path, &bytes)?;
        tracing::debug!(filename = %filename, mime = %mime, size = bytes.len(), "Saved media");
    }
    Ok(Some(filename))
}

fn mime_to_ext(mime: &str) -> &str {
    match mime {
        "image/webp" => "webp",
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/gif" => "gif",
        "video/mp4" => "mp4",
        "audio/ogg" => "ogg",
        "audio/mpeg" => "mp3",
        _ => "bin",
    }
}

/// Downloads an image by clicking to open fullscreen, finding the blob URL
/// via JS, downloading, then closing the viewer.
async fn download_image(
    page: &PageSession,
    media_dir: &Path,
    msg_data_id: &str,
) -> Result<Option<String>> {
    let dom = page.dom().await?;

    // Find and click the "Open picture" button inside this message.
    let escaped = msg_data_id.replace('"', r#"\""#);
    let selector = format!(r#"div[data-id="{}"] [aria-label="Open picture"]"#, escaped);
    let Some(open_btn) = dom.try_query_selector(&selector).await? else {
        return Ok(None);
    };

    open_btn.click().await?;
    tokio::time::sleep(Duration::from_millis(1200)).await;

    // JS: find the fullscreen blob img URL.
    let result = async {
        let blob_url = page
            .eval_value(
                r#"(() => {
                    const specific = document.querySelectorAll('img[crossorigin="anonymous"][src^="blob:"]');
                    if (specific.length > 0) return specific[0].src;
                    const any = document.querySelectorAll('img[src^="blob:"]');
                    for (const img of any) { if (img.src.startsWith('blob:')) return img.src; }
                    return null;
                })()"#,
            )
            .await?;
        let Some(url) = blob_url.as_str() else {
            tracing::warn!("download_image: no blob img found in fullscreen viewer");
            let _ = page.debug_dump("download_image_no_blob").await;
            return Ok::<_, WhatsappError>(None);
        };
        save_media(page, media_dir, url, "image").await
    }
    .await;

    // Close the fullscreen viewer by pressing Escape.
    if let Some(body) = dom.try_query_selector("body").await? {
        let _ = body.press_key("Escape").await;
    }
    tokio::time::sleep(Duration::from_millis(400)).await;

    result
}

// ── Scroll helpers ───────────────────────────────────────────────────────

/// Scrolls up in the message panel to load older messages.
pub(crate) async fn scroll_up_messages(page: &PageSession) -> Result<()> {
    let dom = page.dom().await?;
    let Some(container) = dom.try_query_selector(MSG_SCROLL_CONTAINER).await? else {
        let _ = page.debug_dump("scroll_up_no_container").await;
        return Err(WhatsappError::SelectorNotFound(MSG_SCROLL_CONTAINER));
    };

    container.swipe_down(500.0).await?;
    tokio::time::sleep(Duration::from_millis(500)).await;
    Ok(())
}

/// Scrolls the message panel to the bottom via repeated swipe-up gestures
/// until the last visible message stops changing.
pub(crate) async fn scroll_to_bottom(page: &PageSession) -> Result<()> {
    let dom = page.dom().await?;
    let mut last_bottom_id: Option<String> = None;

    for _ in 0..50 {
        let val = page
            .eval_value(
                r#"(() => {
                    const msgs = document.querySelectorAll('div[data-id]:not(div[data-id] div[data-id])');
                    return msgs.length > 0 ? msgs[msgs.length - 1].getAttribute('data-id') || '' : '';
                })()"#,
            )
            .await?;
        let current = val.as_str().unwrap_or("").to_owned();

        if last_bottom_id.as_deref() == Some(&current) {
            break;
        }
        last_bottom_id = Some(current);

        let Some(container) = dom.try_query_selector(MSG_SCROLL_CONTAINER).await? else {
            break;
        };
        if container.swipe_up(600.0).await.is_err() {
            tokio::time::sleep(Duration::from_millis(200)).await;
            continue;
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
    }

    Ok(())
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Timelike};

    #[test]
    fn parse_pre_plain_text_basic() {
        let (ts, sender) = parse_pre_plain_text("[14:14, 3/30/2026] Guilherme Bernal: ");
        assert!(ts.is_some());
        let ts = ts.unwrap();
        assert_eq!(ts.hour(), 14);
        assert_eq!(ts.minute(), 14);
        assert_eq!(ts.month(), 3);
        assert_eq!(ts.day(), 30);
        assert_eq!(ts.year(), 2026);
        assert_eq!(sender.as_deref(), Some("Guilherme Bernal"));
    }

    #[test]
    fn parse_pre_plain_text_phone() {
        let (ts, sender) = parse_pre_plain_text("[17:18, 3/19/2026] +55 71 8466-9177: ");
        assert!(ts.is_some());
        assert_eq!(sender.as_deref(), Some("+55 71 8466-9177"));
    }
}
