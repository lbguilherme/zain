//! Profile retrieval: navigate to the user's profile and extract info.

use std::time::Duration;

use chromium_driver::PageSession;

use crate::error::{Result, WhatsappError};
use crate::types::UserProfile;

const YOU_BUTTON: &str = r#"button[aria-label="You"]"#;
const MENU_ITEM: &str = r#"button[role="listitem"]"#;
const PROFILE_NAME: &str = r#"span[data-testid="selectable-text"]"#;

/// Opens the user's profile screen, extracts info, and navigates back.
pub(crate) async fn get_profile(page: &PageSession) -> Result<UserProfile> {
    navigate_to_profile(page).await?;

    let _ = page.debug_dump("profile_extract").await;

    let val = page
        .eval_value(
            r#"(() => {
                const nameEl = document.querySelector('span[data-testid="selectable-text"]');
                const name = nameEl ? nameEl.textContent.trim() : null;
                const phoneEl = document.querySelector('div:has(>span[data-icon="phone"]) span:not([data-icon="phone"])');
                const phone = phoneEl ? phoneEl.textContent.trim() : null;
                const avatarEl = document.querySelector('div[aria-label="View group profile picture"] img');
                let avatarUrl = null;
                if (avatarEl && avatarEl.src && avatarEl.src.includes('whatsapp.net')) avatarUrl = avatarEl.src;
                return { name, phone, avatarUrl };
            })()"#,
        )
        .await?;

    let name = val
        .get("name")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or(WhatsappError::SelectorNotFound("profile name"))?
        .to_owned();

    let phone = val
        .get("phone")
        .and_then(|v| v.as_str())
        .filter(|s| s.starts_with('+'))
        .ok_or(WhatsappError::SelectorNotFound("phone number"))?
        .to_owned();

    let avatar_url = val
        .get("avatarUrl")
        .and_then(|v| v.as_str())
        .map(|s| s.to_owned());

    crate::client::navigate_to_chats(page).await?;

    Ok(UserProfile {
        name,
        phone,
        avatar_url,
    })
}

async fn navigate_to_profile(page: &PageSession) -> Result<()> {
    // Ensure we're on the chat list screen first.
    crate::client::navigate_to_chats(page).await?;

    let dom = page.dom().await?;

    let you_btn = dom
        .try_query_selector(YOU_BUTTON)
        .await?
        .ok_or(WhatsappError::SelectorNotFound(YOU_BUTTON))?;
    you_btn.click().await?;

    let _ = page.debug_dump("profile_you_clicked").await;

    if dom
        .wait_for(MENU_ITEM, Duration::from_secs(5))
        .await
        .is_err()
    {
        let _ = page.debug_dump("profile_menu_timeout").await;
        return Err(WhatsappError::SelectorNotFound(MENU_ITEM));
    }

    let _ = page.debug_dump("profile_menu_visible").await;

    // JS: find the "Profile" menu item index.
    let idx = page
        .eval_value(
            r#"(() => {
                const items = document.querySelectorAll('button[role="listitem"]');
                for (let i = 0; i < items.length; i++) {
                    if ((items[i].textContent || '').includes('Profile')) return i;
                }
                return -1;
            })()"#,
        )
        .await?;

    let i = idx.as_i64().unwrap_or(-1);
    if i < 0 {
        return Err(WhatsappError::SelectorNotFound("Profile menu item"));
    }

    let items = dom.query_selector_all(MENU_ITEM).await?;
    let item = items
        .get(i as usize)
        .ok_or(WhatsappError::SelectorNotFound("Profile menu item"))?;
    item.click().await?;

    if dom
        .wait_for(PROFILE_NAME, Duration::from_secs(5))
        .await
        .is_err()
    {
        let _ = page.debug_dump("profile_detail_timeout").await;
        return Err(WhatsappError::Timeout("profile detail panel".into()));
    }

    tokio::time::sleep(Duration::from_millis(500)).await;

    Ok(())
}
