//! Cliente para a API do Ollama.
//!
//! Suporta chat com tool calls e embeddings (com cache opcional em disco).

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::chat::{ChatMessage, ChatResponse};

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
        let mut body = serde_json::json!({
            "model": model,
            "messages": messages,
            "stream": false,
        });

        if !tools.is_empty() {
            body["tools"] = Value::Array(tools.to_vec());
        }

        let resp = self
            .http
            .post(format!("{}/api/chat", self.base_url))
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json::<ChatResponse>()
            .await?;

        Ok(resp)
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
