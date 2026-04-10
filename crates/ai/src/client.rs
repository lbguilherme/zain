//! Cliente unificado que despacha chamadas para o provider apropriado
//! com base no prefixo do nome do modelo.
//!
//! Todos os métodos recebem o modelo no formato `"provider/modelo"`.
//! Por exemplo: `"ollama/qwen3:8b"`, `"whisper/whisper-1"`.

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::chat::{ChatMessage, ChatResponse};
use crate::ollama::OllamaClient;
use crate::whisper::WhisperClient;

enum Provider {
    Ollama(OllamaClient),
    Whisper(WhisperClient),
}

/// Builder para configurar os providers disponíveis no [`Client`].
#[derive(Default)]
pub struct ClientBuilder {
    providers: HashMap<String, Provider>,
}

impl ClientBuilder {
    /// Registra o provider `ollama` com a URL base informada.
    pub fn ollama(mut self, base_url: &str) -> Self {
        self.providers.insert(
            "ollama".into(),
            Provider::Ollama(OllamaClient::new(base_url)),
        );
        self
    }

    /// Registra o provider `whisper` (API compatível com OpenAI) com a
    /// URL base informada.
    pub fn whisper(mut self, base_url: &str) -> Self {
        self.providers.insert(
            "whisper".into(),
            Provider::Whisper(WhisperClient::new(base_url)),
        );
        self
    }

    pub fn build(self) -> Client {
        Client {
            providers: self.providers,
        }
    }
}

/// Cliente único para chamar qualquer provider de IA. Roteia chamadas
/// pelo prefixo do nome do modelo (ex.: `"ollama/qwen3:8b"`).
pub struct Client {
    providers: HashMap<String, Provider>,
}

impl Client {
    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }

    fn resolve<'a>(&'a self, qualified_model: &'a str) -> Result<(&'a Provider, &'a str)> {
        let (name, model) = qualified_model.split_once('/').with_context(|| {
            format!("modelo '{qualified_model}' deve estar no formato 'provider/modelo'")
        })?;
        let provider = self
            .providers
            .get(name)
            .with_context(|| format!("provider '{name}' não configurado"))?;
        Ok((provider, model))
    }

    /// Chat com tool calls. O `model` deve ser qualificado pelo provider,
    /// ex.: `"ollama/qwen3:8b"`.
    pub async fn chat(
        &self,
        model: &str,
        messages: &[ChatMessage],
        tools: &[Value],
    ) -> Result<ChatResponse> {
        let (provider, model) = self.resolve(model)?;
        match provider {
            Provider::Ollama(c) => c.chat(model, messages, tools).await,
            Provider::Whisper(_) => bail!("provider 'whisper' não suporta chat"),
        }
    }

    /// Gera embeddings para uma lista de textos. `model` deve ser
    /// qualificado pelo provider, ex.: `"ollama/qwen3-embedding:4b"`.
    pub async fn embed_many(
        &self,
        model: &str,
        texts: &[String],
        cache_dir: Option<&PathBuf>,
    ) -> Result<Vec<Vec<f32>>> {
        let (provider, model) = self.resolve(model)?;
        match provider {
            Provider::Ollama(c) => c.embed_many(model, texts, cache_dir).await,
            Provider::Whisper(_) => bail!("provider 'whisper' não suporta embedding"),
        }
    }

    /// Transcreve áudio. `model` deve ser qualificado pelo provider,
    /// ex.: `"whisper/whisper-1"`.
    pub async fn transcribe(
        &self,
        model: &str,
        audio_bytes: Vec<u8>,
        file_name: &str,
        mime_type: &str,
    ) -> Result<String> {
        let (provider, model) = self.resolve(model)?;
        match provider {
            Provider::Whisper(c) => c.transcribe(model, audio_bytes, file_name, mime_type).await,
            Provider::Ollama(_) => bail!("provider 'ollama' não suporta transcrição"),
        }
    }
}
