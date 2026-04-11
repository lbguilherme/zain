//! Cliente para a API do Google Gemini (`generativelanguage.googleapis.com`).
//!
//! Faz a tradução do formato genérico (`ChatMessage`, tools em schema
//! OpenAI/Ollama) para o wire format do Gemini (`contents` com `parts`,
//! `functionDeclarations`, `functionCall`, `functionResponse`) e de volta.

use anyhow::{Context, Result, anyhow, bail};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde_json::{Value, json};

use crate::chat::{
    ChatMessage, ChatResponse, ChatResponseMessage, ChatUsage, ToolCall, ToolCallFunction,
};

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

    pub async fn chat(
        &self,
        model: &str,
        messages: &[ChatMessage],
        tools: &[Value],
    ) -> Result<ChatResponse> {
        let (system_instruction, contents) = translate_messages(messages)?;
        let function_declarations = translate_tools(tools);

        let mut body = json!({ "contents": contents });

        if let Some(sys) = system_instruction {
            body["systemInstruction"] = sys;
        }

        if !function_declarations.is_empty() {
            body["tools"] = json!([{ "functionDeclarations": function_declarations }]);
        }

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

/// Construtor espelhado do helper `p(...)` do TypeScript — recebe os
/// preços em USD por 1M de tokens e devolve o preço por token.
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

/// Tokens quebrados por modalidade (`text`, `audio`, `image`). Espelha
/// o `getTokensByModality` do TS — percorre o array `*TokensDetails`
/// que o Gemini devolve e soma por `modality`.
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

fn compute_usage(model: &str, resp: &Value) -> ChatUsage {
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
            // a fórmula completa (espelhada do cliente TS). Do contrário
            // caímos num fallback que trata tudo como texto.
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

                // Audio output é cobrado na mesma alíquota do text output
                // (o TS faz o mesmo — não existe tarifa separada ainda).
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

    ChatUsage {
        // Reportamos output_tokens somando thinking — é o que foi gerado
        // e faturado de fato.
        input_tokens,
        output_tokens: output_tokens + thinking_tokens,
        cost,
    }
}

// ── Tradução: ChatMessage[] → (systemInstruction, contents[]) ──────────

fn translate_messages(messages: &[ChatMessage]) -> Result<(Option<Value>, Vec<Value>)> {
    let mut system_parts: Vec<Value> = Vec::new();
    let mut contents: Vec<Value> = Vec::new();

    for msg in messages {
        match msg.role.as_str() {
            "system" => {
                system_parts.push(json!({ "text": msg.content }));
            }
            "user" => {
                // Montamos os parts na ordem: texto primeiro (se houver)
                // seguido das imagens como `inlineData`. O Gemini exige
                // `mimeType` + `data` em base64.
                let mut parts: Vec<Value> = Vec::new();
                if !msg.content.is_empty() || msg.images.is_empty() {
                    parts.push(json!({ "text": msg.content }));
                }
                for img in &msg.images {
                    parts.push(json!({
                        "inlineData": {
                            "mimeType": img.mime_type,
                            "data": BASE64.encode(&img.bytes),
                        }
                    }));
                }
                contents.push(json!({
                    "role": "user",
                    "parts": parts,
                }));
            }
            "assistant" => {
                if let Some(tool_calls) = &msg.tool_calls {
                    let parts: Vec<Value> = tool_calls
                        .iter()
                        .map(|c| {
                            let mut part = json!({
                                "functionCall": {
                                    "name": c.function.name,
                                    "args": c.function.arguments,
                                }
                            });
                            // Gemini 3.x exige que `thoughtSignature` seja
                            // reemitido no mesmo Part do functionCall
                            // original — sem isso a API devolve 400
                            // INVALID_ARGUMENT. Vive como irmão de
                            // `functionCall` no Part, não dentro dele.
                            if let Some(sig) = &c.thought_signature {
                                part["thoughtSignature"] = json!(sig);
                            }
                            part
                        })
                        .collect();
                    contents.push(json!({
                        "role": "model",
                        "parts": parts,
                    }));
                } else {
                    contents.push(json!({
                        "role": "model",
                        "parts": [{ "text": msg.content }],
                    }));
                }
            }
            "tool" => {
                let name = msg.tool_name.as_deref().ok_or_else(|| {
                    anyhow::anyhow!(
                        "ChatMessage role='tool' sem tool_name — Gemini requer o nome da função"
                    )
                })?;
                // Gemini espera `response` como objeto. Se o conteúdo já for
                // JSON-objeto, usa direto; caso contrário envolve em
                // {"content": <valor>}.
                let response = match serde_json::from_str::<Value>(&msg.content) {
                    Ok(v) if v.is_object() => v,
                    Ok(v) => json!({ "content": v }),
                    Err(_) => json!({ "content": msg.content }),
                };
                contents.push(json!({
                    "role": "user",
                    "parts": [{
                        "functionResponse": {
                            "name": name,
                            "response": response,
                        }
                    }],
                }));
            }
            other => {
                bail!("role desconhecido em ChatMessage: '{other}'");
            }
        }
    }

    let system_instruction = if system_parts.is_empty() {
        None
    } else {
        Some(json!({ "parts": system_parts }))
    };

    Ok((system_instruction, contents))
}

// ── Tradução: tools (schema OpenAI) → functionDeclarations ─────────────

fn translate_tools(tools: &[Value]) -> Vec<Value> {
    tools
        .iter()
        .filter_map(|t| {
            let func = t.get("function")?;
            let name = func.get("name")?.as_str()?;
            let description = func
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let parameters = func.get("parameters").cloned().unwrap_or(json!({}));
            Some(json!({
                "name": name,
                "description": description,
                "parameters": parameters,
            }))
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

    let mut text_chunks: Vec<String> = Vec::new();
    let mut tool_calls: Vec<ToolCall> = Vec::new();

    for part in parts {
        if let Some(text) = part.get("text").and_then(|v| v.as_str()) {
            text_chunks.push(text.to_owned());
        } else if let Some(call) = part.get("functionCall") {
            let name = call
                .get("name")
                .and_then(|v| v.as_str())
                .context("functionCall sem 'name'")?
                .to_owned();
            let args = call.get("args").cloned().unwrap_or(json!({}));
            // `thoughtSignature` vive no Part (irmão de `functionCall`),
            // não dentro do objeto functionCall. Gemini 3.x exige que ele
            // seja devolvido no turno seguinte.
            let thought_signature = part
                .get("thoughtSignature")
                .and_then(|v| v.as_str())
                .map(|s| s.to_owned());
            tool_calls.push(ToolCall {
                function: ToolCallFunction {
                    name,
                    arguments: args,
                },
                thought_signature,
            });
        }
    }

    Ok(ChatResponse {
        message: ChatResponseMessage {
            role: "assistant".into(),
            content: text_chunks.join(""),
            tool_calls: if tool_calls.is_empty() {
                None
            } else {
                Some(tool_calls)
            },
        },
        usage: compute_usage(model, resp),
    })
}
