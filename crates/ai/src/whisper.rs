//! Cliente para transcrição de áudio via API compatível com OpenAI Whisper.

use anyhow::Result;
use serde_json::Value;

pub struct WhisperClient {
    http: reqwest::Client,
    base_url: String,
}

impl WhisperClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(300))
                .build()
                .unwrap(),
            base_url: base_url.to_owned(),
        }
    }

    /// Transcreve um áudio enviando os bytes para a API (multipart).
    ///
    /// `file_name` e `mime_type` descrevem o arquivo enviado (ex.:
    /// `"audio.ogg"`, `"audio/ogg"`).
    pub async fn transcribe(
        &self,
        model: &str,
        audio_bytes: Vec<u8>,
        file_name: &str,
        mime_type: &str,
    ) -> Result<String> {
        let part = reqwest::multipart::Part::bytes(audio_bytes)
            .file_name(file_name.to_owned())
            .mime_str(mime_type)?;
        let form = reqwest::multipart::Form::new()
            .text("model", model.to_owned())
            .part("file", part);

        let resp: Value = self
            .http
            .post(&self.base_url)
            .multipart(form)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let transcription = resp
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned();

        Ok(transcription)
    }
}
