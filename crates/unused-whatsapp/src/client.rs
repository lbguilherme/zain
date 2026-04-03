use std::time::Duration;

use chromium_driver::{Browser, ChromiumProcess, LaunchOptions, PageSession};

use crate::WEB_URL;
use crate::error::{Result, WhatsappError};
use crate::qr;

pub(crate) const AUTHENTICATED_SELECTOR: &str = r#"div[aria-label="Chat list"]"#;

/// Options for launching a WhatsApp Web client.
pub struct WhatsAppOptions {
    /// Chromium launch options. Set `user_data_dir` for session persistence.
    pub launch: LaunchOptions,
    /// Maximum time to wait for the initial page load event.
    pub page_load_timeout: Duration,
    /// Total time budget for the authentication flow (QR display + scan).
    pub auth_timeout: Duration,
    /// How often to poll the DOM for state changes.
    pub poll_interval: Duration,
}

impl Default for WhatsAppOptions {
    fn default() -> Self {
        Self {
            launch: LaunchOptions::default(),
            page_load_timeout: Duration::from_secs(30),
            auth_timeout: Duration::from_secs(120),
            poll_interval: Duration::from_secs(1),
        }
    }
}

/// WhatsApp Web client backed by a real Chromium instance.
///
/// Created via [`WhatsAppClient::launch`]. Owns the browser process and
/// a page session navigated to `web.whatsapp.com`.
pub struct WhatsAppClient {
    process: ChromiumProcess,
    browser: Browser,
    page: PageSession,
    auth_timeout: Duration,
    poll_interval: Duration,
}

impl WhatsAppClient {
    /// Launches a Chromium instance and navigates to WhatsApp Web.
    pub async fn launch(opts: WhatsAppOptions) -> Result<Self> {
        let (process, browser) = chromium_driver::launch(opts.launch).await?;

        let target = browser.create_page("about:blank").await?;
        let page = target.attach().await?;
        page.enable().await?;
        page.navigate(WEB_URL).await?;
        page.wait_for_load(opts.page_load_timeout).await?;

        Ok(Self {
            process,
            browser,
            page,
            auth_timeout: opts.auth_timeout,
            poll_interval: opts.poll_interval,
        })
    }

    /// Authenticates the WhatsApp Web session.
    ///
    /// If the session is already authenticated (persisted in `user_data_dir`),
    /// returns a [`WhatsAppSession`] immediately without calling `on_qr`.
    ///
    /// If a QR code is needed, calls `on_qr` with each new QR data as it
    /// rotates (~20s intervals). Keeps polling until the user scans the
    /// QR and the chat list appears.
    pub async fn authenticate(self, mut on_qr: impl FnMut(&str)) -> Result<WhatsAppSession> {
        let dom = self.page.dom().await?;

        let deadline = tokio::time::Instant::now() + self.auth_timeout;
        let mut last_qr_data: Option<String> = None;

        loop {
            if dom
                .try_query_selector(AUTHENTICATED_SELECTOR)
                .await?
                .is_some()
            {
                return Ok(WhatsAppSession {
                    process: self.process,
                    browser: self.browser,
                    page: self.page,
                });
            }

            if let Some(el) = qr::find_qr_element(dom).await?
                && let Ok(data) = qr::extract_from_element(&el).await
            {
                let is_new = last_qr_data.as_ref() != Some(&data);
                if is_new {
                    on_qr(&data);
                    last_qr_data = Some(data);
                }
            }

            if tokio::time::Instant::now() >= deadline {
                return Err(WhatsappError::Timeout("authentication".into()));
            }

            tokio::time::sleep(self.poll_interval).await;
        }
    }
}

/// Authenticated WhatsApp Web session.
///
/// Obtained from [`WhatsAppClient::authenticate`]. Owns the browser process
/// and has an active, logged-in page session with the DOM domain already enabled.
pub struct WhatsAppSession {
    process: ChromiumProcess,
    browser: Browser,
    page: PageSession,
}

impl WhatsAppSession {
    /// Access the underlying page session for raw CDP operations.
    pub fn page(&self) -> &PageSession {
        &self.page
    }

    /// Access the browser instance.
    pub fn browser(&self) -> &Browser {
        &self.browser
    }

    /// Dumps the current page HTML and a PNG screenshot to `dumps/{name}.html`
    /// and `dumps/{name}.png`.
    pub async fn dump_dom(&self, name: &str) -> Result<()> {
        self.page.debug_dump(name).await?;
        Ok(())
    }

    /// Gracefully closes the browser and terminates the process.
    pub async fn close(mut self) -> Result<()> {
        self.browser.close().await?;
        self.process.wait().await?;
        Ok(())
    }

    /// Retrieves the logged-in user's profile (name, phone, avatar).
    pub async fn profile(&self) -> Result<crate::types::UserProfile> {
        crate::profile::get_profile(&self.page).await
    }

    /// Lists chats from the sidebar, scrolling down until the oldest visible
    /// chat's timestamp is before `since`.
    pub async fn get_chats(
        &self,
        since: chrono::NaiveDate,
    ) -> Result<Vec<crate::types::ChatPreview>> {
        crate::chat::get_chats(&self.page, since).await
    }

    /// Opens a chat by clicking its row in the sidebar (matched by title).
    pub async fn open_chat(&self, title: &str) -> Result<()> {
        crate::chat::open_chat(&self.page, title).await
    }

    /// Reads all visible messages in the currently open chat.
    pub async fn read_messages(
        &self,
        media_dir: &std::path::Path,
    ) -> Result<Vec<crate::types::RawMessage>> {
        crate::message::read_visible_messages(&self.page, media_dir).await
    }

    /// Scrolls the message panel to the bottom (most recent messages).
    pub async fn scroll_to_bottom(&self) -> Result<()> {
        crate::message::scroll_to_bottom(&self.page).await
    }

    /// Scrolls up in the message panel to load older messages.
    pub async fn scroll_up_messages(&self) -> Result<()> {
        crate::message::scroll_up_messages(&self.page).await
    }

    /// Navigates back to the chat list by clicking the "Chats" navbar button.
    pub async fn navigate_to_chats(&self) -> Result<()> {
        navigate_to_chats(&self.page).await
    }
}

const CHATS_NAV_BUTTON: &str = r#"button[aria-label="Chats"]"#;

/// Clicks the "Chats" navbar button to return to the chat list.
pub(crate) async fn navigate_to_chats(page: &PageSession) -> Result<()> {
    let dom = page.dom().await?;
    let Some(chats_btn) = dom.try_query_selector(CHATS_NAV_BUTTON).await? else {
        let _ = page.debug_dump("nav_chats_no_button").await;
        return Err(WhatsappError::SelectorNotFound(CHATS_NAV_BUTTON));
    };
    chats_btn.click().await?;

    if dom
        .wait_for(AUTHENTICATED_SELECTOR, Duration::from_secs(5))
        .await
        .is_err()
    {
        let _ = page.debug_dump("nav_chats_timeout").await;
        return Err(WhatsappError::Timeout("return to chat list".into()));
    }

    Ok(())
}
