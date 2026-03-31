use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TargetId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FrameId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BrowserContextId(pub String);

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MonotonicTime(pub f64);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetInfo {
    pub target_id: TargetId,
    #[serde(rename = "type")]
    pub target_type: String,
    pub title: String,
    pub url: String,
    pub attached: bool,
    #[serde(default)]
    pub opener_id: Option<TargetId>,
    #[serde(default)]
    pub browser_context_id: Option<BrowserContextId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NavigationEntry {
    pub id: i64,
    pub url: String,
    #[serde(default)]
    pub user_typed_url: Option<String>,
    pub title: String,
    #[serde(default)]
    pub transition_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Frame {
    pub id: FrameId,
    #[serde(default)]
    pub parent_id: Option<FrameId>,
    pub url: String,
    #[serde(default)]
    pub security_origin: Option<String>,
    #[serde(default)]
    pub mime_type: Option<String>,
}
