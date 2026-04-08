use chrono::{DateTime, Utc};
use cubos_sql::sql;
use deadpool_postgres::Pool;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::llm::{ChatMessage, OllamaClient};
use crate::states::{self, ConversationMessage};
use crate::tools::{self, ToolResult};

#[derive(Debug, Clone)]
pub struct ClientRow {
    pub id: Uuid,
    pub chat_id: String,
    pub phone: Option<String>,
    pub name: Option<String>,
    pub state: String,
    pub state_props: Value,
    pub memory: Value,
    pub last_whatsapp_message_processed_at: Option<DateTime<Utc>>,
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
    let row = sql!(
        pool,
        "UPDATE zain.clients
         SET needs_processing = false, updated_at = now()
         WHERE id = (
             SELECT id FROM zain.clients
             WHERE needs_processing = true
             ORDER BY updated_at ASC
             FOR UPDATE SKIP LOCKED
             LIMIT 1
         )
         RETURNING id, chat_id, phone, name, state, state_props, memory,
                   last_whatsapp_message_processed_at"
    )
    .fetch_optional()
    .await?;

    Ok(row.map(|r| ClientRow {
        id: r.id,
        chat_id: r.chat_id,
        phone: r.phone,
        name: r.name,
        state: r.state,
        state_props: r.state_props,
        memory: r.memory,
        last_whatsapp_message_processed_at: r.last_whatsapp_message_processed_at,
    }))
}

// ── Process ────────────────────────────────────────────────────────────

pub async fn process_client(
    pool: &Pool,
    ollama: &OllamaClient,
    mut client: ClientRow,
) -> anyhow::Result<()> {
    loop {
        let (history, max_ts) = fetch_history(pool, &client.chat_id).await?;

        // Se não há mensagens novas desde o último processamento, nada a fazer
        let has_new = match (max_ts, client.last_whatsapp_message_processed_at) {
            (Some(latest), Some(processed)) => latest > processed,
            (Some(_), None) => true,
            _ => false,
        };

        if !has_new {
            tracing::debug!(client_id = %client.id, "Sem mensagens novas, pulando");
            break;
        }

        // Extrair apenas as novas mensagens para informar o LLM
        let new_messages: Vec<&ConversationMessage> = history
            .iter()
            .filter(|m| {
                if let (Some(ts), Some(processed)) =
                    (m.timestamp, client.last_whatsapp_message_processed_at)
                {
                    ts > processed
                } else {
                    // Se nunca processou, todas são novas
                    client.last_whatsapp_message_processed_at.is_none()
                }
            })
            .collect();

        let new_count = new_messages.len();
        let new_summary: String = new_messages
            .iter()
            .filter(|m| !m.from_me)
            .map(|m| m.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        let exec_id = create_execution(pool, &client).await?;

        let result = run_workflow(pool, ollama, &client, &history, new_count, &new_summary).await;

        match result {
            Ok((final_state, llm_log)) => {
                complete_execution(pool, exec_id, &final_state, &llm_log).await?;
                client.state = final_state;
                if let Some(ts) = max_ts {
                    update_last_processed(pool, client.id, ts).await?;
                    client.last_whatsapp_message_processed_at = Some(ts);
                }
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

async fn run_workflow(
    pool: &Pool,
    ollama: &OllamaClient,
    client: &ClientRow,
    history: &[ConversationMessage],
    new_message_count: usize,
    new_messages_summary: &str,
) -> anyhow::Result<(String, Vec<ChatMessage>)> {
    let handler = states::get_handler(&client.state);

    // Montar tools: estado-específicas + send_whatsapp_message (global)
    let mut state_tools = handler.tool_definitions();
    state_tools.push(tools::send_whatsapp_message_tool());
    let tools_json: Vec<Value> = state_tools.iter().map(|t| t.to_ollama_json()).collect();

    // System prompt com histórico embutido
    let system_prompt = handler.system_prompt(client, history);

    // User message informa o que motivou a execução
    let user_msg = format!(
        "O cliente enviou {new_message_count} nova(s) mensagem(ns):\n\n{new_messages_summary}\n\n\
         Responda ao cliente usando send_whatsapp_message.",
    );

    let mut messages = vec![
        ChatMessage::system(system_prompt),
        ChatMessage::user(user_msg),
    ];

    let mut current_state = client.state.clone();
    let mut state_props = client.state_props.clone();
    let mut memory = client.memory.clone();
    let mut sent_message = false;

    // Loop de interação com o LLM (tool calls iterativas)
    let max_iterations = 10;
    for _ in 0..max_iterations {
        let response = ollama.chat(&messages, &tools_json).await?;

        if let Some(ref tool_calls) = response.message.tool_calls {
            messages.push(ChatMessage::assistant_tool_calls(tool_calls));

            for call in tool_calls {
                let tool_name = &call.function.name;
                let tool_args = &call.function.arguments;

                tracing::info!(client_id = %client.id, tool_name, "Executando tool");

                if tool_name == "send_whatsapp_message" {
                    let msg_text = tool_args
                        .get("message")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if !msg_text.is_empty() {
                        write_outbox(pool, &client.chat_id, msg_text).await?;
                        sent_message = true;
                    }
                    messages.push(ChatMessage::tool(
                        json!({ "status": "ok", "mensagem_enviada": true }).to_string(),
                    ));
                } else {
                    let result =
                        handler.execute_tool(tool_name, tool_args, &mut state_props, &mut memory);

                    match result {
                        ToolResult::Ok(value) => {
                            messages.push(ChatMessage::tool(value.to_string()));
                        }
                        ToolResult::StateTransition {
                            new_state,
                            new_props,
                        } => {
                            current_state = new_state.clone();
                            state_props = new_props;
                            save_client_state(
                                pool,
                                client.id,
                                &current_state,
                                &state_props,
                                &memory,
                            )
                            .await?;
                            messages.push(ChatMessage::tool(format!(
                                "Transição para estado {new_state} realizada com sucesso."
                            )));
                        }
                    }
                }
            }
        } else {
            // LLM respondeu com texto sem chamar tools
            if !sent_message {
                // Nunca chamou send_whatsapp_message — orientar a usar a tool
                tracing::warn!(
                    client_id = %client.id,
                    "LLM respondeu texto sem usar send_whatsapp_message, re-orientando"
                );
                messages.push(ChatMessage::assistant(response.message.content.clone()));
                messages.push(ChatMessage::user(
                    "Você não pode responder com texto diretamente. \
                     Use a ferramenta send_whatsapp_message para enviar sua resposta ao cliente."
                        .into(),
                ));
                continue;
            }
            break;
        }
    }

    // Salvar estado final
    save_client_state(pool, client.id, &current_state, &state_props, &memory).await?;

    Ok((current_state, messages))
}

// ── Helpers ────────────────────────────────────────────────────────────

async fn create_execution(pool: &Pool, client: &ClientRow) -> anyhow::Result<Uuid> {
    let client_id = client.id;
    let state_before = &client.state;
    let trigger_type = "message";

    let id = sql!(
        pool,
        "INSERT INTO zain.executions (client_id, state_before, trigger_type)
         VALUES ($client_id, $state_before, $trigger_type)
         RETURNING id"
    )
    .fetch_value()
    .await?;

    Ok(id)
}

async fn complete_execution(
    pool: &Pool,
    exec_id: Uuid,
    state_after: &str,
    llm_messages: &[ChatMessage],
) -> anyhow::Result<()> {
    let state_after = Some(state_after);
    let llm_json: Option<Value> = Some(serde_json::to_value(llm_messages)?);

    sql!(
        pool,
        "UPDATE zain.executions
         SET status = 'completed', state_after = $state_after,
             llm_messages = $llm_json, finished_at = now()
         WHERE id = $exec_id"
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

async fn save_client_state(
    pool: &Pool,
    client_id: Uuid,
    state: &str,
    state_props: &Value,
    memory: &Value,
) -> anyhow::Result<()> {
    let state_props = state_props.clone();
    let memory = memory.clone();

    sql!(
        pool,
        "UPDATE zain.clients
         SET state = $state, state_props = $state_props, memory = $memory, updated_at = now()
         WHERE id = $client_id"
    )
    .execute()
    .await?;

    Ok(())
}

async fn fetch_history(
    pool: &Pool,
    chat_id: &str,
) -> anyhow::Result<(Vec<ConversationMessage>, Option<DateTime<Utc>>)> {
    let rows = sql!(
        pool,
        "SELECT from_me, text_body, \"timestamp\"
         FROM whatsapp.messages
         WHERE chat_id = $chat_id
         ORDER BY \"timestamp\" DESC
         LIMIT 60"
    )
    .fetch_all()
    .await?;

    let mut messages = Vec::new();
    let mut total_chars = 0usize;
    let max_ts: Option<DateTime<Utc>> = rows.first().map(|r| r.timestamp);

    for row in &rows {
        let text: String = row.text_body.clone().unwrap_or_default();
        total_chars += text.len();
        if total_chars > 10_000 && !messages.is_empty() {
            break;
        }
        messages.push(ConversationMessage {
            from_me: row.from_me,
            text,
            timestamp: Some(row.timestamp),
        });
    }

    messages.reverse();
    Ok((messages, max_ts))
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

async fn write_outbox(pool: &Pool, chat_id: &str, text: &str) -> anyhow::Result<()> {
    let content = json!({ "body": text });
    let content_type = "text";

    sql!(
        pool,
        "INSERT INTO whatsapp.outbox (chat_id, content_type, content)
         VALUES ($chat_id, $content_type, $content)"
    )
    .execute()
    .await?;

    Ok(())
}
