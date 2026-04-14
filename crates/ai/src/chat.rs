use serde::{Deserialize, Serialize};
use serde_json::Value;

// ── Mensagem ───────────────────────────────────────────────────────────

/// Uma única "linha" da conversa.
///
/// Variantes conceitualmente do mesmo turno (texto + imagem de input,
/// texto + tool calls de output) ficam adjacentes na `Vec<ChatMessage>`;
/// cada provider funde as adjacências do mesmo lado num único
/// content/message no seu tradutor.
///
/// O system prompt vai como argumento separado de [`ChatRequest`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatMessage {
    InputText {
        text: String,
    },
    /// Imagem anexada ao turno de input. Os bytes são crus — a
    /// codificação necessária para o wire format acontece no momento
    /// do envio.
    ///
    /// Para correlacionar a imagem com menções no texto, empurre uma
    /// `InputText` com o label imediatamente antes da `InputImage` —
    /// os tradutores agrupam adjacências do mesmo lado num único
    /// content/parts, então a ordem é preservada no wire.
    ///
    /// `bytes` é marcado `serde(skip)` porque serializar imagens em
    /// base64 num log JSON é desperdício; quem precisar persistir deve
    /// guardar os bytes por fora.
    InputImage {
        #[serde(skip, default)]
        bytes: Vec<u8>,
        mime_type: String,
    },
    OutputText {
        text: String,
        /// Assinatura opaca de "raciocínio" que alguns modelos anexam
        /// ao Part e exigem de volta no turno seguinte. Providers que
        /// não usam o campo ignoram; `skip_serializing_if` impede
        /// vazamento no wire quando `None`.
        #[serde(skip_serializing_if = "Option::is_none", default)]
        thought_signature: Option<String>,
    },
    /// Chamada de tool, já acoplada à sua resposta. O caller empurra
    /// com `result: None`, executa a tool, e muta `result` em `Some(...)`
    /// na mesma posição do histórico. O tradutor de cada provider
    /// expande o par chamada+resultado no wire format nativo (uma
    /// mensagem "assistant" com a call, seguida de uma mensagem de
    /// resposta com o result).
    ///
    /// Invariante: a partir da primeira `ToolCall { result: None }`
    /// dentro de um run de variantes de output, só podem vir
    /// `OutputText` ou mais `ToolCall { result: None }`. Nunca outra
    /// call com `Some` depois dessa fronteira — caller que violar
    /// produz wire format rejeitado pelo provider.
    ToolCall {
        name: String,
        arguments: Value,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        result: Option<String>,
        /// Ver [`ChatMessage::OutputText::thought_signature`].
        #[serde(skip_serializing_if = "Option::is_none", default)]
        thought_signature: Option<String>,
    },
}

// ── Requisição ─────────────────────────────────────────────────────────

/// Argumentos para [`crate::Client::chat`].
pub struct ChatRequest<'a> {
    /// Modelo qualificado pelo provider, no formato `"provider/modelo"`.
    pub model: &'a str,
    /// Prompt fixo que vai no topo da conversa. Passe `""` para omitir.
    pub system: &'a str,
    pub messages: &'a [ChatMessage],
    pub tools: &'a [ChatTool<'a>],
}

/// Definição de uma tool exposta ao modelo. `parameters` é um JSON
/// Schema — `Value` arbitrário porque o shape varia por tool.
pub struct ChatTool<'a> {
    pub name: &'a str,
    pub description: &'a str,
    pub parameters: &'a Value,
}

// ── Resposta ───────────────────────────────────────────────────────────

/// Resultado de uma chamada de chat. `messages` carrega zero ou mais
/// variantes de output ([`ChatMessage::OutputText`] e
/// [`ChatMessage::ToolCall`]) na ordem em que o modelo as emitiu —
/// pode haver texto antes, depois ou intercalado com tool calls.
#[derive(Debug)]
pub struct ChatResponse {
    pub messages: Vec<ChatMessage>,
    pub input_tokens: u32,
    pub output_tokens: u32,
    /// Custo estimado em USD, ou `0.0` quando o provider não expõe
    /// pricing.
    pub cost: f64,
}

// ── Structured output ──────────────────────────────────────────────────

/// Argumentos para [`crate::Client::chat_structured`]. Igual a
/// [`ChatRequest`] mas sem `tools` — structured output e tool calls são
/// mutuamente exclusivos nos providers.
pub struct StructuredRequest<'a> {
    /// Modelo qualificado pelo provider, no formato `"provider/modelo"`.
    pub model: &'a str,
    /// Prompt fixo que vai no topo da conversa. Passe `""` para omitir.
    pub system: &'a str,
    pub messages: &'a [ChatMessage],
}

/// Resultado de [`crate::Client::chat_structured`]. `value` é a struct
/// já decodificada; `input_tokens`/`output_tokens`/`cost` seguem a
/// mesma semântica de [`ChatResponse`].
#[derive(Debug)]
pub struct StructuredResponse<T> {
    pub value: T,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cost: f64,
}
