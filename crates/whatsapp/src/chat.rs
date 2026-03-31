//! Chat list retrieval: scroll through the sidebar and extract chat previews.

use std::collections::HashSet;
use std::time::Duration;

use chrono::{Datelike, Local, NaiveDate, NaiveDateTime, NaiveTime, Weekday};
use chromium_driver::dom::Element;

use crate::error::{Result, WhatsappError};
use crate::types::ChatPreview;

const CHAT_LIST: &str = r#"div[role="grid"][aria-label="Chat list"]"#;
const CHAT_ROW: &str = r#"div[role="row"]"#;
const PINNED_INDICATOR: &str = r#"div[aria-label="Pinned chat"]"#;

/// Fetches chats from the sidebar, scrolling down until the oldest visible
/// chat timestamp is older than `since`.
///
/// Pinned chats are always included regardless of their timestamp.
pub(crate) async fn get_chats(
    page: &chromium_driver::PageSession,
    since: NaiveDate,
) -> Result<Vec<ChatPreview>> {
    let timing = crate::timing();
    let dom = page.dom().await?;
    let grid = dom
        .query_selector(CHAT_LIST)
        .await
        .map_err(|_| WhatsappError::SelectorNotFound(CHAT_LIST))?;

    // Scroll chat list to the top before iterating.
    scroll_to_top(&grid, &timing).await?;

    let mut seen_titles: HashSet<String> = HashSet::new();
    let mut chats: Vec<ChatPreview> = Vec::new();
    let mut done_with_pinned = false;
    let mut no_new_count = 0;

    loop {
        let rows = grid.query_selector_all(CHAT_ROW).await?;

        let mut found_new = false;
        for row in &rows {
            let Some(preview) = parse_row(row).await? else {
                continue;
            };

            let is_pinned = row.try_query_selector(PINNED_INDICATOR).await?.is_some();

            if !is_pinned {
                done_with_pinned = true;
            }

            if seen_titles.contains(&preview.title) {
                continue;
            }
            found_new = true;
            seen_titles.insert(preview.title.clone());

            // For non-pinned chats, check if we've gone past our date cutoff.
            if done_with_pinned && !is_pinned {
                if let Some(ts) = preview.timestamp {
                    if ts.date() < since {
                        return Ok(chats);
                    }
                }
            }

            chats.push(preview);
        }

        if !found_new {
            no_new_count += 1;
            if no_new_count >= 3 {
                break;
            }
        } else {
            no_new_count = 0;
        }

        // Scroll down the chat list to load more rows.
        grid.swipe_up(500.0, &timing).await?;
        tokio::time::sleep(Duration::from_millis(300)).await;
    }

    Ok(chats)
}

async fn parse_row(row: &Element) -> Result<Option<ChatPreview>> {
    // Title: the span with a `title` attr inside the first gridcell column 2.
    let Some(title_el) = row
        .try_query_selector(r#"div[aria-colindex="2"] span[title]"#)
        .await?
    else {
        return Ok(None);
    };

    let title = strip_bidi(
        &title_el
            .attribute("title")
            .await?
            .unwrap_or_default(),
    );
    if title.is_empty() {
        return Ok(None);
    }

    // Last message preview: span[title] inside the message preview area.
    let last_message = if let Some(msg_el) = row
        .try_query_selector(r#"div._ak8k span[title]"#)
        .await?
    {
        strip_bidi(
            &msg_el
                .attribute("title")
                .await?
                .unwrap_or_default(),
        )
    } else {
        String::new()
    };

    // Unread count from aria-label like "1 unread message" or "3 unread messages".
    let unread_count = if let Some(unread_el) = row
        .try_query_selector(r#"span[aria-label*="unread message"]"#)
        .await?
    {
        let label = unread_el
            .attribute("aria-label")
            .await?
            .unwrap_or_default();
        parse_unread_count(&label)
    } else {
        0
    };

    // Time text (e.g. "07:09", "Yesterday", "Sunday", "3/25/2026").
    let time_raw = if let Some(time_el) = row
        .try_query_selector(r#"div._ak8i span"#)
        .await?
    {
        let t = time_el.text().await.unwrap_or_default().trim().to_owned();
        if t.is_empty() { None } else { Some(t) }
    } else {
        None
    };

    let timestamp = time_raw.as_deref().and_then(parse_time_text);

    Ok(Some(ChatPreview {
        title,
        last_message,
        unread_count,
        timestamp,
    }))
}

/// Parses WhatsApp's time text into a NaiveDateTime, picking the most recent
/// instant that matches the description.
///
/// Formats:
/// - `"HH:MM"` → today at that time
/// - `"Yesterday"` / `"Ontem"` → yesterday at 23:59
/// - Weekday name (`"Sunday"`, `"Monday"`, ...) → most recent occurrence, at 23:59
/// - `"M/D/YYYY"` or `"DD/MM/YYYY"` → that date at 23:59
fn parse_time_text(text: &str) -> Option<NaiveDateTime> {
    let text = text.trim();
    let today = Local::now().date_naive();

    // "HH:MM" — today at that exact time
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

    // Weekday name — most recent past occurrence (or today)
    if let Some(weekday) = parse_weekday(text) {
        let date = most_recent_weekday(today, weekday);
        return Some(date.and_time(end_of_day));
    }

    // Explicit date
    if let Some(date) = parse_date_text(text) {
        return Some(date.and_time(end_of_day));
    }

    None
}

/// Returns the most recent date with the given weekday, no later than `today`.
/// If `today` is that weekday, returns `today`.
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
    // DD/MM/YYYY (pt-BR)
    if let Ok(d) = NaiveDate::parse_from_str(text, "%d/%m/%Y") {
        return Some(d);
    }
    // M/D/YYYY (en-US)
    if let Ok(d) = NaiveDate::parse_from_str(text, "%-m/%-d/%Y") {
        return Some(d);
    }
    None
}

fn parse_unread_count(label: &str) -> u32 {
    label
        .split_whitespace()
        .next()
        .and_then(|n| n.parse().ok())
        .unwrap_or(0)
}

/// Scrolls the chat list grid to the top via repeated swipe-down gestures
/// until the first visible row stops changing.
async fn scroll_to_top(grid: &Element, timing: &chromium_driver::dom::HumanDelay) -> Result<()> {
    let mut last_top_title: Option<String> = None;

    for _ in 0..30 {
        let rows = grid.query_selector_all(CHAT_ROW).await?;
        let current_top = if let Some(first) = rows.first() {
            if let Some(el) = first
                .try_query_selector(r#"div[aria-colindex="2"] span[title]"#)
                .await?
            {
                el.attribute("title").await?.unwrap_or_default()
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        if last_top_title.as_deref() == Some(&current_top) {
            break;
        }
        last_top_title = Some(current_top);

        grid.swipe_down(500.0, timing).await?;
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    }

    Ok(())
}

/// Strips Unicode bidi control characters and trims whitespace.
fn strip_bidi(s: &str) -> String {
    s.chars()
        .filter(|c| !matches!(c,
            '\u{200E}' | '\u{200F}' |  // LRM, RLM
            '\u{202A}' | '\u{202B}' |  // LRE, RLE
            '\u{202C}' | '\u{202D}' |  // PDF, LRO
            '\u{202E}' |               // RLO
            '\u{2066}' | '\u{2067}' |  // LRI, RLI
            '\u{2068}' | '\u{2069}'    // FSI, PDI
        ))
        .collect::<String>()
        .trim()
        .to_owned()
}
