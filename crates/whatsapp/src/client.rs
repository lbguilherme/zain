use std::path::Path;
use std::time::Duration;

use chromium_driver::{Browser, ChromiumProcess, LaunchOptions, PageSession};

use crate::error::{Result, WhatsappError};
use crate::qr;
use crate::WEB_URL;

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
    /// If a QR code is needed, calls `on_qr` with each new [`QrCode`] as it
    /// rotates (~20s intervals). The callback is called at least once, and again
    /// each time the QR content changes. Keeps polling until the user scans the
    /// QR and the chat list appears.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # async fn example() -> whatsapp::Result<()> {
    /// let client = whatsapp::WhatsAppClient::launch(Default::default()).await?;
    ///
    /// let session = client.authenticate(|qr_data| {
    ///     println!("Scan this QR: {qr_data}");
    /// }).await?;
    ///
    /// session.close().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn authenticate(self, mut on_qr: impl FnMut(&str)) -> Result<WhatsAppSession> {
        let dom = self.page.dom().await?;

        let deadline = tokio::time::Instant::now() + self.auth_timeout;
        let mut last_qr_data: Option<String> = None;

        loop {
            // Check if already authenticated
            if dom.try_query_selector(AUTHENTICATED_SELECTOR).await?.is_some() {
                tokio::time::sleep(self.poll_interval).await;

                return Ok(WhatsAppSession {
                    process: self.process,
                    browser: self.browser,
                    page: self.page,
                });
            }

            // Check for QR code
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
    ///
    /// Creates the `dumps/` directory if it doesn't exist. The HTML output is
    /// beautified (indented) with `<script>` and `<style>` blocks removed
    /// for easier inspection.
    pub async fn dump_dom(&self, name: &str) -> Result<()> {
        let dir = Path::new("dumps");
        std::fs::create_dir_all(dir)?;

        // HTML dump
        let html_path = dir.join(format!("{name}.html"));
        let html = self.page.dom().await?.page_html().await?;
        let clean = beautify_html(&html);
        std::fs::write(&html_path, &clean)?;
        eprintln!("DOM dump saved to {}", html_path.display());

        // PNG screenshot (full visible page)
        let png_path = dir.join(format!("{name}.png"));
        let png_bytes = self.page.capture_screenshot().await?;
        std::fs::write(&png_path, &png_bytes)?;
        eprintln!("Screenshot saved to {}", png_path.display());

        Ok(())
    }

    /// Gracefully closes the browser and terminates the process.
    pub async fn close(mut self) -> Result<()> {
        self.browser.close().await?;
        self.process.wait().await?;
        Ok(())
    }

    /// Retrieves the logged-in user's profile (name, phone, avatar).
    ///
    /// Navigates to the profile screen, extracts the information, and
    /// returns to the chat list. Fails if any expected UI element is missing.
    pub async fn profile(&self) -> Result<crate::types::UserProfile> {
        crate::profile::get_profile(self.page.dom().await?).await
    }

    /// Lists chats from the sidebar, scrolling down until the oldest visible
    /// chat's timestamp is before `since`.
    ///
    /// Pinned chats are always included regardless of their timestamp.
    pub async fn get_chats(
        &self,
        since: chrono::NaiveDate,
    ) -> Result<Vec<crate::types::ChatPreview>> {
        crate::chat::get_chats(&self.page, since).await
    }

    /// Opens a chat by clicking its row in the sidebar (matched by title).
    ///
    /// Waits for the conversation panel to appear before returning.
    pub async fn open_chat(&self, title: &str) -> Result<()> {
        let dom = self.page.dom().await?;
        let timing = crate::timing();
        crate::message::open_chat(dom, &self.page, title, &timing).await
    }

    /// Closes the currently open chat via "..." menu → "Close chat".
    pub async fn close_chat(&self) -> Result<()> {
        let dom = self.page.dom().await?;
        let timing = crate::timing();
        crate::message::close_chat(dom, &timing).await
    }

    /// Reads all visible messages in the currently open chat.
    pub async fn read_messages(
        &self,
        media_dir: &std::path::Path,
    ) -> Result<Vec<crate::types::RawMessage>> {
        let dom = self.page.dom().await?;
        crate::message::read_visible_messages(dom, &self.page, media_dir).await
    }

    /// Scrolls up in the message panel to load older messages.
    pub async fn scroll_up_messages(&self) -> Result<()> {
        let dom = self.page.dom().await?;
        let timing = crate::timing();
        crate::message::scroll_up_messages(dom, &timing).await
    }

    /// Navigates back to the chat list by clicking the "Chats" navbar button.
    pub async fn navigate_to_chats(&self) -> Result<()> {
        let dom = self.page.dom().await?;
        let timing = crate::timing();
        navigate_to_chats(dom, &timing).await
    }
}

const CHATS_NAV_BUTTON: &str = r#"button[aria-label="Chats"]"#;

/// Clicks the "Chats" navbar button to return to the chat list.
pub(crate) async fn navigate_to_chats(dom: &chromium_driver::dom::Dom, timing: &chromium_driver::dom::HumanDelay) -> Result<()> {
    let chats_btn = dom
        .try_query_selector(CHATS_NAV_BUTTON)
        .await?
        .ok_or(WhatsappError::SelectorNotFound(CHATS_NAV_BUTTON))?;
    chats_btn.click(timing).await?;

    dom.wait_for(AUTHENTICATED_SELECTOR, Duration::from_secs(5))
        .await
        .map_err(|_| WhatsappError::Timeout("return to chat list".into()))?;

    Ok(())
}

/// Beautifies raw HTML by indenting tags and removing `<script>`/`<style>` blocks.
fn beautify_html(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut indent: usize = 0;
    let mut pos = 0;
    let bytes = html.as_bytes();

    while pos < bytes.len() {
        if bytes[pos] == b'<' {
            // Find end of tag
            let tag_end = match html[pos..].find('>') {
                Some(i) => pos + i + 1,
                None => break,
            };
            let tag = &html[pos..tag_end];

            // Check if this is a script, style, or link tag
            let tag_lower = tag.to_ascii_lowercase();
            if tag_lower.starts_with("<link") {
                pos = tag_end;
                continue;
            }
            if tag_lower.starts_with("<script") || tag_lower.starts_with("<style") {
                // Skip everything until closing tag
                let close = if tag_lower.starts_with("<script") {
                    "</script>"
                } else {
                    "</style>"
                };
                if let Some(end) = html[tag_end..].to_ascii_lowercase().find(close) {
                    pos = tag_end + end + close.len();
                } else {
                    pos = bytes.len();
                }
                continue;
            }

            let is_close = tag.starts_with("</");
            let is_void = tag.ends_with("/>")
                || is_void_tag(tag);

            if is_close {
                indent = indent.saturating_sub(1);
            }

            write_indent(&mut out, indent);
            out.push_str(tag);
            out.push('\n');

            if !is_close && !is_void {
                indent += 1;
            }

            pos = tag_end;
        } else {
            // Text node — collect until next '<'
            let text_end = html[pos..].find('<').map(|i| pos + i).unwrap_or(bytes.len());
            let text = html[pos..text_end].trim();
            if !text.is_empty() {
                write_indent(&mut out, indent);
                out.push_str(text);
                out.push('\n');
            }
            pos = text_end;
        }
    }

    out
}

fn write_indent(out: &mut String, level: usize) {
    for _ in 0..level {
        out.push_str("  ");
    }
}

fn is_void_tag(tag: &str) -> bool {
    const VOIDS: &[&str] = &[
        "area", "base", "br", "col", "embed", "hr", "img",
        "input", "link", "meta", "source", "track", "wbr",
    ];
    let name = tag
        .trim_start_matches('<')
        .split(|c: char| c.is_whitespace() || c == '>' || c == '/')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    VOIDS.contains(&name.as_str())
}
