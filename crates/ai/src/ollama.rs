//! Cliente para a API do Ollama.
//!
//! Chat usa o endpoint OpenAI-compatível (`/v1/chat/completions`), que
//! aceita `content` como array de parts (`text` + `image_url`) e
//! permite intercalar texto e imagens na mesma user message. Embeddings
//! usam o endpoint nativo `/api/embed`.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::chat::{ChatMessage, ChatRequest, ChatResponse, StructuredRequest};

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

    pub async fn chat(&self, request: ChatRequest<'_>) -> Result<ChatResponse> {
        let mut translated = Vec::with_capacity(request.messages.len() + 1);
        if !request.system.is_empty() {
            translated.push(json!({ "role": "system", "content": request.system }));
        }
        translated.extend(translate_messages(request.messages));

        let mut body = json!({
            "model": request.model,
            "messages": translated,
            "stream": false,
        });

        if !request.tools.is_empty() {
            let tools: Vec<Value> = request
                .tools
                .iter()
                .map(|t| {
                    json!({
                        "type": "function",
                        "function": {
                            "name": t.name,
                            "description": t.description,
                            "parameters": t.parameters,
                        }
                    })
                })
                .collect();
            body["tools"] = Value::Array(tools);
        }

        let raw = self.post_chat(&body).await?;
        translate_response(&raw)
    }

    /// Structured output: força o Ollama a devolver um JSON que
    /// decodifica na struct do caller. O schema vai no campo `format`
    /// (extensão do endpoint OpenAI-compat do Ollama). Tools não são
    /// suportadas neste modo — o tipo [`StructuredRequest`] não
    /// carrega o campo.
    pub async fn chat_structured(
        &self,
        request: StructuredRequest<'_>,
        schema: &Value,
    ) -> Result<(String, u32, u32, f64)> {
        let mut translated = Vec::with_capacity(request.messages.len() + 1);
        if !request.system.is_empty() {
            translated.push(json!({ "role": "system", "content": request.system }));
        }
        translated.extend(translate_messages(request.messages));

        let body = json!({
            "model": request.model,
            "messages": translated,
            "stream": false,
            "format": schema,
        });

        let raw = self.post_chat(&body).await?;
        let resp = translate_response(&raw)?;
        let text = resp
            .messages
            .into_iter()
            .find_map(|m| match m {
                ChatMessage::OutputText { text, .. } => Some(text),
                _ => None,
            })
            .context("resposta do Ollama sem texto")?;
        Ok((text, resp.input_tokens, resp.output_tokens, resp.cost))
    }

    async fn post_chat(&self, body: &Value) -> Result<Value> {
        self.http
            .post(format!("{}/v1/chat/completions", self.base_url))
            .json(body)
            .send()
            .await
            .context("falha ao chamar Ollama")?
            .error_for_status()
            .context("erro na resposta do Ollama")?
            .json()
            .await
            .context("falha ao parsear JSON do Ollama")
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

/// Traduz `ChatMessage`s para o formato esperado pelo endpoint
/// OpenAI-compatível. Variantes adjacentes do mesmo role são fundidas
/// numa única entrada:
///
/// * `InputText`/`InputImage` → `role=user` com `content` como array de
///   parts (`text` + `image_url` com data URI).
/// * `OutputText`/`ToolCall` → um `role=assistant` (texto
///   concatenado + `tool_calls`), seguido de uma `role=tool` por call
///   que tem `result: Some`, seguido de um segundo `role=assistant`
///   com o sufixo (texto e calls com `result: None`), quando houver.
fn translate_messages(messages: &[ChatMessage]) -> Vec<Value> {
    let mut out: Vec<Value> = Vec::with_capacity(messages.len());
    let mut call_id_seq: u64 = 0;
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
                out.push(translate_user_group(&messages[start..i]));
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
                expand_assistant_group(&messages[start..i], &mut call_id_seq, &mut out);
            }
        }
    }

    out
}

fn translate_user_group(group: &[ChatMessage]) -> Value {
    // Caso comum: um único texto. Mantém `content` como string para não
    // forçar o array de parts (mais barato e mais legível no log).
    if let [ChatMessage::InputText { text }] = group {
        return json!({ "role": "user", "content": text });
    }

    let mut parts: Vec<Value> = Vec::new();
    for msg in group {
        match msg {
            ChatMessage::InputText { text } if text.is_empty() => {}
            ChatMessage::InputText { text } => {
                parts.push(json!({ "type": "text", "text": text }));
            }
            ChatMessage::InputImage { bytes, mime_type } => {
                let data_uri = format!("data:{};base64,{}", mime_type, BASE64.encode(bytes));
                parts.push(json!({
                    "type": "image_url",
                    "image_url": { "url": data_uri },
                }));
            }
            _ => unreachable!("translate_user_group recebeu variante não-user"),
        }
    }
    json!({ "role": "user", "content": parts })
}

/// Expande um run de variantes assistant em mensagens OpenAI:
/// assistant → (tool messages com results) → assistant (sufixo).
///
/// `split` = índice da primeira `ToolCall` com `result: None`
/// (ou `group.len()` se não houver). Se o prefix `[0..split]` contém
/// pelo menos uma call com `result: Some`, emite o par
/// assistant+tool_messages; caso contrário o run inteiro vira uma
/// única mensagem assistant (evita dois `role=assistant` adjacentes).
fn expand_assistant_group(group: &[ChatMessage], call_id_seq: &mut u64, out: &mut Vec<Value>) {
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
        // Nenhuma call resolvida → emite o run inteiro como um único
        // assistant message (sem user turn subsequente).
        out.push(build_assistant_message(group, call_id_seq, None));
        return;
    }

    // Prefix carrega pelo menos uma call com result. Quebra em:
    // assistant(prefix) → tool messages → assistant(sufixo) [se houver].
    let mut collected_results: Vec<(String, String)> = Vec::new();
    out.push(build_assistant_message(
        &group[..split],
        call_id_seq,
        Some(&mut collected_results),
    ));
    for (id, content) in collected_results {
        out.push(json!({
            "role": "tool",
            "tool_call_id": id,
            "content": content,
        }));
    }
    if split < group.len() {
        out.push(build_assistant_message(&group[split..], call_id_seq, None));
    }
}

/// Monta um único `role=assistant` com texto concatenado e `tool_calls`.
/// Se `results_sink` for `Some`, cada call com `result: Some` tem seu
/// id sintético e content empurrados lá (para emitir como `role=tool`
/// logo em seguida).
fn build_assistant_message(
    slice: &[ChatMessage],
    call_id_seq: &mut u64,
    mut results_sink: Option<&mut Vec<(String, String)>>,
) -> Value {
    let mut text_chunks: Vec<&str> = Vec::new();
    let mut tool_calls: Vec<Value> = Vec::new();

    for msg in slice {
        match msg {
            ChatMessage::OutputText { text, .. } => {
                if !text.is_empty() {
                    text_chunks.push(text);
                }
            }
            ChatMessage::ToolCall {
                name,
                arguments,
                result,
                ..
            } => {
                let id = format!("call_{}", *call_id_seq);
                *call_id_seq += 1;
                // OpenAI exige `arguments` como string JSON.
                let args_str = serde_json::to_string(arguments).unwrap_or_else(|_| "{}".into());
                tool_calls.push(json!({
                    "id": &id,
                    "type": "function",
                    "function": {
                        "name": name,
                        "arguments": args_str,
                    }
                }));
                if let (Some(sink), Some(content)) = (results_sink.as_deref_mut(), result.as_ref())
                {
                    sink.push((id, content.clone()));
                }
            }
            _ => unreachable!("build_assistant_message recebeu variante não-assistant"),
        }
    }

    // OpenAI-compat aceita `content` junto com `tool_calls` no turno
    // do assistant — preserva o texto quando o modelo devolve ambos,
    // senão manda null.
    let content = if text_chunks.is_empty() {
        Value::Null
    } else {
        Value::String(text_chunks.concat())
    };

    if tool_calls.is_empty() {
        json!({ "role": "assistant", "content": content })
    } else {
        json!({
            "role": "assistant",
            "content": content,
            "tool_calls": tool_calls,
        })
    }
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

    let mut messages: Vec<ChatMessage> = Vec::new();
    if !content.is_empty() {
        messages.push(ChatMessage::OutputText {
            text: content,
            thought_signature: None,
        });
    }

    if let Some(arr) = message.get("tool_calls").and_then(|v| v.as_array()) {
        for call in arr {
            let Some(func) = call.get("function") else {
                continue;
            };
            let Some(name) = func.get("name").and_then(|v| v.as_str()) else {
                continue;
            };
            // OpenAI devolve `arguments` como string JSON, mas alguns
            // servidores compatíveis mandam objeto direto. Aceitamos os
            // dois formatos.
            let arguments = match func.get("arguments") {
                Some(Value::String(s)) => serde_json::from_str(s).unwrap_or(Value::Null),
                Some(v) => v.clone(),
                None => Value::Null,
            };
            messages.push(ChatMessage::ToolCall {
                name: name.to_owned(),
                arguments,
                result: None,
                thought_signature: None,
            });
        }
    }

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
        messages,
        input_tokens,
        output_tokens,
        // Ollama roda local — sem custo monetário.
        cost: 0.0,
    })
}
