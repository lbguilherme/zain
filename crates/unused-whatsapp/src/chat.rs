//! Chat list: listing, opening, closing chats in the sidebar.

use std::collections::HashSet;
use std::time::Duration;

use chromium_driver::JsObject;
use chromium_driver::PageSession;
use chromium_driver::dom::{Dom, Element};
use chrono::{Datelike, Local, NaiveDate, NaiveDateTime, NaiveTime, Weekday};

use crate::error::{Result, WhatsappError};
use crate::types::ChatPreview;

const CHAT_LIST: &str = r#"div[role="grid"][aria-label="Chat list"]"#;

/// Fetches chats from the sidebar, scrolling down until the oldest visible
/// chat timestamp is older than `since`.
///
/// Pinned chats are always included regardless of their timestamp.
pub(crate) async fn get_chats(
    page: &chromium_driver::PageSession,
    since: NaiveDate,
) -> Result<Vec<ChatPreview>> {
    let dom = page.dom().await?;
    let grid = match dom.query_selector(CHAT_LIST).await {
        Ok(g) => g,
        Err(_) => {
            let _ = page.debug_dump("get_chats_no_grid").await;
            return Err(WhatsappError::SelectorNotFound(CHAT_LIST));
        }
    };
    let grid_js = grid.resolve().await?;

    scroll_to_top(&grid, &grid_js).await?;

    let mut seen_titles: HashSet<String> = HashSet::new();
    let mut chats: Vec<ChatPreview> = Vec::new();
    let mut done_with_pinned = false;
    let mut no_new_count = 0;

    loop {
        let extraction: JsExtraction = serde_json::from_value(
            grid_js
                .eval_value(
                    r#"function() {
                        const BIDI = /[\u200E\u200F\u202A-\u202E\u2066-\u2069]/g;
                        const gr = this.getBoundingClientRect();
                        let lastBottom = gr.top;
                        const allRows = this.querySelectorAll('div[role="row"]');
                        const rows = Array.from(allRows).map(row => {
                            const titleEl = row.querySelector('div[aria-colindex="2"] span[title]');
                            if (!titleEl) return null;
                            const title = (titleEl.getAttribute('title') || '').replace(BIDI, '').trim();
                            if (!title) return null;
                            lastBottom = row.getBoundingClientRect().bottom;
                            const msgEl = row.querySelector('div._ak8k span[title]');
                            const lastMessage = msgEl ? (msgEl.getAttribute('title') || '').replace(BIDI, '').trim() : '';
                            const unreadEl = row.querySelector('span[aria-label*="unread message"]');
                            const unreadCount = unreadEl ? parseInt((unreadEl.getAttribute('aria-label') || '')) || 0 : 0;
                            const timeEl = row.querySelector('div._ak8i span');
                            const timeText = timeEl ? (timeEl.textContent || '').trim() || null : null;
                            const isPinned = !!row.querySelector('div[aria-label="Pinned chat"]');
                            return { title, lastMessage, unreadCount, timeText, isPinned };
                        }).filter(r => r !== null);
                        const midY = gr.top + gr.height / 2;
                        const swipeDist = lastBottom > midY ? lastBottom - midY : 0;
                        return {
                            rows,
                            swipeX: gr.left + gr.width / 2,
                            swipeStartY: gr.top + gr.height * 0.7,
                            swipeDist,
                        };
                    }"#,
                )
                .await?,
        )
        .unwrap_or_default();

        let mut found_new = false;
        for row in &extraction.rows {
            if !row.is_pinned {
                done_with_pinned = true;
            }

            if seen_titles.contains(&row.title) {
                continue;
            }
            found_new = true;
            seen_titles.insert(row.title.clone());

            let timestamp = row.time_text.as_deref().and_then(parse_time_text);

            if done_with_pinned
                && !row.is_pinned
                && let Some(ts) = timestamp
                && ts.date() < since
            {
                return Ok(chats);
            }

            chats.push(ChatPreview {
                title: row.title.clone(),
                last_message: row.last_message.clone(),
                unread_count: row.unread_count,
                timestamp,
            });
        }

        if !found_new {
            no_new_count += 1;
            if no_new_count >= 3 {
                break;
            }
        } else {
            no_new_count = 0;
        }

        if extraction.swipe_dist < 10.0 {
            break;
        }
        let end_y = extraction.swipe_start_y - extraction.swipe_dist;
        dom.swipe_vertical(extraction.swipe_x, extraction.swipe_start_y, end_y)
            .await?;
        tokio::time::sleep(Duration::from_millis(300)).await;
    }

    Ok(chats)
}

// ── JS extraction ────────────────────────────────────────────────────────

#[derive(serde::Deserialize, Default)]
struct JsExtraction {
    rows: Vec<JsRowData>,
    #[serde(rename = "swipeX", default)]
    swipe_x: f64,
    #[serde(rename = "swipeStartY", default)]
    swipe_start_y: f64,
    #[serde(rename = "swipeDist", default)]
    swipe_dist: f64,
}

#[derive(serde::Deserialize)]
struct JsRowData {
    title: String,
    #[serde(rename = "lastMessage")]
    last_message: String,
    #[serde(rename = "unreadCount")]
    unread_count: u32,
    #[serde(rename = "timeText")]
    time_text: Option<String>,
    #[serde(rename = "isPinned")]
    is_pinned: bool,
}

// ── Timestamp parsing ────────────────────────────────────────────────────

/// Parses WhatsApp's time text into a NaiveDateTime, picking the most recent
/// instant that matches the description.
///
/// Formats:
/// - `"HH:MM"` → today at that time
/// - `"Yesterday"` → yesterday at 23:59
/// - Weekday name (`"Sunday"`, `"Monday"`, ...) → most recent occurrence, at 23:59
/// - `"M/D/YYYY"` or `"DD/MM/YYYY"` → that date at 23:59
fn parse_time_text(text: &str) -> Option<NaiveDateTime> {
    let text = text.trim();
    let today = Local::now().date_naive();

    if let Some((h, m)) = text.split_once(':') {
        let h: u32 = h.trim().parse().ok()?;
        let m: u32 = m.trim().parse().ok()?;
        let time = NaiveTime::from_hms_opt(h, m, 59)?;
        return Some(today.and_time(time));
    }

    let end_of_day = NaiveTime::from_hms_opt(23, 59, 59).unwrap();

    if text.eq_ignore_ascii_case("yesterday") {
        let yesterday = today - chrono::Duration::days(1);
        return Some(yesterday.and_time(end_of_day));
    }

    if let Some(weekday) = parse_weekday(text) {
        let date = most_recent_weekday(today, weekday);
        return Some(date.and_time(end_of_day));
    }

    if let Some(date) = parse_date_text(text) {
        return Some(date.and_time(end_of_day));
    }

    None
}

fn most_recent_weekday(today: NaiveDate, target: Weekday) -> NaiveDate {
    let today_wd = today.weekday().num_days_from_monday();
    let target_wd = target.num_days_from_monday();
    let days_back = (today_wd + 7 - target_wd) % 7;
    today - chrono::Duration::days(days_back as i64)
}

fn parse_weekday(text: &str) -> Option<Weekday> {
    match text.to_ascii_lowercase().as_str() {
        "monday" => Some(Weekday::Mon),
        "tuesday" => Some(Weekday::Tue),
        "wednesday" => Some(Weekday::Wed),
        "thursday" => Some(Weekday::Thu),
        "friday" => Some(Weekday::Fri),
        "saturday" => Some(Weekday::Sat),
        "sunday" => Some(Weekday::Sun),
        _ => None,
    }
}

fn parse_date_text(text: &str) -> Option<NaiveDate> {
    if let Ok(d) = NaiveDate::parse_from_str(text, "%d/%m/%Y") {
        return Some(d);
    }
    if let Ok(d) = NaiveDate::parse_from_str(text, "%-m/%-d/%Y") {
        return Some(d);
    }
    None
}

// ── Open / close chat ────────────────────────────────────────────────────

/// Opens a chat by clicking on its row in the sidebar.
///
/// Uses JS to find the row and read its position, then CDP for scrolling
/// and clicking.
pub(crate) async fn open_chat(page: &PageSession, title: &str) -> Result<()> {
    let dom = page.dom().await?;
    let escaped_js = title.replace('\\', "\\\\").replace('"', "\\\"");

    let mut clicked = false;
    for attempt in 0..50 {
        // Single JS call: find row, read its position + viewport info.
        let info = page
            .eval_value(&format!(
                r#"(() => {{
                    const grid = document.querySelector('div[role="grid"][aria-label="Chat list"]');
                    if (!grid) return {{ found: false, viewportH: window.innerHeight, sidebarX: 185 }};
                    const gr = grid.getBoundingClientRect();
                    const sidebarX = gr.left + gr.width / 2;
                    const viewportH = window.innerHeight;
                    const span = grid.querySelector('span[title="{}"]');
                    if (!span) return {{ found: false, viewportH, sidebarX }};
                    const cell = span.closest('div[role="gridcell"]') || span;
                    const rect = cell.getBoundingClientRect();
                    return {{ found: true, centerY: rect.top + rect.height / 2, viewportH, sidebarX }};
                }})()"#,
                escaped_js
            ))
            .await?;

        let found = info.get("found").and_then(|v| v.as_bool()).unwrap_or(false);
        let viewport_h = info
            .get("viewportH")
            .and_then(|v| v.as_f64())
            .unwrap_or(900.0);
        let sidebar_x = info
            .get("sidebarX")
            .and_then(|v| v.as_f64())
            .unwrap_or(185.0);

        if !found {
            tracing::info!(title, attempt, "Not found, scrolling down");
            let start_y = viewport_h * 0.70;
            let end_y = viewport_h * 0.20;
            let _ = dom.swipe_vertical(sidebar_x, start_y, end_y).await;
            tokio::time::sleep(Duration::from_millis(300)).await;
            continue;
        }

        let center_y = info.get("centerY").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let safe_bottom = viewport_h * 0.70;

        if center_y > safe_bottom {
            let ideal_y = viewport_h * 0.35;
            let swipe_dist = (center_y - ideal_y).min(400.0).max(100.0);
            let start_y = viewport_h * 0.70;
            let end_y = (start_y - swipe_dist).max(viewport_h * 0.10);
            tracing::info!(
                title,
                attempt,
                center_y,
                safe_bottom,
                swipe_dist,
                "Too low, swiping up"
            );
            let _ = dom.swipe_vertical(sidebar_x, start_y, end_y).await;
            tokio::time::sleep(Duration::from_millis(300)).await;
            continue;
        }

        // Row is in safe zone — find clickable element via CDP and click.
        let Some(row_el) = find_chat_row(dom, title).await? else {
            tokio::time::sleep(Duration::from_millis(200)).await;
            continue;
        };

        match row_el.click().await {
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
        tracing::warn!(title, "Could not find/click chat row");
        let _ = page.debug_dump("open_chat_not_found").await;
        return Err(WhatsappError::SelectorNotFound("chat row by title"));
    }

    if dom
        .wait_for(crate::message::MSG_SCROLL_CONTAINER, Duration::from_secs(5))
        .await
        .is_err()
    {
        let _ = page.debug_dump("open_chat_panel_timeout").await;
        return Err(WhatsappError::Timeout("chat message panel".into()));
    }

    Ok(())
}

/// Finds a chat row's gridcell by title for clicking.
async fn find_chat_row(dom: &Dom, target_title: &str) -> Result<Option<Element>> {
    let escaped = target_title.replace('\\', "\\\\").replace('"', "\\\"");

    let gridcell_selector = format!(
        r#"div[role="grid"][aria-label="Chat list"] div[role="gridcell"]:has(span[title="{}"])"#,
        escaped
    );
    match dom.try_query_selector(&gridcell_selector).await {
        Ok(Some(el)) => return Ok(Some(el)),
        Ok(None) => {}
        Err(_) => {}
    }

    let span_selector = format!(
        r#"div[role="grid"][aria-label="Chat list"] span[title="{}"]"#,
        escaped
    );
    match dom.try_query_selector(&span_selector).await {
        Ok(el) => Ok(el),
        Err(_) => Ok(None),
    }
}

// ── Scroll helpers ───────────────────────────────────────────────────────

/// Scrolls the chat list grid to the top via repeated swipe-down gestures
/// until the first row's top aligns with the container's top.
async fn scroll_to_top(grid: &Element, grid_js: &JsObject) -> Result<()> {
    for _ in 0..30 {
        let at_top = grid_js
            .eval_value(
                r#"function() {
                    const row = this.querySelector('div[role="row"]');
                    if (!row) return true;
                    const gr = this.getBoundingClientRect();
                    const rr = row.getBoundingClientRect();
                    return Math.abs(rr.top - gr.top) < 5;
                }"#,
            )
            .await?;

        if at_top.as_bool().unwrap_or(true) {
            break;
        }

        grid.swipe_down(500.0).await?;
        tokio::time::sleep(Duration::from_millis(300)).await;
    }

    Ok(())
}
