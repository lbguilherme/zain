use serde::Deserialize;

use crate::error::Result;
use crate::session::CdpSession;

/// Return type for [`BrowserCommands::browser_get_version`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetVersionReturn {
    /// Protocol version (e.g. `"1.3"`).
    pub protocol_version: String,
    /// Product name (e.g. `"Chrome/120.0.6099.109"`).
    pub product: String,
    /// Product revision.
    pub revision: String,
    /// User-Agent string.
    pub user_agent: String,
    /// V8 version.
    pub js_version: String,
}

/// `Browser` domain CDP methods.
///
/// Reference: <https://chromedevtools.github.io/devtools-protocol/tot/Browser/>
pub trait BrowserCommands {
    /// Returns version information about the browser, protocol, user-agent and V8.
    ///
    /// CDP: `Browser.getVersion`
    async fn browser_get_version(&self) -> Result<GetVersionReturn>;

    /// Gracefully closes the browser. The process will terminate after this call.
    ///
    /// CDP: `Browser.close`
    async fn browser_close(&self) -> Result<()>;
}

impl BrowserCommands for CdpSession {
    async fn browser_get_version(&self) -> Result<GetVersionReturn> {
        self.call("Browser.getVersion", &serde_json::json!({})).await
    }

    async fn browser_close(&self) -> Result<()> {
        self.call_no_response("Browser.close", &serde_json::json!({}))
            .await
    }
}
