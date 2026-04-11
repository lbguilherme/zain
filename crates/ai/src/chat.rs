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
    /// Imagens anexadas à mensagem. Vive fora do `content` porque cada
    /// provider codifica anexos de forma diferente (Ollama espera base64
    /// no campo `images`; Gemini espera `inlineData` parts). Por isso é
    /// `skip_serializing` — cada provider reconstrói o wire format.
    #[serde(skip, default)]
    pub images: Vec<ChatImage>,
}

/// Imagem anexada a uma [`ChatMessage`]. Os bytes são crus — cada provider
/// faz a codificação necessária (base64 para Ollama/Gemini) no momento do
/// envio.
#[derive(Debug, Clone)]
pub struct ChatImage {
    pub bytes: Vec<u8>,
    pub mime_type: String,
}

impl ChatImage {
    pub fn new(bytes: Vec<u8>, mime_type: impl Into<String>) -> Self {
        Self {
            bytes,
            mime_type: mime_type.into(),
        }
    }
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
    #[serde(default)]
    pub usage: ChatUsage,
}

#[derive(Debug, Deserialize)]
pub struct ChatResponseMessage {
    pub role: String,
    #[serde(default)]
    pub content: String,
    pub tool_calls: Option<Vec<ToolCall>>,
}

/// Contabilidade de tokens e custo estimado de uma resposta. O custo
/// é sempre em USD; providers sem pricing conhecido (ex.: Ollama local)
/// devolvem `cost = 0.0`.
#[derive(Debug, Default, Clone, Deserialize)]
pub struct ChatUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cost: f64,
}

// ── ChatMessage constructors ───────────────────────────────────────────

impl ChatMessage {
    pub fn system(content: String) -> Self {
        Self {
            role: "system".into(),
            content,
            tool_calls: None,
            tool_name: None,
            images: Vec::new(),
        }
    }

    pub fn user(content: String) -> Self {
        Self {
            role: "user".into(),
            content,
            tool_calls: None,
            tool_name: None,
            images: Vec::new(),
        }
    }

    /// Mensagem `user` carregando uma ou mais imagens além do texto. O
    /// `content` pode ser vazio se só as imagens importarem.
    pub fn user_with_images(content: String, images: Vec<ChatImage>) -> Self {
        Self {
            role: "user".into(),
            content,
            tool_calls: None,
            tool_name: None,
            images,
        }
    }

    pub fn tool(name: String, content: String) -> Self {
        Self {
            role: "tool".into(),
            content,
            tool_calls: None,
            tool_name: Some(name),
            images: Vec::new(),
        }
    }

    pub fn assistant(content: String) -> Self {
        Self {
            role: "assistant".into(),
            content,
            tool_calls: None,
            tool_name: None,
            images: Vec::new(),
        }
    }

    pub fn assistant_tool_calls(calls: &[ToolCall]) -> Self {
        Self {
            role: "assistant".into(),
            content: String::new(),
            tool_calls: Some(calls.to_vec()),
            tool_name: None,
            images: Vec::new(),
        }
    }
}
