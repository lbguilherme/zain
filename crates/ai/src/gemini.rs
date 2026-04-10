//! Cliente para a API do Google Gemini (`generativelanguage.googleapis.com`).
//!
//! Faz a tradução do formato genérico (`ChatMessage`, tools em schema
//! OpenAI/Ollama) para o wire format do Gemini (`contents` com `parts`,
//! `functionDeclarations`, `functionCall`, `functionResponse`) e de volta.

use anyhow::{Context, Result, anyhow, bail};
use serde_json::{Value, json};

use crate::chat::{ChatMessage, ChatResponse, ChatResponseMessage, ToolCall, ToolCallFunction};

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

        translate_response(&resp)
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
                contents.push(json!({
                    "role": "user",
                    "parts": [{ "text": msg.content }],
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

fn translate_response(resp: &Value) -> Result<ChatResponse> {
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
    })
}
