use cubos_sql::sql;
use deadpool_postgres::Pool;

use crate::client::WhapiClient;

pub async fn outbox_loop(pool: &Pool, api: &WhapiClient) -> anyhow::Result<()> {
    loop {
        if let Err(e) = process_outbox_batch(pool, api).await {
            tracing::error!("Erro no outbox: {e:#}");
        }

        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
}

async fn process_outbox_batch(pool: &Pool, api: &WhapiClient) -> anyhow::Result<()> {
    // Claim batch de mensagens pendentes com lock
    let messages = sql!(
        pool,
        "UPDATE whatsapp.outbox
         SET status = 'sending', attempts = attempts + 1
         WHERE id IN (
             SELECT id FROM whatsapp.outbox
             WHERE status IN ('pending', 'failed')
             AND attempts < 5
             ORDER BY created_at ASC
             FOR UPDATE SKIP LOCKED
             LIMIT 10
         )
         RETURNING id, chat_id, content_type, content"
    )
    .fetch_all()
    .await?;

    for msg in &messages {
        let msg_id: uuid::Uuid = msg.id;
        let chat_id: &str = &msg.chat_id;
        let content: &serde_json::Value = &msg.content;

        // Por enquanto só suporta texto
        let body = content.get("body").and_then(|v| v.as_str()).unwrap_or("");

        if body.is_empty() {
            tracing::warn!(%msg_id, "Mensagem do outbox sem body, pulando");
            mark_failed(pool, msg_id, "body vazio").await?;
            continue;
        }

        match api.send_text(chat_id, body).await {
            Ok(sent_id) => {
                tracing::info!(%msg_id, %chat_id, "Mensagem enviada");
                mark_sent(pool, msg_id, &sent_id).await?;
            }
            Err(e) => {
                tracing::error!(%msg_id, %chat_id, "Erro enviando mensagem: {e:#}");
                mark_failed(pool, msg_id, &e.to_string()).await?;
            }
        }
    }

    Ok(())
}

async fn mark_sent(pool: &Pool, msg_id: uuid::Uuid, sent_message_id: &str) -> anyhow::Result<()> {
    let sent_message_id = Some(sent_message_id);

    sql!(
        pool,
        "UPDATE whatsapp.outbox
         SET status = 'sent', sent_message_id = $sent_message_id, sent_at = now()
         WHERE id = $msg_id"
    )
    .execute()
    .await?;
    Ok(())
}

async fn mark_failed(pool: &Pool, msg_id: uuid::Uuid, error: &str) -> anyhow::Result<()> {
    let error = Some(error);

    sql!(
        pool,
        "UPDATE whatsapp.outbox
         SET status = 'failed', last_error = $error
         WHERE id = $msg_id"
    )
    .execute()
    .await?;
    Ok(())
}
