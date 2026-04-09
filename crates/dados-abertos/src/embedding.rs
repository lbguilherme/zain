use std::collections::HashMap;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use half::f16;
use serde::Deserialize;

const EMBEDDING_MODEL: &str = "qwen3-embedding:4b-q4_K_M";
const BATCH_SIZE: usize = 64;
const CACHE_MAGIC: &[u8; 4] = b"EMBC";
const CACHE_VERSION: u32 = 1;

#[derive(Deserialize)]
struct EmbedResponse {
    embeddings: Vec<Vec<f32>>,
}

pub struct EmbeddingClient {
    http: reqwest::Client,
    base_url: String,
    cache: HashMap<String, Vec<f32>>,
    cache_path: PathBuf,
}

impl EmbeddingClient {
    pub async fn new(cache_name: &str) -> Result<Self> {
        let base_url =
            std::env::var("OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434".to_string());

        let cache_dir = PathBuf::from(".dados_abertos").join("embeddings");
        tokio::fs::create_dir_all(&cache_dir).await?;

        let cache_path = cache_dir.join(format!("{cache_name}.bin"));
        let cache = load_cache(&cache_path).unwrap_or_default();

        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()?;

        Ok(Self {
            http,
            base_url,
            cache,
            cache_path,
        })
    }

    pub async fn embed_many(&mut self, texts: &[String]) -> Result<Vec<pgvector::HalfVector>> {
        let mut results: Vec<Option<Vec<f32>>> = vec![None; texts.len()];
        let mut uncached: Vec<usize> = Vec::new();

        for (i, text) in texts.iter().enumerate() {
            if let Some(cached) = self.cache.get(text) {
                results[i] = Some(cached.clone());
            } else {
                uncached.push(i);
            }
        }

        let cached_count = texts.len() - uncached.len();

        if uncached.is_empty() {
            println!("    {} embeddings (todos do cache)", texts.len());
        } else {
            println!(
                "    {} embeddings ({} do cache, {} novos)",
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
                        "model": EMBEDDING_MODEL,
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
                    self.cache.insert(texts[idx].clone(), embedding.clone());
                    results[idx] = Some(embedding);
                }
            }
        }

        Ok(results
            .into_iter()
            .map(|r| {
                let v: Vec<f16> = r.unwrap().into_iter().map(f16::from_f32).collect();
                pgvector::HalfVector::from(v)
            })
            .collect())
    }

    pub async fn save_cache(&self) -> Result<()> {
        let file = std::fs::File::create(&self.cache_path)?;
        let mut w = BufWriter::new(file);

        w.write_all(CACHE_MAGIC)?;
        w.write_all(&CACHE_VERSION.to_le_bytes())?;

        let model = EMBEDDING_MODEL.as_bytes();
        w.write_all(&(model.len() as u32).to_le_bytes())?;
        w.write_all(model)?;

        w.write_all(&(self.cache.len() as u32).to_le_bytes())?;

        for (key, vec) in &self.cache {
            let key_bytes = key.as_bytes();
            w.write_all(&(key_bytes.len() as u32).to_le_bytes())?;
            w.write_all(key_bytes)?;
            w.write_all(&(vec.len() as u32).to_le_bytes())?;
            for &v in vec {
                w.write_all(&v.to_le_bytes())?;
            }
        }

        w.flush()?;
        println!(
            "    Cache salvo em {} ({} entradas)",
            self.cache_path.display(),
            self.cache.len()
        );
        Ok(())
    }
}

fn load_cache(path: &PathBuf) -> Result<HashMap<String, Vec<f32>>> {
    if !path.exists() {
        return Ok(HashMap::new());
    }

    let file = std::fs::File::open(path)?;
    let mut r = BufReader::new(file);

    let mut magic = [0u8; 4];
    r.read_exact(&mut magic)?;
    if &magic != CACHE_MAGIC {
        return Ok(HashMap::new());
    }

    let mut buf4 = [0u8; 4];

    r.read_exact(&mut buf4)?;
    let version = u32::from_le_bytes(buf4);
    if version != CACHE_VERSION {
        return Ok(HashMap::new());
    }

    r.read_exact(&mut buf4)?;
    let model_len = u32::from_le_bytes(buf4) as usize;
    let mut model_buf = vec![0u8; model_len];
    r.read_exact(&mut model_buf)?;
    let model = String::from_utf8(model_buf)?;
    if model != EMBEDDING_MODEL {
        return Ok(HashMap::new());
    }

    r.read_exact(&mut buf4)?;
    let count = u32::from_le_bytes(buf4) as usize;

    let mut cache = HashMap::with_capacity(count);
    for _ in 0..count {
        r.read_exact(&mut buf4)?;
        let key_len = u32::from_le_bytes(buf4) as usize;
        let mut key_buf = vec![0u8; key_len];
        r.read_exact(&mut key_buf)?;
        let key = String::from_utf8(key_buf)?;

        r.read_exact(&mut buf4)?;
        let dim = u32::from_le_bytes(buf4) as usize;
        let mut vec_buf = vec![0u8; dim * 4];
        r.read_exact(&mut vec_buf)?;
        let vec: Vec<f32> = vec_buf
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();

        cache.insert(key, vec);
    }

    Ok(cache)
}
