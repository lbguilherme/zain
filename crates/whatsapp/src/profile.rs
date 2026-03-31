//! Profile retrieval: navigate to the user's profile and extract info.

use std::time::Duration;

use chromium_driver::dom::Dom;

use crate::error::{Result, WhatsappError};
use crate::types::UserProfile;

const YOU_BUTTON: &str = r#"button[aria-label="You"]"#;
const MENU_ITEM: &str = r#"button[role="listitem"]"#;
const PROFILE_NAME: &str = r#"span[data-testid="selectable-text"]"#;
const AVATAR_IMG: &str = r#"div[aria-label="View group profile picture"] img"#;

const PHONE_SELECTOR: &str =
    r#"div:has(>span[data-icon="phone"]) span:not([data-icon="phone"])"#;

/// Opens the user's profile screen, extracts info, and navigates back.
pub(crate) async fn get_profile(dom: &Dom) -> Result<UserProfile> {
    let timing = crate::timing();
    navigate_to_profile(dom, &timing).await?;

    let name = extract_name(dom).await?;
    let phone = extract_phone(dom).await?;
    let avatar_url = extract_avatar_url(dom).await;

    crate::client::navigate_to_chats(dom, &timing).await?;

    Ok(UserProfile {
        name,
        phone,
        avatar_url,
    })
}

async fn navigate_to_profile(dom: &Dom, timing: &chromium_driver::dom::HumanDelay) -> Result<()> {
    let you_btn = dom
        .try_query_selector(YOU_BUTTON)
        .await?
        .ok_or(WhatsappError::SelectorNotFound(YOU_BUTTON))?;
    you_btn.click(timing).await?;

    dom.wait_for(MENU_ITEM, Duration::from_secs(3))
        .await
        .map_err(|_| WhatsappError::SelectorNotFound(MENU_ITEM))?;

    let items = dom.query_selector_all(MENU_ITEM).await?;
    for item in &items {
        let text = item.text().await.unwrap_or_default();
        if text.contains("Profile") {
            item.click(timing).await?;
            dom.wait_for(PROFILE_NAME, Duration::from_secs(5))
                .await
                .map_err(|_| WhatsappError::Timeout("profile detail panel".into()))?;
            tokio::time::sleep(Duration::from_millis(500)).await;
            return Ok(());
        }
    }

    Err(WhatsappError::SelectorNotFound("Profile menu item"))
}

async fn extract_name(dom: &Dom) -> Result<String> {
    let el = dom
        .try_query_selector(PROFILE_NAME)
        .await?
        .ok_or(WhatsappError::SelectorNotFound(PROFILE_NAME))?;
    let name = el.text().await?.trim().to_owned();
    if name.is_empty() {
        return Err(WhatsappError::SelectorNotFound("profile name text"));
    }
    Ok(name)
}

async fn extract_phone(dom: &Dom) -> Result<String> {
    let el = dom
        .try_query_selector(PHONE_SELECTOR)
        .await?
        .ok_or(WhatsappError::SelectorNotFound(PHONE_SELECTOR))?;
    let phone = el.text().await?.trim().to_owned();
    if phone.is_empty() || !phone.starts_with('+') {
        return Err(WhatsappError::SelectorNotFound("phone number text"));
    }
    Ok(phone)
}

async fn extract_avatar_url(dom: &Dom) -> Option<String> {
    let img = dom.try_query_selector(AVATAR_IMG).await.ok()??;
    let src = img.attribute("src").await.ok()??;
    if src.contains("whatsapp.net") {
        Some(src)
    } else {
        None
    }
}

