use serde::Deserialize;

use crate::error::Result;
use crate::session::CdpSession;

// ── Types ───────────────────────────────────────────────────────────────────

/// Description of the protocol domain.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Domain {
    /// Domain name.
    pub name: String,
    /// Domain version.
    pub version: String,
}

// ── Return types ────────────────────────────────────────────────────────────

/// Return type for [`SchemaCommands::schema_get_domains`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetDomainsReturn {
    /// List of supported domains.
    pub domains: Vec<Domain>,
}

// ── Domain trait ─────────────────────────────────────────────────────────────

/// `Schema` domain CDP methods.
///
/// Reference: <https://chromedevtools.github.io/devtools-protocol/tot/Schema/>
pub trait SchemaCommands {
    /// Returns supported domains.
    ///
    /// CDP: `Schema.getDomains`
    async fn schema_get_domains(&self) -> Result<GetDomainsReturn>;
}

// ── Impl ────────────────────────────────────────────────────────────────────

impl SchemaCommands for CdpSession {
    async fn schema_get_domains(&self) -> Result<GetDomainsReturn> {
        self.call("Schema.getDomains", &serde_json::json!({})).await
    }
}
