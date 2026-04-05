use reqwest::Client;
use serde::Deserialize;

use crate::types::{ChatsList, MessagesList};

pub struct WhapiClient {
    http: Client,
    base_url: String,
    token: String,
}

#[derive(Debug, Deserialize)]
struct SendMessageResponse {
    #[serde(default)]
    message_id: Option<String>,
    // whapi retorna "sent" com o id
    #[serde(default)]
    sent: Option<SendSent>,
}

#[derive(Debug, Deserialize)]
struct SendSent {
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

        let msg_id = resp
            .sent
            .and_then(|s| s.id)
            .or(resp.message_id)
            .unwrap_or_default();

        Ok(msg_id)
    }
}
