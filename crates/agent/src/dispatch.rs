use std::sync::Arc;

use ai::ChatMessage;
use chrono::{DateTime, Utc};
use cubos_sql::sql;
use deadpool_postgres::Pool;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::history::{self, ConversationMessage};
use crate::workflow;

/// Conjunto de modelos usados pelo agent. Todos são qualificados pelo
/// provider (`ollama/`, `whisper/`, `gemini/`, ...) e vêm do ambiente.
#[derive(Debug, Clone)]
pub struct Models {
    pub chat: String,
    pub transcription: String,
    /// Deve bater com o modelo usado para popular `cnae.*.embedding`.
    pub embedding: String,
}

impl Models {
    pub fn from_env() -> anyhow::Result<Self> {
        use anyhow::Context;
        Ok(Self {
            chat: std::env::var("CHAT_MODEL").context("CHAT_MODEL não definido")?,
            transcription: std::env::var("TRANSCRIPTION_MODEL")
                .context("TRANSCRIPTION_MODEL não definido")?,
            embedding: std::env::var("EMBEDDING_MODEL").context("EMBEDDING_MODEL não definido")?,
        })
    }
}

pub enum WorkflowOutcome {
    Completed { llm_log: Vec<ChatMessage> },
    Restart,
}

#[derive(Debug, Clone)]
pub struct ClientRow {
    pub id: Uuid,
    pub chat_id: String,
    pub phone: Option<String>,
    pub name: Option<String>,
    pub props: Value,
    pub memory: Value,
    pub last_whatsapp_message_processed_at: Option<DateTime<Utc>>,
    pub history_starts_at: Option<DateTime<Utc>>,
    pub exec_id: Uuid,
}

// ── Recovery ───────────────────────────────────────────────────────────

pub async fn recover_crashed(pool: &Pool) -> anyhow::Result<()> {
    let crashed = sql!(
        pool,
        "UPDATE zain.executions
         SET status = 'crashed', finished_at = now(), error = 'service restarted'
         WHERE status = 'running'
         RETURNING client_id"
    )
    .fetch_all()
    .await?;

    for row in &crashed {
        let client_id: Uuid = row.client_id;
        sql!(
            pool,
            "UPDATE zain.clients SET needs_processing = true WHERE id = $client_id"
        )
        .execute()
        .await?;
    }

    if !crashed.is_empty() {
        tracing::warn!(count = crashed.len(), "Execuções crashadas recuperadas");
    }

    Ok(())
}

// ── Claim ──────────────────────────────────────────────────────────────

pub async fn claim_next_client(pool: &Pool) -> anyhow::Result<Option<ClientRow>> {
    // Claim + criação de execução em transação para evitar race condition.
    // A execução 'running' serve como lock — o claim exclui clients que já têm uma.
    let mut db = pool.get().await?;
    let tx = db.transaction().await?;

    let row = sql!(
        &tx,
        "UPDATE zain.clients
         SET needs_processing = false, updated_at = now()
         WHERE id = (
             SELECT c.id FROM zain.clients c
             WHERE c.needs_processing = true
               AND NOT EXISTS (
                   SELECT 1 FROM zain.executions e
                   WHERE e.client_id = c.id AND e.status = 'running'
               )
             ORDER BY c.updated_at ASC
             FOR UPDATE SKIP LOCKED
             LIMIT 1
         )
         RETURNING id, chat_id, phone, name, props, memory,
                   last_whatsapp_message_processed_at, history_starts_at"
    )
    .fetch_optional()
    .await?;

    let Some(r) = row else {
        // Nenhum client para processar — não precisa commitar
        return Ok(None);
    };

    let client_id = r.id;
    let exec_id: Uuid = sql!(
        &tx,
        "INSERT INTO zain.executions (client_id, trigger_type)
         VALUES ($client_id, 'message')
         RETURNING id"
    )
    .fetch_value()
    .await?;

    tx.commit().await?;

    Ok(Some(ClientRow {
        id: r.id,
        chat_id: r.chat_id,
        phone: r.phone,
        name: r.name,
        props: r.props,
        memory: r.memory,
        last_whatsapp_message_processed_at: r.last_whatsapp_message_processed_at,
        history_starts_at: r.history_starts_at,
        exec_id,
    }))
}

// ── Process ────────────────────────────────────────────────────────────

pub async fn process_client(
    pool: &Pool,
    ai: &Arc<ai::Client>,
    models: &Arc<Models>,
    mut client: ClientRow,
) -> anyhow::Result<()> {
    let mut exec_id = client.exec_id;

    loop {
        let (history_msgs, max_ts) =
            history::fetch_history(pool, &client.chat_id, client.history_starts_at, ai, models)
                .await?;

        // Se não há mensagens novas desde o último processamento, nada a fazer
        let has_new = match (max_ts, client.last_whatsapp_message_processed_at) {
            (Some(latest), Some(processed)) => latest > processed,
            (Some(_), None) => true,
            _ => false,
        };

        if !has_new {
            tracing::debug!(client_id = %client.id, "Sem mensagens novas, pulando");
            complete_execution(pool, exec_id, &[]).await?;
            break;
        }

        // Extrair apenas as novas mensagens recebidas (from_me=false)
        let new_messages: Vec<&ConversationMessage> = history_msgs
            .iter()
            .filter(|m| {
                if m.from_me {
                    return false;
                }
                if let (Some(ts), Some(processed)) =
                    (m.timestamp, client.last_whatsapp_message_processed_at)
                {
                    ts > processed
                } else {
                    client.last_whatsapp_message_processed_at.is_none()
                }
            })
            .collect();

        // Comando /reset: resetar client e limpar histórico
        if let Some(reset_msg) = new_messages.iter().find(|m| m.text.trim() == "/reset") {
            tracing::info!(client_id = %client.id, "Comando /reset recebido, resetando client");
            let reset_ts = reset_msg.timestamp;
            let client_id = client.id;
            sql!(
                pool,
                "UPDATE zain.clients
                 SET props = '{}', memory = '{}',
                     updated_at = now(),
                     last_whatsapp_message_processed_at = $reset_ts,
                     history_starts_at = $reset_ts
                 WHERE id = $client_id"
            )
            .execute()
            .await?;
            client.props = json!({});
            client.memory = json!({});
            client.last_whatsapp_message_processed_at = reset_ts;
            client.history_starts_at = reset_ts;
            exec_id = rotate_execution(pool, exec_id, "cancelled", None).await?;
            continue;
        }

        let new_count = new_messages.len();
        let new_summary: String = new_messages
            .iter()
            .map(|m| {
                let mut line = m.text.clone();
                for img in &m.images {
                    line.push_str(&format!(" <attachment type=\"image\" id=\"{}\"/>", img.id));
                }
                line
            })
            .collect::<Vec<_>>()
            .join("\n");

        let result = workflow::run_workflow(
            pool,
            ai,
            models,
            &client,
            &history_msgs,
            new_count,
            &new_summary,
            max_ts,
            exec_id,
        )
        .await;

        match result {
            Ok(WorkflowOutcome::Restart) => {
                tracing::info!(client_id = %client.id, "Workflow reiniciado por novas mensagens");
                exec_id = rotate_execution(pool, exec_id, "cancelled", None).await?;
                continue;
            }
            Ok(WorkflowOutcome::Completed { llm_log }) => {
                if let Some(ts) = max_ts {
                    update_last_processed(pool, client.id, ts).await?;
                    client.last_whatsapp_message_processed_at = Some(ts);
                }
                exec_id = rotate_execution(pool, exec_id, "completed", Some(&llm_log)).await?;
            }
            Err(e) => {
                fail_execution(pool, exec_id, &e.to_string()).await?;
                let client_id = client.id;
                sql!(
                    pool,
                    "UPDATE zain.clients SET needs_processing = true WHERE id = $client_id"
                )
                .execute()
                .await?;
                return Err(e);
            }
        }
    }

    Ok(())
}

// ── Execution tracking ─────────────────────────────────────────────────

/// Fecha a execução atual e abre uma nova atomicamente,
/// garantindo que sempre existe uma execução 'running' para o client.
async fn rotate_execution(
    pool: &Pool,
    old_exec_id: Uuid,
    close_status: &str,
    llm_messages: Option<&[ChatMessage]>,
) -> anyhow::Result<Uuid> {
    let llm_json: Value = llm_messages
        .map(serde_json::to_value)
        .transpose()?
        .unwrap_or(Value::Null);

    let mut db = pool.get().await?;
    let tx = db.transaction().await?;

    let client_id: Uuid = sql!(
        &tx,
        "UPDATE zain.executions
         SET status = $close_status,
             llm_messages = COALESCE($llm_json, llm_messages),
             finished_at = now()
         WHERE id = $old_exec_id
         RETURNING client_id"
    )
    .fetch_value()
    .await?;

    let new_id: Uuid = sql!(
        &tx,
        "INSERT INTO zain.executions (client_id, trigger_type)
         VALUES ($client_id, 'message')
         RETURNING id"
    )
    .fetch_value()
    .await?;

    tx.commit().await?;
    Ok(new_id)
}

async fn complete_execution(
    pool: &Pool,
    exec_id: Uuid,
    llm_messages: &[ChatMessage],
) -> anyhow::Result<()> {
    let llm_json: Option<Value> = Some(serde_json::to_value(llm_messages)?);

    sql!(
        pool,
        "UPDATE zain.executions
         SET status = 'completed', llm_messages = $llm_json, finished_at = now()
         WHERE id = $exec_id"
    )
    .execute()
    .await?;

    Ok(())
}

pub async fn update_execution_messages(
    pool: &Pool,
    exec_id: Uuid,
    messages: &[ChatMessage],
) -> anyhow::Result<()> {
    let llm_json: Option<Value> = Some(serde_json::to_value(messages)?);

    sql!(
        pool,
        "UPDATE zain.executions SET llm_messages = $llm_json WHERE id = $exec_id"
    )
    .execute()
    .await?;

    Ok(())
}

async fn fail_execution(pool: &Pool, exec_id: Uuid, error: &str) -> anyhow::Result<()> {
    let error = Some(error);

    sql!(
        pool,
        "UPDATE zain.executions
         SET status = 'failed', error = $error, finished_at = now()
         WHERE id = $exec_id"
    )
    .execute()
    .await?;

    Ok(())
}

pub async fn save_client_props(
    pool: &Pool,
    client_id: Uuid,
    props: &Value,
    memory: &Value,
) -> anyhow::Result<()> {
    let props = props.clone();
    let memory = memory.clone();

    sql!(
        pool,
        "UPDATE zain.clients
         SET props = $props, memory = $memory, updated_at = now()
         WHERE id = $client_id"
    )
    .execute()
    .await?;

    Ok(())
}

async fn update_last_processed(
    pool: &Pool,
    client_id: Uuid,
    ts: DateTime<Utc>,
) -> anyhow::Result<()> {
    let ts = Some(ts);
    sql!(
        pool,
        "UPDATE zain.clients
         SET last_whatsapp_message_processed_at = $ts, updated_at = now()
         WHERE id = $client_id"
    )
    .execute()
    .await?;

    Ok(())
}
