use std::sync::Arc;

use ai::ChatMessage;
use chrono::{DateTime, Utc};
use cubos_sql::sql;
use deadpool_postgres::Pool;
use rpa::govbr::Nivel;
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
    pub cpf: Option<String>,
    pub cnpj: Option<String>,
    pub quer_abrir_mei: Option<bool>,
    pub cnae: Option<String>,
    /// Descrição da subclasse CNAE, vindo do join com `cnae.subclasses`.
    /// Não é uma coluna de `zain.clients` — derivada em `claim_next_client`.
    pub cnae_descricao: Option<String>,
    pub endereco: Option<String>,
    pub pagamento_solicitado_em: Option<DateTime<Utc>>,
    pub recusa_motivo: Option<String>,
    pub recusado_em: Option<DateTime<Utc>>,
    /// `true` quando `govbr_session IS NOT NULL` no cliente. Derivada.
    pub govbr_autenticado: bool,
    /// Nome retornado pelo perfil gov.br após login bem-sucedido.
    pub govbr_nome: Option<String>,
    /// Selo gov.br (bronze/prata/ouro) retornado pelo perfil.
    pub govbr_nivel: Option<Nivel>,
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

    // Passo 1: claim + lock atualiza needs_processing e pega o id.
    // O enriquecimento (join com cnae.subclasses, flag govbr) vem
    // num SELECT separado — o cubos_sql casa os tipos de colunas da
    // cláusula RETURNING, o que dificulta misturar joins aqui.
    let claimed = sql!(
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
         RETURNING id"
    )
    .fetch_optional()
    .await?;

    let Some(claimed) = claimed else {
        return Ok(None);
    };
    let claimed_id = claimed.id;

    let row = sql!(
        &tx,
        "SELECT c.id, c.chat_id, c.phone, c.name,
                c.cpf, c.cnpj, c.quer_abrir_mei, c.cnae, c.endereco,
                c.pagamento_solicitado_em, c.recusa_motivo, c.recusado_em,
                s.descricao AS cnae_descricao,
                (c.govbr_session IS NOT NULL) AS govbr_autenticado,
                c.govbr_nome, c.govbr_nivel,
                c.memory,
                c.last_whatsapp_message_processed_at, c.history_starts_at
         FROM zain.clients c
         LEFT JOIN cnae.subclasses s ON s.id = c.cnae
         WHERE c.id = $claimed_id"
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
        cpf: r.cpf,
        cnpj: r.cnpj,
        quer_abrir_mei: r.quer_abrir_mei,
        cnae: r.cnae,
        cnae_descricao: r.cnae_descricao,
        endereco: r.endereco,
        pagamento_solicitado_em: r.pagamento_solicitado_em,
        recusa_motivo: r.recusa_motivo,
        recusado_em: r.recusado_em,
        govbr_autenticado: r.govbr_autenticado,
        govbr_nome: r.govbr_nome,
        govbr_nivel: r.govbr_nivel,
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
            // LLM não foi chamado nessa execução — apaga a row em vez
            // de deixar uma execução vazia/ruído no histórico.
            delete_execution(pool, exec_id).await?;
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

        // Comando /reset: apaga a row inteira e recria uma nova com
        // apenas os campos de identidade (chat_id, phone, name), como se
        // o cliente tivesse acabado de chegar pela integração do
        // WhatsApp. Qualquer coisa acumulada (props, memory, govbr_*,
        // etc.) some junto — faz o reset continuar valendo à medida que
        // novas colunas "de estado" forem adicionadas.
        //
        // As únicas duas colunas não-identidade que preservamos na
        // reinserção são `last_whatsapp_message_processed_at` e
        // `history_starts_at = reset_ts`: sem esse corte a próxima
        // iteração do loop re-leria o próprio comando `/reset` como
        // mensagem nova e disparamos um loop infinito.
        if let Some(reset_msg) = new_messages.iter().find(|m| m.text.trim() == "/reset") {
            tracing::info!(client_id = %client.id, "Comando /reset recebido, recriando client");
            let reset_ts = reset_msg.timestamp;
            let old_id = client.id;
            let chat_id = client.chat_id.clone();
            let phone = client.phone.clone();
            let name = client.name.clone();

            let mut db = pool.get().await?;
            let tx = db.transaction().await?;

            // Remove todas as execuções (a `running` atual inclusive) —
            // a FK em zain.executions bloquearia o DELETE do client.
            sql!(&tx, "DELETE FROM zain.executions WHERE client_id = $old_id")
                .execute()
                .await?;

            sql!(&tx, "DELETE FROM zain.clients WHERE id = $old_id")
                .execute()
                .await?;

            let new_id: Uuid = sql!(
                &tx,
                "INSERT INTO zain.clients (
                    chat_id, phone, name,
                    last_whatsapp_message_processed_at,
                    history_starts_at
                 )
                 VALUES ($chat_id, $phone, $name, $reset_ts, $reset_ts)
                 RETURNING id"
            )
            .fetch_value()
            .await?;

            let new_exec_id: Uuid = sql!(
                &tx,
                "INSERT INTO zain.executions (client_id, trigger_type)
                 VALUES ($new_id, 'message')
                 RETURNING id"
            )
            .fetch_value()
            .await?;

            tx.commit().await?;

            client.id = new_id;
            client.cpf = None;
            client.cnpj = None;
            client.quer_abrir_mei = None;
            client.cnae = None;
            client.cnae_descricao = None;
            client.endereco = None;
            client.pagamento_solicitado_em = None;
            client.recusa_motivo = None;
            client.recusado_em = None;
            client.govbr_autenticado = false;
            client.memory = json!({});
            client.last_whatsapp_message_processed_at = reset_ts;
            client.history_starts_at = reset_ts;
            exec_id = new_exec_id;
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

async fn delete_execution(pool: &Pool, exec_id: Uuid) -> anyhow::Result<()> {
    sql!(pool, "DELETE FROM zain.executions WHERE id = $exec_id")
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

pub async fn save_client_memory(
    pool: &Pool,
    client_id: Uuid,
    memory: &Value,
) -> anyhow::Result<()> {
    let memory = memory.clone();

    sql!(
        pool,
        "UPDATE zain.clients
         SET memory = $memory, updated_at = now()
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
