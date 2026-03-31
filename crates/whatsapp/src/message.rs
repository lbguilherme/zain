//! Chat opening, message reading, and chat closing.

use std::time::Duration;

use chrono::NaiveDateTime;
use std::path::Path;

use chromium_driver::dom::{Dom, Element, HumanDelay};
use chromium_driver::PageSession;
use sha2::{Digest, Sha256};

use crate::error::{Result, WhatsappError};
use crate::types::{DataId, MessageType, RawMessage};

/// Selector for the chat message input box (appears when a chat is open).
const MESSAGE_INPUT: &str = r#"div[role="textbox"][data-tab="10"]"#;

/// Scrollable container for messages inside an open chat.
const MSG_SCROLL_CONTAINER: &str =
    r#"div[data-scrolltracepolicy="wa.web.conversation.messages"]"#;

/// Each message element has a `data-id` attribute.
const MSG_ELEMENT: &str = r#"div[data-id]"#;

/// Menu button inside the chat header (kebab "...").
const CHAT_MENU_BUTTON: &str = r#"button[data-tab="6"][aria-label="Menu"]"#;

/// Chat list grid selector.
const CHAT_LIST: &str = r#"div[role="grid"][aria-label="Chat list"]"#;

/// Opens a chat by clicking on its row in the sidebar.
///
/// Scrolls the sidebar to find and position the target chat row in a safe
/// vertical zone before clicking. Matches titles after stripping bidi chars.
pub(crate) async fn open_chat(dom: &Dom, title: &str, timing: &HumanDelay) -> Result<()> {
    let mut clicked = false;
    for attempt in 0..50 {
        tracing::info!(title, attempt, "find_chat_row...");
        let found = find_chat_row(dom, title).await?;

        let Some(row_el) = found else {
            tracing::debug!(title, attempt, "Not found, scrolling down");
            let Some(grid) = dom.try_query_selector(CHAT_LIST).await? else {
                tracing::debug!(title, "Chat list grid not found, breaking");
                break;
            };
            let _ = grid.swipe_up(300.0, timing).await;
            tokio::time::sleep(Duration::from_millis(300)).await;
            continue;
        };

        tracing::debug!(title, attempt, "Found row, getting box model");

        // Check vertical position: avoid clicking near the bottom (banner zone).
        let el_center_y = match row_el.box_model().await {
            Ok(bm) if bm.content.len() >= 8 => {
                let cy = (bm.content[1] + bm.content[5]) / 2.0;
                tracing::debug!(title, attempt, cy, "Row center Y");
                cy
            }
            Ok(bm) => {
                tracing::debug!(title, attempt, content_len = bm.content.len(), "Invalid box model");
                tokio::time::sleep(Duration::from_millis(200)).await;
                continue;
            }
            Err(e) => {
                tracing::debug!(title, attempt, "box_model failed: {e:#}");
                tokio::time::sleep(Duration::from_millis(200)).await;
                continue;
            }
        };

        let grid_bounds = match dom.try_query_selector(CHAT_LIST).await? {
            Some(g) => match g.box_model().await {
                Ok(bm) if bm.content.len() >= 8 => {
                    let top = bm.content[1];
                    let bottom = bm.content[5];
                    tracing::debug!(title, attempt, top, bottom, "Grid bounds");
                    Some((top, bottom))
                }
                _ => None,
            },
            None => None,
        };

        if let Some((grid_top, grid_bottom)) = grid_bounds {
            let grid_height = grid_bottom - grid_top;
            let safe_bottom = grid_top + grid_height * 0.75;
            let ideal_center = grid_top + grid_height * 0.4;

            if el_center_y > safe_bottom {
                let swipe_dist = (el_center_y - ideal_center).min(grid_height * 0.5);
                tracing::debug!(title, attempt, el_center_y, safe_bottom, swipe_dist, "Too low, swiping up");
                if let Some(grid) = dom.try_query_selector(CHAT_LIST).await? {
                    let _ = grid.swipe_up(swipe_dist, timing).await;
                    tokio::time::sleep(Duration::from_millis(300)).await;
                    continue;
                }
            }

            if el_center_y < grid_top {
                let swipe_dist =
                    (grid_top - el_center_y + grid_height * 0.3).min(grid_height * 0.5);
                tracing::debug!(title, attempt, el_center_y, grid_top, swipe_dist, "Too high, swiping down");
                if let Some(grid) = dom.try_query_selector(CHAT_LIST).await? {
                    let _ = grid.swipe_down(swipe_dist, timing).await;
                    tokio::time::sleep(Duration::from_millis(300)).await;
                    continue;
                }
            }
        }

        tracing::debug!(title, attempt, "Clicking...");
        match row_el.click(timing).await {
            Ok(()) => {
                tracing::debug!(title, "Click succeeded");
                clicked = true;
                break;
            }
            Err(e) => {
                tracing::debug!(title, attempt, "Click failed: {e:#}");
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        }
    }

    if !clicked {
        // Dump DOM for debugging.
        tracing::warn!(title, "Could not find/click chat row, dumping DOM");
        dump_debug(dom, "open_chat_failed").await;
        return Err(WhatsappError::SelectorNotFound("chat row by title"));
    }

    if let Err(_) = dom.wait_for(MESSAGE_INPUT, Duration::from_secs(5)).await {
        tracing::warn!(title, "Message panel did not appear, dumping DOM");
        dump_debug(dom, "message_panel_timeout").await;
        return Err(WhatsappError::Timeout("chat message panel".into()));
    }

    tokio::time::sleep(Duration::from_millis(500)).await;
    scroll_to_bottom(dom).await?;
    tracing::debug!(title, "Chat opened and scrolled to bottom");

    Ok(())
}

/// Finds a chat row by title using a CSS selector with the title attribute.
/// Much faster than iterating all rows — single CDP call.
async fn find_chat_row(dom: &Dom, target_title: &str) -> Result<Option<Element>> {
    // Use CSS attribute selector to find the span directly.
    let selector = format!(
        r#"div[role="grid"][aria-label="Chat list"] div[role="row"] span[title="{}"]"#,
        css_escape_attr(target_title)
    );

    let span = match dom.try_query_selector(&selector).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            tracing::debug!(target_title, "CSS selector found no match");
            return Ok(None);
        }
        Err(e) => {
            tracing::debug!(target_title, "CSS selector error: {e:#}");
            return Ok(None);
        }
    };

    // Navigate up to the row element: the span is inside the row.
    // We can't go to parent in CDP easily, so we re-query the row
    // that contains this specific title.
    let row_selector = format!(
        r#"div[role="grid"][aria-label="Chat list"] div[role="row"]:has(span[title="{}"])"#,
        css_escape_attr(target_title)
    );

    match dom.try_query_selector(&row_selector).await {
        Ok(Some(row)) => Ok(Some(row)),
        Ok(None) => {
            // Fallback: click the span itself.
            tracing::debug!(target_title, ":has() selector failed, using span directly");
            Ok(Some(span))
        }
        Err(e) => {
            tracing::debug!(target_title, ":has() selector error: {e:#}, using span");
            Ok(Some(span))
        }
    }
}

/// Escapes a string for use inside a CSS attribute selector value.
fn css_escape_attr(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}


/// Saves a debug DOM dump to `dumps/{name}.html`.
async fn dump_debug(dom: &Dom, name: &str) {
    let dir = std::path::Path::new("dumps");
    let _ = std::fs::create_dir_all(dir);
    if let Ok(html) = dom.page_html().await {
        let path = dir.join(format!("{name}.html"));
        let _ = std::fs::write(&path, &html);
        tracing::info!("Debug dump saved to {}", path.display());
    }
}

/// Gets the data-id of the last visible message element.
async fn get_last_msg_id(dom: &Dom) -> Result<String> {
    let msgs = dom.query_selector_all(MSG_ELEMENT).await?;
    if let Some(last) = msgs.last() {
        Ok(last.attribute("data-id").await?.unwrap_or_default())
    } else {
        Ok(String::new())
    }
}

/// Scrolls the message panel to the bottom (most recent messages) via
/// repeated swipe-up gestures until no new content appears.
/// Re-queries elements each iteration to avoid stale node IDs.
async fn scroll_to_bottom(dom: &Dom) -> Result<()> {
    let timing = crate::timing();
    let mut last_bottom_id: Option<String> = None;

    for _ in 0..50 {
        let current_bottom_id = match get_last_msg_id(dom).await {
            Ok(id) => id,
            Err(_) => {
                tokio::time::sleep(Duration::from_millis(200)).await;
                continue;
            }
        };

        if last_bottom_id.as_deref() == Some(&current_bottom_id) {
            break;
        }
        last_bottom_id = Some(current_bottom_id);

        let Some(container) = dom.try_query_selector(MSG_SCROLL_CONTAINER).await? else {
            break;
        };
        if container.swipe_up(600.0, &timing).await.is_err() {
            tokio::time::sleep(Duration::from_millis(200)).await;
            continue;
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
    }

    Ok(())
}

/// Closes the currently open chat via "..." menu → "Close chat".
pub(crate) async fn close_chat(dom: &Dom, timing: &HumanDelay) -> Result<()> {
    let menu_btn = dom
        .try_query_selector(CHAT_MENU_BUTTON)
        .await?
        .ok_or(WhatsappError::SelectorNotFound(CHAT_MENU_BUTTON))?;

    menu_btn.click(timing).await?;
    tokio::time::sleep(Duration::from_millis(400)).await;

    // TODO: remove after confirming selectors
    dump_menu_once(dom).await;

    // Find "Close chat" in the dropdown menu.
    let menu_items = dom.query_selector_all(r#"li[role="button"]"#).await?;
    for item in &menu_items {
        let text = item.text().await.unwrap_or_default();
        if text.contains("Close chat") || text.contains("Fechar conversa") {
            item.click(timing).await?;
            dom.wait_for(
                crate::client::AUTHENTICATED_SELECTOR,
                Duration::from_secs(5),
            )
            .await
            .map_err(|_| WhatsappError::Timeout("close chat → chat list".into()))?;
            return Ok(());
        }
    }

    Err(WhatsappError::SelectorNotFound("Close chat menu item"))
}

/// Reads all visible messages in the currently open chat.
pub(crate) async fn read_visible_messages(
    dom: &Dom,
    page: &PageSession,
    media_dir: &Path,
) -> Result<Vec<RawMessage>> {
    let msg_elements = dom.query_selector_all(MSG_ELEMENT).await?;
    let mut messages = Vec::new();
    let mut skipped = 0u32;

    for el in &msg_elements {
        match parse_message(el, page, media_dir, &crate::timing()).await {
            Ok(Some(msg)) => messages.push(msg),
            Ok(None) => {
                skipped += 1;
                if let Ok(Some(raw_id)) = el.attribute("data-id").await {
                    tracing::trace!(data_id = %raw_id, "Skipped message (parse returned None)");
                }
            }
            Err(e) => {
                tracing::warn!("Error parsing message element: {e:#}");
            }
        }
    }

    if skipped > 0 {
        tracing::debug!(parsed = messages.len(), skipped, "read_visible_messages");
    }

    Ok(messages)
}

/// Scrolls up in the message panel to load older messages.
pub(crate) async fn scroll_up_messages(dom: &Dom, timing: &HumanDelay) -> Result<()> {
    let container = dom
        .try_query_selector(MSG_SCROLL_CONTAINER)
        .await?
        .ok_or(WhatsappError::SelectorNotFound(MSG_SCROLL_CONTAINER))?;

    container.swipe_up(500.0, timing).await?;
    tokio::time::sleep(Duration::from_millis(500)).await;
    Ok(())
}

/// Parses a single message element (`div[data-id]`).
async fn parse_message(el: &Element, page: &PageSession, media_dir: &Path, timing: &HumanDelay) -> Result<Option<RawMessage>> {
    let raw_id = match el.attribute("data-id").await? {
        Some(id) if !id.is_empty() => id,
        _ => return Ok(None),
    };

    let data_id = match DataId::parse(&raw_id) {
        Some(id) => id,
        None => {
            tracing::debug!(raw_id = %raw_id, "Could not parse data-id");
            return Ok(None);
        }
    };

    let is_from_me = data_id.outgoing;

    // Sender JID: for 1:1 chats it's the chat_jid (the other person);
    // for groups, it's the sender_lid. For outgoing, it's us.
    let sender_jid = if is_from_me {
        None
    } else {
        Some(
            data_id
                .sender_lid
                .clone()
                .unwrap_or_else(|| data_id.chat_jid.clone()),
        )
    };

    // Detect message type.
    let msg_type = detect_message_type(el).await?;

    // Extract text.
    let text = extract_text(el, &msg_type).await?;

    // Parse timestamp and sender name from data-pre-plain-text.
    let (timestamp, sender_name) = extract_pre_plain_text(el).await?;

    // Download sticker media if applicable.
    let sticker_media = if msg_type == MessageType::Sticker {
        download_sticker(el, page, media_dir).await.unwrap_or_else(|e| {
            tracing::warn!("Failed to download sticker: {e:#}");
            None
        })
    } else {
        None
    };

    // Download image media if applicable.
    let image_media = if msg_type == MessageType::Image {
        download_image(el, page, media_dir, timing).await.unwrap_or_else(|e| {
            tracing::warn!("Failed to download image: {e:#}");
            None
        })
    } else {
        None
    };

    Ok(Some(RawMessage {
        raw_id: raw_id.clone(),
        data_id,
        msg_type,
        text,
        sender_jid,
        sender_name,
        is_from_me,
        timestamp,
        sticker_media,
        image_media,
    }))
}

/// System message texts that should be captured.
const SYSTEM_TEXTS: &[&str] = &[
    "secure service from Meta",
];

/// Detects the message type using stable selectors.
async fn detect_message_type(el: &Element) -> Result<MessageType> {
    // System messages: no message-in/message-out class, contain known text.
    let has_direction = el
        .try_query_selector(r#"div[class*="message-in"], div[class*="message-out"]"#)
        .await?
        .is_some();

    if !has_direction {
        // Check if it's a known system message.
        let text = el.text().await.unwrap_or_default();
        for pattern in SYSTEM_TEXTS {
            if text.contains(pattern) {
                return Ok(MessageType::System);
            }
        }
        // Unknown non-message element (date separator, etc.) — skip.
        return Ok(MessageType::Unknown);
    }

    // Sticker
    if el
        .try_query_selector(r#"[label^="Sticker with"]"#)
        .await?
        .is_some()
        || el
            .try_query_selector(r#"img[alt^="Sticker with"]"#)
            .await?
            .is_some()
    {
        return Ok(MessageType::Sticker);
    }

    // Image
    if el
        .try_query_selector(r#"[aria-label="Open picture"]"#)
        .await?
        .is_some()
    {
        return Ok(MessageType::Image);
    }

    // Voice
    if el
        .try_query_selector(r#"[aria-label="Voice message"]"#)
        .await?
        .is_some()
    {
        return Ok(MessageType::Voice);
    }

    // Video
    if el
        .try_query_selector(r#"[data-icon="ic-videocam"]"#)
        .await?
        .is_some()
    {
        return Ok(MessageType::Video);
    }

    // Text: has selectable-text with content
    if let Some(text_el) = el
        .try_query_selector(r#"span[data-testid="selectable-text"]"#)
        .await?
    {
        let text = text_el.text().await.unwrap_or_default();
        if !text.trim().is_empty() {
            return Ok(MessageType::Text);
        }
    }

    Ok(MessageType::Unknown)
}

/// Extracts text content based on message type.
async fn extract_text(el: &Element, msg_type: &MessageType) -> Result<Option<String>> {
    match msg_type {
        MessageType::Text => {
            if let Some(text_el) = el
                .try_query_selector(r#"span[data-testid="selectable-text"]"#)
                .await?
            {
                let t = text_el.text().await.unwrap_or_default().trim().to_owned();
                if !t.is_empty() {
                    return Ok(Some(t));
                }
            }
            Ok(None)
        }
        MessageType::Image | MessageType::Video => {
            // Caption text if present.
            if let Some(text_el) = el
                .try_query_selector(r#"span[data-testid="selectable-text"]"#)
                .await?
            {
                let t = text_el.text().await.unwrap_or_default().trim().to_owned();
                if !t.is_empty() {
                    return Ok(Some(t));
                }
            }
            Ok(None)
        }
        MessageType::Sticker => {
            if let Some(s) = el.try_query_selector(r#"[label^="Sticker with"]"#).await? {
                if let Some(label) = s.attribute("label").await? {
                    return Ok(Some(label));
                }
            }
            if let Some(img) = el.try_query_selector(r#"img[alt^="Sticker with"]"#).await? {
                if let Some(alt) = img.attribute("alt").await? {
                    return Ok(Some(alt));
                }
            }
            Ok(None)
        }
        MessageType::Voice => Ok(None),
        MessageType::System => {
            let t = el.text().await.unwrap_or_default().trim().to_owned();
            if !t.is_empty() { Ok(Some(t)) } else { Ok(None) }
        }
        MessageType::Unknown => Ok(None),
    }
}

/// Extracts timestamp and sender name from `data-pre-plain-text`.
///
/// Format: `[HH:MM, M/D/YYYY] Sender Name: `
async fn extract_pre_plain_text(
    el: &Element,
) -> Result<(Option<NaiveDateTime>, Option<String>)> {
    let Some(copyable) = el
        .try_query_selector(r#"div[data-pre-plain-text]"#)
        .await?
    else {
        return Ok((None, None));
    };

    let Some(attr) = copyable.attribute("data-pre-plain-text").await? else {
        return Ok((None, None));
    };

    Ok(parse_pre_plain_text(&attr))
}

/// Parses the `data-pre-plain-text` attribute value.
///
/// Format: `[14:14, 3/30/2026] Guilherme Bernal: `
fn parse_pre_plain_text(s: &str) -> (Option<NaiveDateTime>, Option<String>) {
    let s = s.trim();

    // Extract content between [ and ]
    let Some(bracket_start) = s.find('[') else {
        return (None, None);
    };
    let Some(bracket_end) = s.find(']') else {
        return (None, None);
    };
    let inside = &s[bracket_start + 1..bracket_end];

    // Parse time and date: "14:14, 3/30/2026"
    let timestamp = if let Some((time_part, date_part)) = inside.split_once(", ") {
        let time_part = time_part.trim();
        let date_part = date_part.trim();
        let datetime_str = format!("{date_part} {time_part}");
        // Try M/D/YYYY HH:MM
        NaiveDateTime::parse_from_str(&datetime_str, "%-m/%-d/%Y %H:%M")
            .or_else(|_| NaiveDateTime::parse_from_str(&datetime_str, "%d/%m/%Y %H:%M"))
            .ok()
    } else {
        None
    };

    // Extract sender name: everything after "] " and before the trailing ": "
    let after_bracket = s[bracket_end + 1..].trim();
    let sender = if let Some(colon_pos) = after_bracket.rfind(": ") {
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

/// Dumps the DOM once when the chat menu is open (for selector discovery).
async fn dump_menu_once(dom: &Dom) {
    use std::sync::atomic::{AtomicBool, Ordering};
    static DUMPED: AtomicBool = AtomicBool::new(false);
    if DUMPED.swap(true, Ordering::Relaxed) {
        return;
    }
    let dir = std::path::Path::new("dumps");
    let _ = std::fs::create_dir_all(dir);
    if let Ok(html) = dom.page_html().await {
        let _ = std::fs::write(dir.join("chat_menu_open.html"), &html);
        eprintln!("DOM dump saved to dumps/chat_menu_open.html");
    }
}

/// Downloads sticker image from a blob URL via CDP IO, saves to
/// `media_dir/sticker_{sha256}`, and returns the filename.
async fn download_sticker(
    el: &Element,
    page: &PageSession,
    media_dir: &Path,
) -> Result<Option<String>> {
    let Some(img) = el.try_query_selector(r#"img[alt^="Sticker with"]"#).await? else {
        return Ok(None);
    };
    let Some(src) = img.attribute("src").await? else {
        return Ok(None);
    };
    if !src.starts_with("blob:") {
        return Ok(None);
    }

    // Extract UUID from blob URL: "blob:https://web.whatsapp.com/<uuid>"
    let uuid = src
        .rsplit('/')
        .next()
        .ok_or_else(|| WhatsappError::Screenshot("no UUID in blob URL".into()))?;

    let bytes = page.read_blob(uuid).await?;

    if bytes.is_empty() {
        return Ok(None);
    }

    // Save with content-addressed filename.
    let hash = Sha256::digest(&bytes);
    let filename = format!("sticker_{:x}", hash);
    let path = media_dir.join(&filename);
    if !path.exists() {
        std::fs::write(&path, &bytes)?;
        tracing::debug!(filename = %filename, size = bytes.len(), "Saved sticker");
    }

    Ok(Some(filename))
}

/// Downloads an image by clicking to open fullscreen, finding the blob URL,
/// downloading via CDP IO, saving to media dir, then closing the viewer.
async fn download_image(
    el: &Element,
    page: &PageSession,
    media_dir: &Path,
    timing: &HumanDelay,
) -> Result<Option<String>> {
    // Click the "Open picture" button to open fullscreen viewer.
    let Some(open_btn) = el
        .try_query_selector(r#"[aria-label="Open picture"]"#)
        .await?
    else {
        return Ok(None);
    };

    open_btn.click(timing).await?;
    tokio::time::sleep(Duration::from_millis(800)).await;

    // Find the fullscreen image with a blob: src.
    // The fullscreen viewer shows the image in a large <img> with crossorigin="anonymous".
    let dom = page.dom().await?;
    let result = async {
        let imgs = dom
            .query_selector_all(r#"img[crossorigin="anonymous"][src^="blob:"]"#)
            .await?;

        for img in &imgs {
            let Some(src) = img.attribute("src").await? else {
                continue;
            };
            if !src.starts_with("blob:") {
                continue;
            }

            let uuid = src.rsplit('/').next().unwrap_or("");
            if uuid.is_empty() {
                continue;
            }

            let bytes = page.read_blob(uuid).await?;
            if bytes.is_empty() {
                continue;
            }

            let hash = Sha256::digest(&bytes);
            let filename = format!("image_{:x}", hash);
            let path = media_dir.join(&filename);
            if !path.exists() {
                std::fs::write(&path, &bytes)?;
                tracing::debug!(filename = %filename, size = bytes.len(), "Saved image");
            }

            return Ok(Some(filename));
        }

        Ok::<_, WhatsappError>(None)
    }
    .await;

    // Close the fullscreen viewer by pressing Escape.
    // Use a fresh DOM query to find a focusable element and press Escape on it.
    if let Some(body) = dom.try_query_selector("body").await? {
        let _ = body.press_key("Escape", timing).await;
    }
    tokio::time::sleep(Duration::from_millis(300)).await;

    result
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_pre_plain_text_basic() {
        let (ts, sender) =
            parse_pre_plain_text("[14:14, 3/30/2026] Guilherme Bernal: ");
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
        let (ts, sender) =
            parse_pre_plain_text("[17:18, 3/19/2026] +55 71 8466-9177: ");
        assert!(ts.is_some());
        assert_eq!(sender.as_deref(), Some("+55 71 8466-9177"));
    }
}
