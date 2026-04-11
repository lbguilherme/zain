use std::path::PathBuf;

use anyhow::{Context, Result};
use half::f16;

pub struct EmbeddingClient {
    ai: ai::Client,
    model: String,
    cache_dir: PathBuf,
}

impl EmbeddingClient {
    pub fn new(cache_dir: PathBuf) -> Result<Self> {
        let model = std::env::var("EMBEDDING_MODEL")
            .context("variável de ambiente EMBEDDING_MODEL não definida")?;
        Ok(Self {
            ai: ai::Client::from_env(),
            model,
            cache_dir,
        })
    }

    pub async fn embed_many(&self, texts: &[String]) -> Result<Vec<pgvector::HalfVector>> {
        let vecs = self
            .ai
            .embed_many(&self.model, texts, Some(&self.cache_dir))
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
