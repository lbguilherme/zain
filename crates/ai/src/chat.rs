use serde::{Deserialize, Serialize};
use serde_json::Value;

// ── Request types ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    /// Nome da tool a que este message responde (role="tool"). Usado pelo
    /// provider Gemini para montar o `functionResponse.name`. É ignorado
    /// na serialização para providers que não precisam do campo.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub function: ToolCallFunction,
    /// Assinatura opaca devolvida pelo Gemini 3.x junto com o `functionCall`.
    /// DEVE ser reemitida no mesmo Part no turno seguinte, senão o Gemini
    /// rejeita com 400 INVALID_ARGUMENT. Outros providers (Ollama etc.)
    /// ignoram o campo — `skip_serializing_if` impede vazamento no wire.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub thought_signature: Option<String>,
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
            tool_name: None,
        }
    }

    pub fn user(content: String) -> Self {
        Self {
            role: "user".into(),
            content,
            tool_calls: None,
            tool_name: None,
        }
    }

    pub fn tool(name: String, content: String) -> Self {
        Self {
            role: "tool".into(),
            content,
            tool_calls: None,
            tool_name: Some(name),
        }
    }

    pub fn assistant(content: String) -> Self {
        Self {
            role: "assistant".into(),
            content,
            tool_calls: None,
            tool_name: None,
        }
    }

    pub fn assistant_tool_calls(calls: &[ToolCall]) -> Self {
        Self {
            role: "assistant".into(),
            content: String::new(),
            tool_calls: Some(calls.to_vec()),
            tool_name: None,
        }
    }
}
