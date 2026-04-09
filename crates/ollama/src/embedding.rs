use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};

use crate::OllamaClient;

const BATCH_SIZE: usize = 64;

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

impl OllamaClient {
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

            for chunk in uncached.chunks(BATCH_SIZE) {
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
