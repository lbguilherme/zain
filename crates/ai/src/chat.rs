use serde::{Deserialize, Serialize};
use serde_json::Value;

// ── Request types ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub function: ToolCallFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallFunction {
    pub name: String,
    pub arguments: Value,
}

// ── Response types ─────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ChatResponse {
    pub message: ChatResponseMessage,
}

#[derive(Debug, Deserialize)]
pub struct ChatResponseMessage {
    pub role: String,
    #[serde(default)]
    pub content: String,
    pub tool_calls: Option<Vec<ToolCall>>,
}

// ── ChatMessage constructors ───────────────────────────────────────────

impl ChatMessage {
    pub fn system(content: String) -> Self {
        Self {
            role: "system".into(),
            content,
            tool_calls: None,
        }
    }

    pub fn user(content: String) -> Self {
        Self {
            role: "user".into(),
            content,
            tool_calls: None,
        }
    }

    pub fn tool(content: String) -> Self {
        Self {
            role: "tool".into(),
            content,
            tool_calls: None,
        }
    }

    pub fn assistant(content: String) -> Self {
        Self {
            role: "assistant".into(),
            content,
            tool_calls: None,
        }
    }

    pub fn assistant_tool_calls(calls: &[ToolCall]) -> Self {
        Self {
            role: "assistant".into(),
            content: String::new(),
            tool_calls: Some(calls.to_vec()),
        }
    }
}
