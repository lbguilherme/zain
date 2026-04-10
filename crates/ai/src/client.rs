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
use crate::gemini::GeminiClient;
use crate::ollama::OllamaClient;
use crate::whisper::WhisperClient;

enum Provider {
    Ollama(OllamaClient),
    Gemini(GeminiClient),
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

    /// Registra o provider `gemini` com a API key informada.
    pub fn gemini(mut self, api_key: &str) -> Self {
        self.providers.insert(
            "gemini".into(),
            Provider::Gemini(GeminiClient::new(api_key)),
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

    /// Constrói um [`Client`] lendo a configuração de cada provider
    /// diretamente do ambiente. Um provider só é registrado se a sua
    /// variável de ambiente correspondente existir:
    ///
    /// - `OLLAMA_URL`   → provider `ollama`
    /// - `GEMINI_API_KEY` → provider `gemini`
    /// - `WHISPER_URL`  → provider `whisper`
    ///
    /// Se nenhuma estiver setada, retorna um client vazio — qualquer
    /// chamada falha com "provider '…' não configurado".
    pub fn from_env() -> Self {
        let mut builder = ClientBuilder::default();

        if let Ok(url) = std::env::var("OLLAMA_URL") {
            tracing::info!(provider = "ollama", %url, "registering ai provider");
            builder = builder.ollama(&url);
        }
        if let Ok(key) = std::env::var("GEMINI_API_KEY") {
            tracing::info!(provider = "gemini", "registering ai provider");
            builder = builder.gemini(&key);
        }
        if let Ok(url) = std::env::var("WHISPER_URL") {
            tracing::info!(provider = "whisper", %url, "registering ai provider");
            builder = builder.whisper(&url);
        }

        builder.build()
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
            Provider::Gemini(c) => c.chat(model, messages, tools).await,
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
            Provider::Gemini(_) => bail!("provider 'gemini' não suporta embedding"),
            Provider::Whisper(_) => bail!("provider 'whisper' não suporta embedding"),
        }
    }

    /// Versão conveniente de [`Self::embed_many`] para um único texto.
    pub async fn embed(
        &self,
        model: &str,
        text: &str,
        cache_dir: Option<&PathBuf>,
    ) -> Result<Vec<f32>> {
        let mut vecs = self
            .embed_many(model, &[text.to_string()], cache_dir)
            .await?;
        vecs.pop()
            .ok_or_else(|| anyhow::anyhow!("embed_many retornou vazio"))
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
            Provider::Gemini(_) => bail!("provider 'gemini' não suporta transcrição"),
        }
    }
}
