use std::sync::Arc;
use std::time::Duration;

use cubos_sql::sql;
use deadpool_postgres::Pool;
use tokio::sync::Notify;
use tokio_postgres::{AsyncMessage, NoTls};

use crate::client::WhapiClient;

pub async fn outbox_loop(pool: &Pool, api: &WhapiClient, database_url: &str) -> anyhow::Result<()> {
    let notify = Arc::new(Notify::new());

    // Task dedicada de LISTEN em uma conexão fora do pool (LISTEN prende a
    // conexão). Reconecta em loop com backoff se a conexão cair.
    let listen_notify = notify.clone();
    let listen_url = database_url.to_owned();
    tokio::spawn(async move {
        loop {
            if let Err(e) = listen_task(&listen_url, &listen_notify).await {
                tracing::error!("Listen task caiu: {e:#}. Reconectando em 5s...");
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });

    loop {
        if let Err(e) = process_outbox_batch(pool, api).await {
            tracing::error!("Erro no outbox: {e:#}");
        }

        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(20)) => {}
            _ = notify.notified() => {}
        }
    }
}

async fn listen_task(database_url: &str, notify: &Arc<Notify>) -> anyhow::Result<()> {
    let (client, connection) = tokio_postgres::connect(database_url, NoTls).await?;

    // Canal para repassar notifications do driver da conexão para o loop.
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<()>();

    // Sub-task que dirige a Connection dedicada e extrai AsyncMessages.
    let driver = tokio::spawn(async move {
        tokio::pin!(connection);
        loop {
            let msg = std::future::poll_fn(|cx| connection.as_mut().poll_message(cx)).await;
            match msg {
                Some(Ok(AsyncMessage::Notification(_))) => {
                    if tx.send(()).is_err() {
                        break;
                    }
                }
                Some(Ok(_)) => {}
                Some(Err(e)) => {
                    tracing::error!("Erro na conexão LISTEN: {e}");
                    break;
                }
                None => break,
            }
        }
    });

    client.batch_execute("LISTEN whatsapp_outbox").await?;

    while rx.recv().await.is_some() {
        notify.notify_one();
    }

    driver.abort();
    anyhow::bail!("conexão LISTEN encerrou");
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
