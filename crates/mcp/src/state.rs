use std::sync::Arc;

use deadpool_postgres::Pool;

/// Modelos usados pelo servidor MCP. Qualificados pelo provider
/// (`ollama/`, `gemini/`, ...) e vêm do ambiente. Apenas os que as
/// tools efetivamente usam — `chat` é consumido por `abrir_empresa`
/// (LLM auxiliar do RPA) e `embedding` por `buscar_cnae`.
#[derive(Debug, Clone)]
pub struct Models {
    pub chat: String,
    /// Deve bater com o modelo usado para popular `cnae.*.embedding`.
    pub embedding: String,
}

impl Models {
    pub fn from_env() -> anyhow::Result<Self> {
        use anyhow::Context;
        Ok(Self {
            chat: std::env::var("CHAT_MODEL").context("CHAT_MODEL não definido")?,
            embedding: std::env::var("EMBEDDING_MODEL").context("EMBEDDING_MODEL não definido")?,
        })
    }
}

/// Estado compartilhado entre todas as chamadas de tools.
#[derive(Clone)]
pub struct AppState {
    pub pool: Pool,
    pub ai: Arc<ai::Client>,
    pub models: Arc<Models>,
}
