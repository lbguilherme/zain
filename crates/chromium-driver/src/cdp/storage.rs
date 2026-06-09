use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::session::CdpSession;
use crate::types::BrowserContextId;

// ── Types ───────────────────────────────────────────────────────────────────

/// A browser cookie.
///
/// The common fields are typed; every other field (`expires`, `httpOnly`,
/// `secure`, `sameSite`, `size`, `session`, …) is preserved verbatim in
/// [`extra`](Self::extra). This makes a cookie read via `getCookies`
/// round-trip losslessly back through `setCookies` — important when a server
/// (e.g. gov.br) ties a "trusted device" token to the exact cookie attributes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cookie {
    /// Cookie name.
    pub name: String,
    /// Cookie value.
    pub value: String,
    /// Cookie domain.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    /// Cookie path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// All remaining cookie fields, preserved as-is for lossless round-trips.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

// ── Param types ─────────────────────────────────────────────────────────────

/// Parameters for [`StorageCommands::storage_get_cookies`] and
/// [`StorageCommands::storage_clear_cookies`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CookieScopeParams {
    /// Browser context to use when called for an isolated context.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser_context_id: Option<BrowserContextId>,
}

/// Parameters for [`StorageCommands::storage_set_cookies`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetCookiesParams {
    /// Cookies to be set.
    pub cookies: Vec<Cookie>,
    /// Browser context to use when called for an isolated context.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser_context_id: Option<BrowserContextId>,
}

// ── Return types ────────────────────────────────────────────────────────────

/// Return type for [`StorageCommands::storage_get_cookies`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetCookiesReturn {
    /// Array of cookie objects.
    pub cookies: Vec<Cookie>,
}

// ── Domain trait ─────────────────────────────────────────────────────────────

/// `Storage` domain CDP methods (cookie subset).
///
/// Reference: <https://chromedevtools.github.io/devtools-protocol/tot/Storage/>
pub trait StorageCommands {
    /// Returns all browser cookies.
    ///
    /// CDP: `Storage.getCookies`
    async fn storage_get_cookies(&self, params: &CookieScopeParams) -> Result<GetCookiesReturn>;

    /// Sets given cookies.
    ///
    /// CDP: `Storage.setCookies`
    async fn storage_set_cookies(&self, params: &SetCookiesParams) -> Result<()>;

    /// Clears cookies.
    ///
    /// CDP: `Storage.clearCookies`
    async fn storage_clear_cookies(&self, params: &CookieScopeParams) -> Result<()>;
}

// ── Impl ────────────────────────────────────────────────────────────────────

impl StorageCommands for CdpSession {
    async fn storage_get_cookies(&self, params: &CookieScopeParams) -> Result<GetCookiesReturn> {
        self.call("Storage.getCookies", params).await
    }

    async fn storage_set_cookies(&self, params: &SetCookiesParams) -> Result<()> {
        self.call_no_response("Storage.setCookies", params).await
    }

    async fn storage_clear_cookies(&self, params: &CookieScopeParams) -> Result<()> {
        self.call_no_response("Storage.clearCookies", params).await
    }
}
