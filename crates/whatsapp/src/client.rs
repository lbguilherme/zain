use reqwest::Client;
use serde::Deserialize;

use crate::types::{ChatsList, MessagesList};

pub struct WhapiClient {
    http: Client,
    base_url: String,
    token: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct SendMessageResponse {
    #[serde(default)]
    sent: bool,
    #[serde(default)]
    message: Option<SendMessageData>,
}

#[derive(Debug, Deserialize)]
struct SendMessageData {
    #[serde(default)]
    id: Option<String>,
}

impl WhapiClient {
    pub fn new(base_url: &str, token: &str) -> Self {
        Self {
            http: Client::new(),
            base_url: base_url.trim_end_matches('/').to_owned(),
            token: token.to_owned(),
        }
    }

    /// Lista chats com paginação. `count` máximo 500.
    pub async fn get_chats(&self, offset: i32, count: i32) -> anyhow::Result<ChatsList> {
        let resp = self
            .http
            .get(format!("{}/chats", self.base_url))
            .bearer_auth(&self.token)
            .query(&[("offset", offset), ("count", count)])
            .send()
            .await?
            .error_for_status()?
            .json::<ChatsList>()
            .await?;
        Ok(resp)
    }

    /// Lista mensagens de um chat específico com paginação.
    pub async fn get_messages(
        &self,
        chat_id: &str,
        offset: i32,
        count: i32,
    ) -> anyhow::Result<MessagesList> {
        let resp = self
            .http
            .get(format!("{}/messages/list/{}", self.base_url, chat_id))
            .bearer_auth(&self.token)
            .query(&[("offset", offset), ("count", count)])
            .send()
            .await?
            .error_for_status()?
            .json::<MessagesList>()
            .await?;
        Ok(resp)
    }

    /// Lista mensagens de um chat a partir de um timestamp (unix seconds).
    pub async fn get_messages_since(
        &self,
        chat_id: &str,
        time_from: i64,
        offset: i32,
        count: i32,
    ) -> anyhow::Result<MessagesList> {
        let resp = self
            .http
            .get(format!("{}/messages/list/{}", self.base_url, chat_id))
            .bearer_auth(&self.token)
            .query(&[
                ("offset", offset as i64),
                ("count", count as i64),
                ("time_from", time_from),
            ])
            .query(&[("sort", "asc")])
            .send()
            .await?
            .error_for_status()?
            .json::<MessagesList>()
            .await?;
        Ok(resp)
    }

    /// Envia uma mensagem de texto para um chat. Retorna o ID da mensagem enviada.
    pub async fn send_text(&self, chat_id: &str, body: &str) -> anyhow::Result<String> {
        let payload = serde_json::json!({
            "to": chat_id,
            "body": body,
        });

        let resp = self
            .http
            .post(format!("{}/messages/text", self.base_url))
            .bearer_auth(&self.token)
            .json(&payload)
            .send()
            .await?
            .error_for_status()?
            .json::<SendMessageResponse>()
            .await?;

        let msg_id = resp.message.and_then(|m| m.id).unwrap_or_default();

        Ok(msg_id)
    }

    /// Envia um documento para um chat. `media` é uma data URL no
    /// formato `data:{mime};base64,{data}`. `filename` é o nome exibido
    /// no chat e `caption` vira a legenda opcional. Retorna o ID da
    /// mensagem enviada.
    pub async fn send_document(
        &self,
        chat_id: &str,
        media: &str,
        filename: Option<&str>,
        caption: Option<&str>,
    ) -> anyhow::Result<String> {
        let mut payload = serde_json::json!({
            "to": chat_id,
            "media": media,
        });
        if let Some(name) = filename {
            payload["filename"] = serde_json::Value::String(name.to_owned());
        }
        if let Some(c) = caption {
            payload["caption"] = serde_json::Value::String(c.to_owned());
        }

        let resp = self
            .http
            .post(format!("{}/messages/document", self.base_url))
            .bearer_auth(&self.token)
            .json(&payload)
            .send()
            .await?
            .error_for_status()?
            .json::<SendMessageResponse>()
            .await?;

        let msg_id = resp.message.and_then(|m| m.id).unwrap_or_default();

        Ok(msg_id)
    }
}
