//! Cliente para a API do Ollama.
//!
//! Chat usa o endpoint OpenAI-compatível (`/v1/chat/completions`) em vez
//! do nativo `/api/chat` — ele aceita `content` como array de parts
//! (`text` + `image_url`), o que permite intercalar labels e imagens na
//! mesma user message (coisa que o `/api/chat` não suporta). Embeddings
//! continuam usando o endpoint nativo `/api/embed`.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::chat::{
    ChatMessage, ChatResponse, ChatResponseMessage, ChatUsage, ToolCall, ToolCallFunction,
};

const EMBED_BATCH_SIZE: usize = 64;

pub struct OllamaClient {
    http: reqwest::Client,
    base_url: String,
}

impl OllamaClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(300))
                .build()
                .unwrap(),
            base_url: base_url.trim_end_matches('/').to_owned(),
        }
    }

    // ── Chat ───────────────────────────────────────────────────────────

    pub async fn chat(
        &self,
        model: &str,
        messages: &[ChatMessage],
        tools: &[Value],
    ) -> Result<ChatResponse> {
        let translated = translate_messages(messages);

        let mut body = json!({
            "model": model,
            "messages": translated,
            "stream": false,
        });

        if !tools.is_empty() {
            body["tools"] = Value::Array(tools.to_vec());
        }

        let raw: Value = self
            .http
            .post(format!("{}/v1/chat/completions", self.base_url))
            .json(&body)
            .send()
            .await
            .context("falha ao chamar Ollama")?
            .error_for_status()
            .context("erro na resposta do Ollama")?
            .json()
            .await
            .context("falha ao parsear JSON do Ollama")?;

        translate_response(&raw)
    }

    // ── Embeddings ─────────────────────────────────────────────────────

    /// Gera embeddings para uma lista de textos.
    ///
    /// Se `cache_dir` for `Some`, os embeddings são cacheados em disco
    /// como arquivos `.bin` nomeados pelo SHA256 do texto.
    pub async fn embed_many(
        &self,
        model: &str,
        texts: &[String],
        cache_dir: Option<&PathBuf>,
    ) -> Result<Vec<Vec<f32>>> {
        if let Some(dir) = cache_dir {
            std::fs::create_dir_all(dir)?;
        }

        let mut results: Vec<Option<Vec<f32>>> = vec![None; texts.len()];
        let mut uncached: Vec<usize> = Vec::new();

        for (i, text) in texts.iter().enumerate() {
            let cached = cache_dir.and_then(|dir| read_cached(dir, text));
            if let Some(vec) = cached {
                results[i] = Some(vec);
            } else {
                uncached.push(i);
            }
        }

        let cached_count = texts.len() - uncached.len();

        if uncached.is_empty() {
            println!("      {} embeddings (todos do cache)", texts.len());
        } else {
            println!(
                "      {} embeddings ({} cache, {} novos)",
                texts.len(),
                cached_count,
                uncached.len()
            );

            for chunk in uncached.chunks(EMBED_BATCH_SIZE) {
                let input: Vec<&str> = chunk.iter().map(|&i| texts[i].as_str()).collect();

                let resp = self
                    .http
                    .post(format!("{}/api/embed", self.base_url))
                    .json(&serde_json::json!({
                        "model": model,
                        "input": input,
                    }))
                    .send()
                    .await
                    .context("falha ao chamar Ollama")?
                    .error_for_status()
                    .context("erro na resposta do Ollama")?
                    .json::<EmbedResponse>()
                    .await
                    .context("falha ao parsear resposta do Ollama")?;

                if resp.embeddings.len() != chunk.len() {
                    bail!(
                        "Ollama retornou {} embeddings, esperava {}",
                        resp.embeddings.len(),
                        chunk.len()
                    );
                }

                for (j, embedding) in resp.embeddings.into_iter().enumerate() {
                    let idx = chunk[j];
                    if let Some(dir) = cache_dir {
                        write_cached(dir, &texts[idx], &embedding);
                    }
                    results[idx] = Some(embedding);
                }
            }
        }

        Ok(results.into_iter().map(|r| r.unwrap()).collect())
    }
}

#[derive(serde::Deserialize)]
struct EmbedResponse {
    embeddings: Vec<Vec<f32>>,
}

fn cache_key(text: &str) -> String {
    let hash = Sha256::digest(text.as_bytes());
    hash.iter().map(|b| format!("{b:02x}")).collect()
}

fn read_cached(cache_dir: &Path, text: &str) -> Option<Vec<f32>> {
    let path = cache_dir.join(format!("{}.bin", cache_key(text)));
    let data = std::fs::read(&path).ok()?;
    if data.len() % 4 != 0 {
        return None;
    }
    Some(
        data.chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect(),
    )
}

fn write_cached(cache_dir: &Path, text: &str, vec: &[f32]) {
    let path = cache_dir.join(format!("{}.bin", cache_key(text)));
    let data: Vec<u8> = vec.iter().flat_map(|v| v.to_le_bytes()).collect();
    let _ = std::fs::write(&path, &data);
}

// ── Tradução: ChatMessage[] → messages no formato OpenAI ───────────────

/// Traduz o array genérico `ChatMessage` para o formato esperado pelo
/// endpoint OpenAI-compatível. Diferente do `/api/chat` nativo, aqui:
///
/// * user messages com imagens viram `content` como array de parts
///   intercalados (`text` + `image_url` com data URI), preservando a
///   posição de cada label relativa à sua imagem.
/// * tool_calls no assistant reusam o `ToolCall.id` vindo do provider
///   (ou gerado sinteticamente na tradução do Gemini), com `arguments`
///   como string JSON (exigido pelo spec OpenAI).
/// * mensagens `role=tool` usam o `tool_call_id` guardado no próprio
///   `ChatMessage` — isso permite que a mesma tool seja chamada várias
///   vezes sem que respostas sejam trocadas.
fn translate_messages(messages: &[ChatMessage]) -> Vec<Value> {
    let mut out: Vec<Value> = Vec::with_capacity(messages.len());

    for msg in messages.iter() {
        match msg.role.as_str() {
            "system" => {
                out.push(json!({
                    "role": "system",
                    "content": msg.content,
                }));
            }
            "user" => {
                if msg.images.is_empty() {
                    out.push(json!({
                        "role": "user",
                        "content": msg.content,
                    }));
                } else {
                    let mut parts: Vec<Value> = Vec::new();
                    if !msg.content.is_empty() {
                        parts.push(json!({ "type": "text", "text": msg.content }));
                    }
                    for img in &msg.images {
                        if let Some(label) = &img.label {
                            parts.push(json!({ "type": "text", "text": label }));
                        }
                        let data_uri = format!(
                            "data:{};base64,{}",
                            img.mime_type,
                            BASE64.encode(&img.bytes)
                        );
                        parts.push(json!({
                            "type": "image_url",
                            "image_url": { "url": data_uri },
                        }));
                    }
                    out.push(json!({
                        "role": "user",
                        "content": parts,
                    }));
                }
            }
            "assistant" => {
                if let Some(tool_calls) = &msg.tool_calls {
                    let calls: Vec<Value> = tool_calls
                        .iter()
                        .map(|c| {
                            // OpenAI exige `arguments` como string JSON.
                            let args_str = serde_json::to_string(&c.function.arguments)
                                .unwrap_or_else(|_| "{}".into());
                            json!({
                                "id": c.id,
                                "type": "function",
                                "function": {
                                    "name": c.function.name,
                                    "arguments": args_str,
                                }
                            })
                        })
                        .collect();
                    out.push(json!({
                        "role": "assistant",
                        "content": Value::Null,
                        "tool_calls": calls,
                    }));
                } else {
                    out.push(json!({
                        "role": "assistant",
                        "content": msg.content,
                    }));
                }
            }
            "tool" => {
                let tool_call_id = msg.tool_call_id.as_deref().unwrap_or("");
                out.push(json!({
                    "role": "tool",
                    "tool_call_id": tool_call_id,
                    "content": msg.content,
                }));
            }
            _ => {
                // roles desconhecidos caem silenciosamente — igual aos
                // outros providers.
            }
        }
    }

    out
}

// ── Tradução: response OpenAI → ChatResponse ──────────────────────────

fn translate_response(raw: &Value) -> Result<ChatResponse> {
    let choice = raw
        .get("choices")
        .and_then(|c| c.as_array())
        .and_then(|c| c.first())
        .context("resposta do Ollama sem choices[0]")?;

    let message = choice
        .get("message")
        .context("resposta do Ollama sem choices[0].message")?;

    let content = message
        .get("content")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_owned();

    let tool_calls: Option<Vec<ToolCall>> = message
        .get("tool_calls")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .enumerate()
                .filter_map(|(i, call)| {
                    let func = call.get("function")?;
                    let name = func.get("name")?.as_str()?.to_owned();
                    // OpenAI devolve `arguments` como string JSON, mas
                    // alguns servidores compatíveis mandam objeto direto.
                    // Aceitamos os dois formatos.
                    let args = match func.get("arguments") {
                        Some(Value::String(s)) => serde_json::from_str(s).unwrap_or(Value::Null),
                        Some(v) => v.clone(),
                        None => Value::Null,
                    };
                    // Preferimos o id devolvido pelo servidor; se ausente,
                    // fabricamos um estável baseado na posição — suficiente
                    // para pareamento dentro desta mesma rodada.
                    let id = call
                        .get("id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_owned())
                        .unwrap_or_else(|| format!("ollama_call_{i}"));
                    Some(ToolCall {
                        id,
                        function: ToolCallFunction {
                            name,
                            arguments: args,
                        },
                        thought_signature: None,
                    })
                })
                .collect()
        })
        .filter(|v: &Vec<ToolCall>| !v.is_empty());

    let usage = raw.get("usage");
    let input_tokens = usage
        .and_then(|u| u.get("prompt_tokens"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    let output_tokens = usage
        .and_then(|u| u.get("completion_tokens"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;

    Ok(ChatResponse {
        message: ChatResponseMessage {
            role: "assistant".into(),
            content,
            tool_calls,
        },
        usage: ChatUsage {
            input_tokens,
            output_tokens,
            // Ollama roda local — sem custo monetário.
            cost: 0.0,
        },
    })
}
