use chrono::{DateTime, Utc};
use cubos_sql::sql;
use deadpool_postgres::Pool;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::states::{self, ConversationMessage};
use crate::tools::{self, ToolDef, ToolResult};
use ai::ChatMessage;

/// Modelo usado para transcrever áudios recebidos. O prefixo indica o
/// provider dentro do [`ai::Client`].
const TRANSCRIPTION_MODEL: &str = "whisper/whisper-1";

enum WorkflowOutcome {
    Completed {
        final_state: String,
        llm_log: Vec<ChatMessage>,
    },
    Restart,
}

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
         RETURNING id, chat_id, phone, name, state, state_props, memory,
                   last_whatsapp_message_processed_at, history_starts_at"
    )
    .fetch_optional()
    .await?;

    let Some(r) = row else {
        // Nenhum client para processar — não precisa commitar
        return Ok(None);
    };

    let client_id = r.id;
    let state_before: String = r.state.clone();
    let exec_id: Uuid = sql!(
        &tx,
        "INSERT INTO zain.executions (client_id, state_before, trigger_type)
         VALUES ($client_id, $state_before, 'message')
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
        state: r.state,
        state_props: r.state_props,
        memory: r.memory,
        last_whatsapp_message_processed_at: r.last_whatsapp_message_processed_at,
        history_starts_at: r.history_starts_at,
        exec_id,
    }))
}

// ── Process ────────────────────────────────────────────────────────────

pub async fn process_client(
    pool: &Pool,
    ai: &ai::Client,
    chat_model: &str,
    mut client: ClientRow,
) -> anyhow::Result<()> {
    // exec_id começa com a execução criada atomicamente no claim
    let mut exec_id = client.exec_id;

    loop {
        let (history, max_ts) =
            fetch_history(pool, &client.chat_id, client.history_starts_at, ai).await?;

        // Se não há mensagens novas desde o último processamento, nada a fazer
        let has_new = match (max_ts, client.last_whatsapp_message_processed_at) {
            (Some(latest), Some(processed)) => latest > processed,
            (Some(_), None) => true,
            _ => false,
        };

        if !has_new {
            tracing::debug!(client_id = %client.id, "Sem mensagens novas, pulando");
            complete_execution(pool, exec_id, &client.state, &[]).await?;
            break;
        }

        // Extrair apenas as novas mensagens recebidas (from_me=false)
        let new_messages: Vec<&ConversationMessage> = history
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

        // Comando /reset: resetar client para estado inicial e limpar histórico
        if let Some(reset_msg) = new_messages.iter().find(|m| m.text.trim() == "/reset") {
            tracing::info!(client_id = %client.id, "Comando /reset recebido, resetando client");
            let reset_ts = reset_msg.timestamp;
            let client_id = client.id;
            sql!(
                pool,
                "UPDATE zain.clients
                 SET state = 'LEAD', state_props = '{}', memory = '{}',
                     updated_at = now(),
                     last_whatsapp_message_processed_at = $reset_ts,
                     history_starts_at = $reset_ts
                 WHERE id = $client_id"
            )
            .execute()
            .await?;
            // Atualizar estado in-memory para o próximo loop
            client.state = "LEAD".into();
            client.state_props = json!({});
            client.memory = json!({});
            client.last_whatsapp_message_processed_at = reset_ts;
            client.history_starts_at = reset_ts;
            // Fechar execução atual e abrir nova atomicamente
            exec_id = rotate_execution(pool, exec_id, "cancelled", None, &client).await?;
            continue;
        }

        let new_count = new_messages.len();
        let new_summary: String = new_messages
            .iter()
            .map(|m| m.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        let result = run_workflow(
            pool,
            ai,
            chat_model,
            &client,
            &history,
            new_count,
            &new_summary,
            max_ts,
            exec_id,
        )
        .await;

        match result {
            Ok(WorkflowOutcome::Restart) => {
                tracing::info!(client_id = %client.id, "Workflow reiniciado por novas mensagens");
                exec_id = rotate_execution(pool, exec_id, "cancelled", None, &client).await?;
                continue;
            }
            Ok(WorkflowOutcome::Completed {
                final_state,
                llm_log,
            }) => {
                client.state = final_state;
                if let Some(ts) = max_ts {
                    update_last_processed(pool, client.id, ts).await?;
                    client.last_whatsapp_message_processed_at = Some(ts);
                }
                // Fechar + abrir atomicamente — a nova execução será fechada
                // no topo do loop se não houver mensagens novas
                exec_id =
                    rotate_execution(pool, exec_id, "completed", Some(&llm_log), &client).await?;
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

#[allow(clippy::too_many_arguments)]
async fn run_workflow(
    pool: &Pool,
    ai: &ai::Client,
    chat_model: &str,
    client: &ClientRow,
    history: &[ConversationMessage],
    new_message_count: usize,
    new_messages_summary: &str,
    known_max_ts: Option<DateTime<Utc>>,
    exec_id: Uuid,
) -> anyhow::Result<WorkflowOutcome> {
    let handler = states::get_handler(&client.state);

    // Montar tools: estado-específicas + globais (send + done + consultas)
    let mut state_tools = handler.tool_definitions();
    state_tools.push(tools::send_whatsapp_message_tool());
    state_tools.push(tools::done_tool());
    state_tools.push(tools::consultar_simei_cnpj_tool());
    state_tools.push(tools::consultar_cnae_por_codigo_tool());
    state_tools.push(tools::buscar_cnae_por_atividade_tool());
    let tools_json: Vec<Value> = state_tools.iter().map(|t| t.to_ollama_json()).collect();

    let system_prompt = states::build_system_prompt(&*handler);
    let user_msg =
        states::build_context_message(client, history, new_message_count, new_messages_summary);

    let mut messages = vec![
        ChatMessage::system(system_prompt),
        ChatMessage::user(user_msg),
    ];

    let mut current_state = client.state.clone();
    let mut state_props = client.state_props.clone();
    let mut memory = client.memory.clone();
    let mut executed_consequential = false;
    let mut done = false;

    // Loop de interação com o LLM (tool calls iterativas)
    let max_iterations = 10;
    for iteration in 0..max_iterations {
        tracing::debug!(
            client_id = %client.id,
            iteration = iteration,
            "Chamando LLM"
        );
        let response = ai.chat(chat_model, &messages, &tools_json).await?;

        tracing::info!(
            client_id = %client.id,
            iteration = iteration,
            input_tokens = response.usage.input_tokens,
            output_tokens = response.usage.output_tokens,
            cost_usd = format!("{:.6}", response.usage.cost),
            "Resposta do LLM"
        );

        if let Some(ref tool_calls) = response.message.tool_calls {
            let tool_names: Vec<&str> = tool_calls
                .iter()
                .map(|c| c.function.name.as_str())
                .collect();
            tracing::info!(
                client_id = %client.id,
                iteration = iteration,
                tool_count = tool_calls.len(),
                tools = ?tool_names,
                "LLM retornou tool calls"
            );
            messages.push(ChatMessage::assistant_tool_calls(tool_calls));

            for call in tool_calls {
                let tool_name = &call.function.name;
                let tool_args = &call.function.arguments;

                let is_consequential = is_tool_consequential(tool_name, &state_tools);

                // Antes da primeira tool consequencial, verificar se chegou msg nova
                if is_consequential
                    && !executed_consequential
                    && has_new_messages(pool, &client.chat_id, known_max_ts).await?
                {
                    tracing::info!(
                        client_id = %client.id,
                        tool_name,
                        "Novas mensagens detectadas antes de tool consequencial, reiniciando"
                    );
                    // Salvar state_props/memory das tools puras já executadas
                    save_client_state(pool, client.id, &current_state, &state_props, &memory)
                        .await?;
                    update_execution_messages(pool, exec_id, &messages).await?;
                    return Ok(WorkflowOutcome::Restart);
                }

                if is_consequential {
                    executed_consequential = true;
                }

                tracing::info!(client_id = %client.id, tool_name, "Executando tool");

                if tool_name == "done" {
                    messages.push(ChatMessage::tool(
                        tool_name.clone(),
                        json!({ "status": "ok" }).to_string(),
                    ));
                    done = true;
                    continue;
                } else if tool_name == "send_whatsapp_message" {
                    let msg_text = tool_args
                        .get("message")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if !msg_text.is_empty() {
                        write_outbox(pool, &client.chat_id, msg_text).await?;
                    }
                    messages.push(ChatMessage::tool(
                        tool_name.clone(),
                        json!({ "status": "ok", "mensagem_enviada": true }).to_string(),
                    ));
                } else if tool_name == "consultar_simei_cnpj" {
                    let result =
                        execute_consultar_simei(tool_args, &mut state_props, &client.id).await;
                    messages.push(ChatMessage::tool(tool_name.clone(), result.to_string()));
                } else if tool_name == "consultar_cnae_por_codigo" {
                    let result =
                        execute_consultar_cnae_por_codigo(pool, tool_args, &client.id).await;
                    messages.push(ChatMessage::tool(tool_name.clone(), result.to_string()));
                } else if tool_name == "buscar_cnae_por_atividade" {
                    let result =
                        execute_buscar_cnae_por_atividade(pool, tool_args, &client.id).await;
                    messages.push(ChatMessage::tool(tool_name.clone(), result.to_string()));
                } else {
                    let result =
                        handler.execute_tool(tool_name, tool_args, &mut state_props, &mut memory);

                    match result {
                        ToolResult::Ok(value) => {
                            messages.push(ChatMessage::tool(tool_name.clone(), value.to_string()));
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
                            messages.push(ChatMessage::tool(
                                tool_name.clone(),
                                format!("Transição para estado {new_state} realizada com sucesso."),
                            ));
                        }
                    }
                }
            }

            if done {
                break;
            }
        } else {
            // LLM respondeu com texto sem chamar tools — isso conta como done
            break;
        }

        // Salvar progresso do LLM a cada iteração
        update_execution_messages(pool, exec_id, &messages).await?;
    }

    // Salvar estado final
    save_client_state(pool, client.id, &current_state, &state_props, &memory).await?;

    Ok(WorkflowOutcome::Completed {
        final_state: current_state,
        llm_log: messages,
    })
}

fn is_tool_consequential(tool_name: &str, tools: &[ToolDef]) -> bool {
    tools
        .iter()
        .find(|t| t.name == tool_name)
        .map(|t| t.consequential)
        .unwrap_or(true) // desconhecida = tratar como consequencial
}

// ── Global tools de consulta externa ──────────────────────────────────

/// Executa a consulta SIMEI via rpa-mei. Salva o resultado em
/// state_props["ultima_consulta_simei"] para auditoria.
async fn execute_consultar_simei(args: &Value, state_props: &mut Value, client_id: &Uuid) -> Value {
    let cnpj_raw = args.get("cnpj").and_then(|v| v.as_str()).unwrap_or("");
    let cnpj_digits: String = cnpj_raw.chars().filter(|c| c.is_ascii_digit()).collect();

    if cnpj_digits.len() != 14 {
        tracing::warn!(
            client_id = %client_id,
            cnpj_recebido = %cnpj_raw,
            "consultar_simei_cnpj: CNPJ inválido (deve ter 14 dígitos)"
        );
        return json!({
            "erro": "CNPJ inválido — deve ter 14 dígitos",
            "cnpj_recebido": cnpj_raw,
        });
    }

    tracing::info!(
        client_id = %client_id,
        cnpj = %cnpj_digits,
        "consultar_simei_cnpj: iniciando consulta via rpa-mei (~15-30s)"
    );
    let start = std::time::Instant::now();

    match rpa_mei::consulta::consultar_optante(&cnpj_digits).await {
        Ok(consulta) => {
            let elapsed_ms = start.elapsed().as_millis();
            tracing::info!(
                client_id = %client_id,
                cnpj = %cnpj_digits,
                elapsed_ms = elapsed_ms as u64,
                optante_simei = consulta.situacao_simei.optante,
                optante_simples = consulta.situacao_simples.optante,
                nome_empresarial = %consulta.nome_empresarial,
                "consultar_simei_cnpj: consulta concluída com sucesso"
            );

            let result = json!({
                "optante_simei": consulta.situacao_simei.optante,
                "simei_desde": consulta.situacao_simei.desde,
                "optante_simples": consulta.situacao_simples.optante,
                "simples_desde": consulta.situacao_simples.desde,
                "nome_empresarial": consulta.nome_empresarial,
                "data_consulta": consulta.data_consulta,
            });

            // Grava em state_props para auditoria (mesmo sem transição de estado).
            if let Some(obj) = state_props.as_object_mut() {
                obj.insert("ultima_consulta_simei".into(), result.clone());
            }

            result
        }
        Err(e) => {
            let elapsed_ms = start.elapsed().as_millis();
            tracing::warn!(
                client_id = %client_id,
                cnpj = %cnpj_digits,
                elapsed_ms = elapsed_ms as u64,
                error = %e,
                "consultar_simei_cnpj: falha na consulta"
            );
            json!({
                "erro": format!("Falha ao consultar: {}", e),
            })
        }
    }
}

/// Consulta se um código CNAE específico é MEI-compatível.
async fn execute_consultar_cnae_por_codigo(pool: &Pool, args: &Value, client_id: &Uuid) -> Value {
    let codigo_raw = args.get("codigo").and_then(|v| v.as_str()).unwrap_or("");
    let codigo_norm: String = codigo_raw.chars().filter(|c| c.is_ascii_digit()).collect();

    if codigo_norm.is_empty() {
        return json!({ "erro": "código CNAE vazio" });
    }

    let pattern = format!("{}%", codigo_norm);
    let db = match pool.get().await {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(client_id = %client_id, error = %e, "Falha ao obter conexão do pool");
            return json!({ "erro": format!("Falha ao conectar no banco: {}", e) });
        }
    };

    let rows = db
        .query(
            "SELECT ocupacao, cnae_subclasse_id, cnae_descricao
             FROM mei_cnaes.ocupacoes
             WHERE cnae_subclasse_id LIKE $1
             ORDER BY ocupacao
             LIMIT 10",
            &[&pattern],
        )
        .await;

    match rows {
        Ok(rows) => {
            let matches: Vec<Value> = rows
                .iter()
                .map(|row| {
                    let codigo: String = row.get("cnae_subclasse_id");
                    let ocupacao: String = row.get("ocupacao");
                    let descricao: String = row.get("cnae_descricao");
                    json!({
                        "codigo": codigo.trim(),
                        "ocupacao": ocupacao,
                        "descricao": descricao,
                    })
                })
                .collect();

            json!({
                "pode_ser_mei": !matches.is_empty(),
                "matches": matches,
            })
        }
        Err(e) => {
            tracing::warn!(
                client_id = %client_id,
                error = %e,
                "Falha na query de CNAE por código"
            );
            json!({ "erro": format!("Falha ao consultar CNAE: {}", e) })
        }
    }
}

/// Busca CNAEs MEI-compatíveis pela descrição livre da atividade.
async fn execute_buscar_cnae_por_atividade(pool: &Pool, args: &Value, client_id: &Uuid) -> Value {
    let descricao = args
        .get("descricao")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();

    if descricao.is_empty() {
        return json!({ "erro": "descrição vazia" });
    }

    let pattern = format!("%{}%", descricao);
    let db = match pool.get().await {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(client_id = %client_id, error = %e, "Falha ao obter conexão do pool");
            return json!({ "erro": format!("Falha ao conectar no banco: {}", e) });
        }
    };

    let rows = db
        .query(
            "SELECT ocupacao, cnae_subclasse_id, cnae_descricao
             FROM mei_cnaes.ocupacoes
             WHERE ocupacao ILIKE $1 OR cnae_descricao ILIKE $1
             ORDER BY ocupacao
             LIMIT 10",
            &[&pattern],
        )
        .await;

    match rows {
        Ok(rows) => {
            let resultados: Vec<Value> = rows
                .iter()
                .map(|row| {
                    let codigo: String = row.get("cnae_subclasse_id");
                    let ocupacao: String = row.get("ocupacao");
                    let descricao: String = row.get("cnae_descricao");
                    json!({
                        "codigo": codigo.trim(),
                        "ocupacao": ocupacao,
                        "descricao": descricao,
                    })
                })
                .collect();

            if resultados.is_empty() {
                json!({
                    "resultados": [],
                    "mensagem": "Nenhuma ocupação MEI bate com essa descrição. Pode ser uma atividade não permitida para MEI.",
                })
            } else {
                json!({ "resultados": resultados })
            }
        }
        Err(e) => {
            tracing::warn!(
                client_id = %client_id,
                error = %e,
                "Falha na busca de CNAE por atividade"
            );
            json!({ "erro": format!("Falha ao buscar CNAE: {}", e) })
        }
    }
}

async fn has_new_messages(
    pool: &Pool,
    chat_id: &str,
    known_max_ts: Option<DateTime<Utc>>,
) -> anyhow::Result<bool> {
    let Some(known) = known_max_ts else {
        return Ok(false);
    };
    let current_max: Option<DateTime<Utc>> = sql!(
        pool,
        "SELECT MAX(\"timestamp\") AS ts
         FROM whatsapp.messages
         WHERE chat_id = $chat_id AND from_me = false"
    )
    .fetch_value()
    .await?;

    Ok(current_max.is_some_and(|ts| ts > known))
}

// ── Helpers ────────────────────────────────────────────────────────────

/// Fecha a execução atual e abre uma nova atomicamente (CTE),
/// garantindo que sempre existe uma execução 'running' para o client.
async fn rotate_execution(
    pool: &Pool,
    old_exec_id: Uuid,
    close_status: &str,
    llm_messages: Option<&[ChatMessage]>,
    client: &ClientRow,
) -> anyhow::Result<Uuid> {
    let llm_json: Value = llm_messages
        .map(serde_json::to_value)
        .transpose()?
        .unwrap_or(Value::Null);
    let state_after: Option<&str> = if close_status == "completed" {
        Some(&client.state)
    } else {
        None
    };
    let client_id = client.id;
    let state_before = &client.state;

    let mut db = pool.get().await?;
    let tx = db.transaction().await?;

    sql!(
        &tx,
        "UPDATE zain.executions
         SET status = $close_status,
             state_after = $state_after,
             llm_messages = COALESCE($llm_json, llm_messages),
             finished_at = now()
         WHERE id = $old_exec_id"
    )
    .execute()
    .await?;

    let new_id: Uuid = sql!(
        &tx,
        "INSERT INTO zain.executions (client_id, state_before, trigger_type)
         VALUES ($client_id, $state_before, 'message')
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

async fn update_execution_messages(
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
    history_starts_at: Option<DateTime<Utc>>,
    ai: &ai::Client,
) -> anyhow::Result<(Vec<ConversationMessage>, Option<DateTime<Utc>>)> {
    struct Row {
        from_me: bool,
        msg_type: String,
        text_body: Option<String>,
        voice: Option<Value>,
        timestamp: DateTime<Utc>,
    }

    let rows: Vec<Row> = sql!(
        pool,
        "SELECT from_me, msg_type, text_body, voice, \"timestamp\"
         FROM whatsapp.messages
         WHERE chat_id = $chat_id
           AND \"timestamp\" >= COALESCE($history_starts_at?, 'epoch'::timestamptz)
         ORDER BY \"timestamp\" DESC
         LIMIT 60"
    )
    .fetch_all()
    .await?
    .iter()
    .map(|r| Row {
        from_me: r.from_me,
        msg_type: r.msg_type.clone(),
        text_body: r.text_body.clone(),
        voice: r.voice.clone(),
        timestamp: r.timestamp,
    })
    .collect();

    let mut messages = Vec::new();
    let mut total_chars = 0usize;
    let max_ts: Option<DateTime<Utc>> = rows.iter().find(|r| !r.from_me).map(|r| r.timestamp);

    for row in &rows {
        let text: String = match row.msg_type.as_str() {
            "text" => row.text_body.clone().unwrap_or_default(),
            "voice" => {
                let voice_id = row
                    .voice
                    .as_ref()
                    .and_then(|v| v.get("id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let link = row
                    .voice
                    .as_ref()
                    .and_then(|v| v.get("link"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if !voice_id.is_empty() {
                    match transcribe_voice(pool, ai, voice_id, link).await {
                        Ok(t) => format!("[áudio transcrito]: {t}"),
                        Err(e) => {
                            tracing::warn!(voice_id, "Falha ao transcrever áudio: {e:#}");
                            "[áudio não transcrito]".into()
                        }
                    }
                } else {
                    "[áudio]".into()
                }
            }
            other => {
                tracing::debug!(msg_type = other, "Tipo de mensagem ignorado no histórico");
                continue;
            }
        };

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

async fn transcribe_voice(
    pool: &Pool,
    ai: &ai::Client,
    voice_id: &str,
    download_link: &str,
) -> anyhow::Result<String> {
    // 1. Verificar cache
    let cached: Option<String> = sql!(
        pool,
        "SELECT transcription FROM zain.audio_transcriptions WHERE id = $voice_id"
    )
    .fetch_optional()
    .await?
    .map(|r| r.transcription);

    if let Some(transcription) = cached {
        return Ok(transcription);
    }

    // 2. Baixar o áudio
    let audio_bytes = reqwest::Client::new()
        .get(download_link)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;

    // 3. Transcrever via ai::Client
    let transcription = ai
        .transcribe(
            TRANSCRIPTION_MODEL,
            audio_bytes.to_vec(),
            "audio.ogg",
            "audio/ogg",
        )
        .await?;

    // 4. Salvar no cache
    let transcription_ref = &transcription;
    sql!(
        pool,
        "INSERT INTO zain.audio_transcriptions (id, transcription)
         VALUES ($voice_id, $transcription_ref)
         ON CONFLICT (id) DO NOTHING"
    )
    .execute()
    .await?;

    Ok(transcription)
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
