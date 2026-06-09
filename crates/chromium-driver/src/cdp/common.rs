use serde::{Deserialize, Serialize};

use crate::cdp::runtime::ScriptId;
use crate::cdp::runtime::UniqueDebuggerId;

// ── Types ────────────────────────────────────────────────────────────────────

/// Search match for resource.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchMatch {
    /// Line number in resource content.
    pub line_number: f64,
    /// Line with match content.
    pub line_content: String,
}

/// Encapsulates the script ancestry and the root script filter list rule that
/// caused the resource or element to be labeled as an ad.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdAncestry {
    /// A chain of `AdScriptIdentifier`s representing the ancestry of an ad
    /// script that led to the creation of a resource or element. The chain is
    /// ordered from the script itself (lowest level) up to its root ancestor
    /// that was flagged by a filter list.
    pub ancestry_chain: Vec<AdScriptIdentifier>,
    /// The filter list rule that caused the root (last) script in
    /// `ancestryChain` to be tagged as an ad.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_script_filterlist_rule: Option<String>,
}

/// Represents the provenance of an ad resource or element. Only one of
/// `filterlistRule` or `adScriptAncestry` can be set. If `filterlistRule`
/// is provided, the resource URL directly matches a filter list rule. If
/// `adScriptAncestry` is provided, an ad script initiated the resource fetch or
/// appended the element to the DOM. If neither is provided, the entity is
/// known to be an ad, but provenance tracking information is unavailable.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdProvenance {
    /// The filterlist rule that matched, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filterlist_rule: Option<String>,
    /// The script ancestry that created the ad, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ad_script_ancestry: Option<AdAncestry>,
}

/// Identifies the script on the stack that caused a resource or element to be
/// labeled as an ad. For resources, this indicates the context that triggered
/// the fetch. For elements, this indicates the context that caused the element
/// to be appended to the DOM.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdScriptIdentifier {
    /// The script's V8 identifier.
    pub script_id: ScriptId,
    /// V8's debugging ID for the v8::Context.
    pub debugger_id: UniqueDebuggerId,
    /// The script's url (or generated name based on id if inline script).
    pub name: String,
}

/// Cookie object.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cookie {
    /// Cookie name.
    pub name: String,
    /// Cookie value.
    pub value: String,
    /// Cookie domain.
    pub domain: String,
    /// Cookie path.
    pub path: String,
    /// Cookie expiration date as the number of seconds since the UNIX epoch.
    /// The value is set to -1 if the expiry date is not set.
    /// The value can be null for values that cannot be represented in
    /// JSON (±Inf).
    pub expires: f64,
    /// Cookie size.
    pub size: i64,
    /// True if cookie is http-only.
    pub http_only: bool,
    /// True if cookie is secure.
    pub secure: bool,
    /// True in case of session cookie.
    pub session: bool,
    /// Cookie SameSite type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub same_site: Option<CookieSameSite>,
    /// Cookie Priority.
    pub priority: CookiePriority,
    /// Cookie source scheme type.
    pub source_scheme: CookieSourceScheme,
    /// Cookie source port. Valid values are {-1, [1, 65535]}, -1 indicates an unspecified port.
    /// An unspecified port value allows protocol clients to emulate legacy cookie scope for the port.
    /// This is a temporary ability and it will be removed in the future.
    pub source_port: i64,
    /// Cookie partition key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub partition_key: Option<CookiePartitionKey>,
    /// True if cookie partition key is opaque.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub partition_key_opaque: Option<bool>,
}

/// Cookie parameter object.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CookieParam {
    /// Cookie name.
    pub name: String,
    /// Cookie value.
    pub value: String,
    /// The request-URI to associate with the setting of the cookie. This value can affect the
    /// default domain, path, source port, and source scheme values of the created cookie.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Cookie domain.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    /// Cookie path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// True if cookie is secure.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secure: Option<bool>,
    /// True if cookie is http-only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub http_only: Option<bool>,
    /// Cookie SameSite type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub same_site: Option<CookieSameSite>,
    /// Cookie expiration date, session cookie if not set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires: Option<TimeSinceEpoch>,
    /// Cookie Priority.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<CookiePriority>,
    /// Cookie source scheme type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_scheme: Option<CookieSourceScheme>,
    /// Cookie source port. Valid values are {-1, [1, 65535]}, -1 indicates an unspecified port.
    /// An unspecified port value allows protocol clients to emulate legacy cookie scope for the port.
    /// This is a temporary ability and it will be removed in the future.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_port: Option<i64>,
    /// Cookie partition key. If not set, the cookie will be set as not partitioned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub partition_key: Option<CookiePartitionKey>,
}

/// cookiePartitionKey object
/// The representation of the components of the key that are created by the cookiePartitionKey class contained in net/cookies/cookie_partition_key.h.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CookiePartitionKey {
    /// The site of the top-level URL the browser was visiting at the start
    /// of the request to the endpoint that set the cookie.
    pub top_level_site: String,
    /// Indicates if the cookie has any ancestors that are cross-site to the topLevelSite.
    pub has_cross_site_ancestor: bool,
}

/// Represents the cookie's 'Priority' status:
/// https://tools.ietf.org/html/draft-west-cookie-priority-00.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CookiePriority {
    #[default]
    Low,
    Medium,
    High,
}

/// Represents the cookie's 'SameSite' status:
/// https://tools.ietf.org/html/draft-west-first-party-cookies.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CookieSameSite {
    #[default]
    Strict,
    Lax,
    None,
}

/// Represents the source scheme of the origin that originally set the cookie.
/// A value of "Unset" allows protocol clients to emulate legacy cookie scope for the scheme.
/// This is a temporary ability and it will be removed in the future.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CookieSourceScheme {
    #[default]
    Unset,
    NonSecure,
    Secure,
}

/// Unique loader identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LoaderId(pub String);

/// Monotonically increasing time in seconds since an arbitrary point in the past.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MonotonicTime(pub f64);

/// Unique network request identifier.
/// Note that this does not identify individual HTTP requests that are part of
/// a network request.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RequestId(pub String);

/// Resource type as it was perceived by the rendering engine.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceType {
    #[default]
    Document,
    Stylesheet,
    Image,
    Media,
    Font,
    Script,
    TextTrack,
    XHR,
    Fetch,
    Prefetch,
    EventSource,
    WebSocket,
    Manifest,
    SignedExchange,
    Ping,
    CSPViolationReport,
    Preflight,
    FedCM,
    Other,
}

/// UTC time in seconds, counted from January 1, 1970.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TimeSinceEpoch(pub f64);
