use std::path::PathBuf;

use anyhow::Result;
use half::f16;

const EMBEDDING_MODEL: &str = "ollama/qwen3-embedding:4b-q4_K_M";

pub struct EmbeddingClient {
    ai: ai::Client,
    cache_dir: PathBuf,
}

impl EmbeddingClient {
    pub fn new(cache_dir: PathBuf) -> Self {
        let base_url =
            std::env::var("OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434".to_string());
        Self {
            ai: ai::Client::builder().ollama(&base_url).build(),
            cache_dir,
        }
    }

    pub async fn embed_many(&self, texts: &[String]) -> Result<Vec<pgvector::HalfVector>> {
        let vecs = self
            .ai
            .embed_many(EMBEDDING_MODEL, texts, Some(&self.cache_dir))
            .await?;

        Ok(vecs
            .into_iter()
            .map(|v| {
                let half: Vec<f16> = v.into_iter().map(f16::from_f32).collect();
                pgvector::HalfVector::from(half)
            })
            .collect())
    }
}
