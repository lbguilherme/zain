use cubos_sql::sql;
use deadpool_postgres::Pool;
use serde_json::{Value, json};

use crate::llm::{ChatMessage, OllamaClient};
use crate::states::{self, ConversationMessage};
use crate::tools::{self, ToolResult};

#[derive(Debug, Clone)]
pub struct ClientRow {
    pub id: i64,
    pub chat_id: String,
    pub phone: Option<String>,
    pub name: Option<String>,
    pub state: String,
    pub state_props: Value,
    pub memory: Value,
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
        let client_id: i64 = row.client_id;
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
         RETURNING id, chat_id, phone, name, state, state_props, memory"
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
    }))
}

// ── Process ────────────────────────────────────────────────────────────

pub async fn process_client(
    pool: &Pool,
    ollama: &OllamaClient,
    client: ClientRow,
) -> anyhow::Result<()> {
    let exec_id = create_execution(pool, &client).await?;

    let result = run_workflow(pool, ollama, &client).await;

    match result {
        Ok((final_state, llm_log)) => {
            complete_execution(pool, exec_id, &final_state, &llm_log).await?;
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

    Ok(())
}

async fn run_workflow(
    pool: &Pool,
    ollama: &OllamaClient,
    client: &ClientRow,
) -> anyhow::Result<(String, Vec<ChatMessage>)> {
    let handler = states::get_handler(&client.state);

    // TODO: buscar webhooks não processados e extrair mensagens
    let history: Vec<ConversationMessage> = vec![];

    // Montar tools: estado-específicas + send_whatsapp_message (global)
    let mut state_tools = handler.tool_definitions();
    state_tools.push(tools::send_whatsapp_message_tool());
    let tools_json: Vec<Value> = state_tools.iter().map(|t| t.to_ollama_json()).collect();

    // System prompt com histórico embutido
    let system_prompt = handler.system_prompt(client, &history);

    // Única mensagem do "user" é um trigger para o LLM agir
    let mut messages = vec![
        ChatMessage::system(system_prompt),
        ChatMessage::user(
            "Processe as mensagens pendentes do cliente e responda usando send_whatsapp_message."
                .into(),
        ),
    ];

    let mut current_state = client.state.clone();
    let mut state_props = client.state_props.clone();
    let mut memory = client.memory.clone();

    // Loop de interação com o LLM (tool calls iterativas)
    let max_iterations = 10;
    for _ in 0..max_iterations {
        let response = ollama.chat(&messages, &tools_json).await?;

        if let Some(ref tool_calls) = response.message.tool_calls {
            messages.push(ChatMessage::assistant_tool_calls(tool_calls));

            for call in tool_calls {
                let tool_name = &call.function.name;
                let tool_args = &call.function.arguments;

                tracing::info!(client_id = client.id, tool_name, "Executando tool");

                if tool_name == "send_whatsapp_message" {
                    let msg_text = tool_args
                        .get("message")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if !msg_text.is_empty() {
                        write_outbox(pool, &client.chat_id, msg_text).await?;
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
            tracing::debug!(
                client_id = client.id,
                "LLM respondeu com texto cru (ignorado)"
            );
            break;
        }
    }

    // Salvar estado final
    save_client_state(pool, client.id, &current_state, &state_props, &memory).await?;

    Ok((current_state, messages))
}

// ── Helpers ────────────────────────────────────────────────────────────

async fn create_execution(pool: &Pool, client: &ClientRow) -> anyhow::Result<i64> {
    let client_id = client.id;
    let state_before = &client.state;
    let trigger_type = "message";

    let row = sql!(
        pool,
        "INSERT INTO zain.executions (client_id, state_before, trigger_type)
         VALUES ($client_id, $state_before, $trigger_type)
         RETURNING id"
    )
    .fetch_one()
    .await?;

    Ok(row.id)
}

async fn complete_execution(
    pool: &Pool,
    exec_id: i64,
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

async fn fail_execution(pool: &Pool, exec_id: i64, error: &str) -> anyhow::Result<()> {
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
    client_id: i64,
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
