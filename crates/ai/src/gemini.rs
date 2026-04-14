//! Cliente para a API do Google Gemini (`generativelanguage.googleapis.com`).
//!
//! Faz a tradução do formato genérico (`ChatMessage`, tools em schema
//! OpenAI/Ollama) para o wire format do Gemini (`contents` com `parts`,
//! `functionDeclarations`, `functionCall`, `functionResponse`) e de volta.

use anyhow::{Context, Result, anyhow};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde_json::{Value, json};

use crate::chat::{ChatMessage, ChatRequest, ChatResponse, ChatTool};

const BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";

pub struct GeminiClient {
    http: reqwest::Client,
    api_key: String,
}

impl GeminiClient {
    pub fn new(api_key: &str) -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(300))
                .build()
                .unwrap(),
            api_key: api_key.to_owned(),
        }
    }

    pub async fn chat(&self, request: ChatRequest<'_>) -> Result<ChatResponse> {
        let contents = translate_messages(request.messages);
        let function_declarations = translate_tools(request.tools);

        let mut body = json!({ "contents": contents });

        if !request.system.is_empty() {
            body["systemInstruction"] = json!({ "parts": [{ "text": request.system }] });
        }

        if !function_declarations.is_empty() {
            body["tools"] = json!([{ "functionDeclarations": function_declarations }]);
        }

        let model = request.model;
        let url = format!("{BASE_URL}/models/{model}:generateContent");
        let http_resp = self
            .http
            .post(&url)
            .header("x-goog-api-key", &self.api_key)
            .json(&body)
            .send()
            .await
            .context("falha ao chamar Gemini")?;

        let status = http_resp.status();
        let body_text = http_resp
            .text()
            .await
            .context("falha ao ler resposta do Gemini")?;

        if !status.is_success() {
            return Err(anyhow!(
                "erro na resposta do Gemini ({status}): {body_text}"
            ));
        }

        let resp: Value =
            serde_json::from_str(&body_text).context("falha ao parsear resposta do Gemini")?;

        translate_response(model, &resp)
    }
}

// ── Pricing (USD por 1M tokens) ────────────────────────────────────────

/// Preço unitário (USD por token) para cada modalidade faturada pelo
/// Gemini. Gerado a partir de preços em USD por 1M tokens — valores
/// divididos por 1e6 na construção (`p(...)`) para não repetir a
/// divisão a cada soma de custo.
#[derive(Clone, Copy)]
struct Pricing {
    input: f64,
    audio_input: f64,
    output: f64,
    image_output: f64,
    cache: f64,
    cache_audio: f64,
}

/// Recebe preços em USD por 1M de tokens e devolve o preço por token.
fn p(
    input: f64,
    audio_input: f64,
    output: f64,
    image_output: f64,
    cache: f64,
    cache_audio: f64,
) -> Pricing {
    Pricing {
        input: input / 1e6,
        audio_input: audio_input / 1e6,
        output: output / 1e6,
        image_output: image_output / 1e6,
        cache: cache / 1e6,
        cache_audio: cache_audio / 1e6,
    }
}

/// Tabela de preços por modelo. Modelos não conhecidos caem no `None`
/// e resultam em `cost = 0.0`. Alguns modelos têm tier por contexto
/// (`>200k`); aplicamos o tier correto com base no número de input
/// tokens.
fn pricing_for(model: &str, input_tokens: u32) -> Option<Pricing> {
    let over_200k = input_tokens > 200_000;
    Some(match model {
        "gemini-2.0-flash-001" => p(0.1, 0.7, 0.4, 0.4, 0.025, 0.175),
        "gemini-2.0-flash-lite-001" => p(0.075, 0.075, 0.3, 0.3, 0.0, 0.0),
        "gemini-2.5-pro" => {
            if over_200k {
                p(2.5, 2.5, 15.0, 15.0, 0.25, 0.25)
            } else {
                p(1.25, 1.25, 10.0, 10.0, 0.125, 0.125)
            }
        }
        "gemini-2.5-flash" => p(0.3, 1.0, 2.5, 2.5, 0.03, 0.1),
        "gemini-2.5-flash-lite" => p(0.1, 0.3, 0.4, 0.4, 0.01, 0.03),
        "gemini-2.5-flash-image" => p(0.3, 0.3, 2.5, 30.0, 0.0, 0.0),
        "gemini-3-pro-preview" => {
            if over_200k {
                p(4.0, 4.0, 18.0, 18.0, 0.0, 0.0)
            } else {
                p(2.0, 2.0, 12.0, 12.0, 0.0, 0.0)
            }
        }
        "gemini-3-pro-image-preview" => {
            if over_200k {
                p(4.0, 4.0, 18.0, 120.0, 0.0, 0.0)
            } else {
                p(2.0, 2.0, 12.0, 120.0, 0.0, 0.0)
            }
        }
        "gemini-3.1-pro-preview" => {
            if over_200k {
                p(4.0, 4.0, 18.0, 18.0, 0.4, 0.4)
            } else {
                p(2.0, 2.0, 12.0, 12.0, 0.2, 0.2)
            }
        }
        "gemini-3-flash-preview" => p(0.5, 1.0, 3.0, 3.0, 0.05, 0.1),
        "gemini-3.1-flash-lite-preview" => p(0.25, 0.5, 1.5, 1.5, 0.025, 0.05),
        "gemini-3.1-flash-image-preview" => p(0.5, 0.5, 3.0, 60.0, 0.0, 0.0),
        _ => return None,
    })
}

/// Tokens quebrados por modalidade (`text`, `audio`, `image`),
/// somados a partir do array `*TokensDetails` que o Gemini devolve.
#[derive(Default, Clone, Copy)]
struct TokensByModality {
    text: u64,
    audio: u64,
    image: u64,
}

fn tokens_by_modality(details: Option<&Value>) -> TokensByModality {
    let mut out = TokensByModality::default();
    let Some(arr) = details.and_then(|d| d.as_array()) else {
        return out;
    };
    for d in arr {
        let count = d.get("tokenCount").and_then(|v| v.as_u64()).unwrap_or(0);
        match d.get("modality").and_then(|v| v.as_str()) {
            Some("AUDIO") => out.audio += count,
            Some("IMAGE") => out.image += count,
            _ => out.text += count,
        }
    }
    out
}

/// Devolve `(input_tokens, output_tokens, cost)` extraídos do
/// `usageMetadata` da resposta. `output_tokens` já inclui os
/// `thoughtsTokenCount` — é o que foi gerado e faturado de fato.
fn compute_usage(model: &str, resp: &Value) -> (u32, u32, f64) {
    let meta = resp.get("usageMetadata");

    let input_tokens = meta
        .and_then(|m| m.get("promptTokenCount"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    let output_tokens = meta
        .and_then(|m| m.get("candidatesTokenCount"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    let thinking_tokens = meta
        .and_then(|m| m.get("thoughtsTokenCount"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;

    let cost = match pricing_for(model, input_tokens) {
        Some(pricing) => {
            // Se o Gemini devolveu o detalhamento por modalidade usamos
            // a fórmula completa; caso contrário caímos num fallback
            // que trata tudo como texto.
            if let Some(input_details) = meta.and_then(|m| m.get("promptTokensDetails")) {
                let input = tokens_by_modality(Some(input_details));
                let output =
                    tokens_by_modality(meta.and_then(|m| m.get("candidatesTokensDetails")));
                let cache = tokens_by_modality(meta.and_then(|m| m.get("cacheTokensDetails")));

                // Cache é cobrado à parte — desconta do input bruto
                // pra não cobrar duas vezes pelos mesmos tokens.
                let input_cost = (input.text.saturating_sub(cache.text) as f64) * pricing.input
                    + (input.audio.saturating_sub(cache.audio) as f64) * pricing.audio_input
                    + (input.image.saturating_sub(cache.image) as f64) * pricing.input
                    + (cache.text as f64) * pricing.cache
                    + (cache.audio as f64) * pricing.cache_audio
                    + (cache.image as f64) * pricing.cache;

                // Audio output é cobrado na mesma alíquota do text
                // output — a API não expõe tarifa separada.
                let output_cost = (output.text as f64) * pricing.output
                    + (output.image as f64) * pricing.image_output
                    + (output.audio as f64) * pricing.output;

                // thoughtsTokenCount é faturado como output text.
                input_cost + output_cost + (thinking_tokens as f64) * pricing.output
            } else {
                (input_tokens as f64) * pricing.input
                    + ((output_tokens + thinking_tokens) as f64) * pricing.output
            }
        }
        None => 0.0,
    };

    (input_tokens, output_tokens + thinking_tokens, cost)
}

// ── Tradução: ChatMessage[] → contents[] ───────────────────────────────

/// Traduz `ChatMessage`s para o `contents[]` do Gemini, agrupando
/// variantes adjacentes do mesmo role num único `Content` com vários
/// `parts`:
///
/// * `InputText`/`InputImage` → `role=user` com `text`/`inlineData`
///   parts.
/// * `OutputText`/`ToolCall` → um `role=model` (texto e
///   `functionCall` parts), seguido de `role=user` com
///   `functionResponse` parts para as calls que têm `result: Some`,
///   seguido de outro `role=model` com o sufixo quando houver.
///   `thoughtSignature` (Gemini 3.x) é reemitido como irmão do part
///   de origem.
fn translate_messages(messages: &[ChatMessage]) -> Vec<Value> {
    let mut contents: Vec<Value> = Vec::new();
    let mut i = 0;

    while i < messages.len() {
        match &messages[i] {
            ChatMessage::InputText { .. } | ChatMessage::InputImage { .. } => {
                let start = i;
                while i < messages.len()
                    && matches!(
                        messages[i],
                        ChatMessage::InputText { .. } | ChatMessage::InputImage { .. }
                    )
                {
                    i += 1;
                }
                contents.push(translate_user_group(&messages[start..i]));
            }
            ChatMessage::OutputText { .. } | ChatMessage::ToolCall { .. } => {
                let start = i;
                while i < messages.len()
                    && matches!(
                        messages[i],
                        ChatMessage::OutputText { .. } | ChatMessage::ToolCall { .. }
                    )
                {
                    i += 1;
                }
                expand_assistant_group(&messages[start..i], &mut contents);
            }
        }
    }

    contents
}

fn translate_user_group(group: &[ChatMessage]) -> Value {
    let mut parts: Vec<Value> = Vec::new();
    for msg in group {
        match msg {
            ChatMessage::InputText { text } => {
                parts.push(json!({ "text": text }));
            }
            ChatMessage::InputImage { bytes, mime_type } => {
                parts.push(json!({
                    "inlineData": {
                        "mimeType": mime_type,
                        "data": BASE64.encode(bytes),
                    }
                }));
            }
            _ => unreachable!("translate_user_group recebeu variante não-user"),
        }
    }
    // Gemini exige pelo menos um part no Content.
    if parts.is_empty() {
        parts.push(json!({ "text": "" }));
    }
    json!({ "role": "user", "parts": parts })
}

/// Expande um run de variantes assistant em um ou mais `Content`s do
/// Gemini. Ver `expand_assistant_group` em `ollama.rs` para a forma
/// geral; aqui o user turn intermediário vira `role=user` com
/// `functionResponse` parts em vez de `role=tool` messages.
fn expand_assistant_group(group: &[ChatMessage], out: &mut Vec<Value>) {
    let split = group
        .iter()
        .position(|m| matches!(m, ChatMessage::ToolCall { result: None, .. }))
        .unwrap_or(group.len());

    let prefix_has_result = group[..split].iter().any(|m| {
        matches!(
            m,
            ChatMessage::ToolCall {
                result: Some(_),
                ..
            }
        )
    });

    if !prefix_has_result {
        out.push(build_model_content(group));
        return;
    }

    out.push(build_model_content(&group[..split]));
    out.push(build_function_response_content(&group[..split]));
    if split < group.len() {
        out.push(build_model_content(&group[split..]));
    }
}

fn build_model_content(slice: &[ChatMessage]) -> Value {
    let mut parts: Vec<Value> = Vec::new();
    for msg in slice {
        match msg {
            ChatMessage::OutputText {
                text,
                thought_signature,
            } => {
                if text.is_empty() && thought_signature.is_none() {
                    continue;
                }
                let mut part = json!({ "text": text });
                if let Some(sig) = thought_signature {
                    part["thoughtSignature"] = json!(sig);
                }
                parts.push(part);
            }
            ChatMessage::ToolCall {
                name,
                arguments,
                thought_signature,
                ..
            } => {
                let mut part = json!({
                    "functionCall": {
                        "name": name,
                        "args": arguments,
                    }
                });
                // Gemini 3.x exige que `thoughtSignature` seja reemitido
                // no mesmo Part do functionCall original — sem isso a
                // API devolve 400 INVALID_ARGUMENT. Vive como irmão de
                // `functionCall` no Part, não dentro dele.
                if let Some(sig) = thought_signature {
                    part["thoughtSignature"] = json!(sig);
                }
                parts.push(part);
            }
            _ => unreachable!("build_model_content recebeu variante não-assistant"),
        }
    }
    if parts.is_empty() {
        parts.push(json!({ "text": "" }));
    }
    json!({ "role": "model", "parts": parts })
}

/// Emite um `role=user` com `functionResponse` parts para cada
/// `ToolCall` do slice que tenha `result: Some`. Ignora texto
/// e calls com `result: None` (o caller deve garantir que o slice
/// corresponde ao prefix da expansão).
fn build_function_response_content(slice: &[ChatMessage]) -> Value {
    let parts: Vec<Value> = slice
        .iter()
        .filter_map(|msg| match msg {
            ChatMessage::ToolCall {
                name,
                result: Some(content),
                ..
            } => {
                // Gemini espera `response` como objeto. Se o conteúdo já
                // for JSON-objeto, usa direto; caso contrário envolve em
                // {"content": <valor>}.
                let response = match serde_json::from_str::<Value>(content) {
                    Ok(v) if v.is_object() => v,
                    Ok(v) => json!({ "content": v }),
                    Err(_) => json!({ "content": content }),
                };
                Some(json!({
                    "functionResponse": {
                        "name": name,
                        "response": response,
                    }
                }))
            }
            _ => None,
        })
        .collect();
    json!({ "role": "user", "parts": parts })
}

// ── Tradução: ChatTool[] → functionDeclarations ────────────────────────

fn translate_tools(tools: &[ChatTool<'_>]) -> Vec<Value> {
    tools
        .iter()
        .map(|t| {
            json!({
                "name": t.name,
                "description": t.description,
                "parameters": t.parameters,
            })
        })
        .collect()
}

// ── Tradução: response Gemini → ChatResponse ───────────────────────────

fn translate_response(model: &str, resp: &Value) -> Result<ChatResponse> {
    let parts = resp
        .get("candidates")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("content"))
        .and_then(|c| c.get("parts"))
        .and_then(|p| p.as_array())
        .context("resposta do Gemini sem candidates[0].content.parts")?;

    let mut messages: Vec<ChatMessage> = Vec::new();

    for part in parts {
        // `thoughtSignature` vive no Part (irmão de `text`/`functionCall`),
        // não dentro do objeto interno. Gemini 3.x exige que ele seja
        // devolvido no turno seguinte no mesmo Part de origem — então
        // capturamos por part, não por tipo.
        let thought_signature = part
            .get("thoughtSignature")
            .and_then(|v| v.as_str())
            .map(|s| s.to_owned());

        if let Some(text) = part.get("text").and_then(|v| v.as_str()) {
            messages.push(ChatMessage::OutputText {
                text: text.to_owned(),
                thought_signature,
            });
        } else if let Some(call) = part.get("functionCall") {
            let name = call
                .get("name")
                .and_then(|v| v.as_str())
                .context("functionCall sem 'name'")?
                .to_owned();
            let arguments = call.get("args").cloned().unwrap_or(json!({}));
            messages.push(ChatMessage::ToolCall {
                name,
                arguments,
                result: None,
                thought_signature,
            });
        }
    }

    let (input_tokens, output_tokens, cost) = compute_usage(model, resp);
    Ok(ChatResponse {
        messages,
        input_tokens,
        output_tokens,
        cost,
    })
}
