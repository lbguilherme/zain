use std::sync::Arc;

use ai::{ChatMessage, ChatRequest};
use chrono::{DateTime, Utc};
use cubos_sql::sql;
use deadpool_postgres::Pool;
use uuid::Uuid;

use crate::dispatch::{
    ClientRow, Models, WorkflowOutcome, save_client_memory, update_execution_messages,
};
use crate::history::ConversationMessage;
use crate::prompt;
use crate::tools::{self, Tool, ToolContext};

#[allow(clippy::too_many_arguments)]
pub async fn run_workflow(
    pool: &Pool,
    ai: &Arc<ai::Client>,
    models: &Arc<Models>,
    client: &ClientRow,
    history: &[ConversationMessage],
    new_message_count: usize,
    new_messages_summary: &str,
    known_max_ts: Option<DateTime<Utc>>,
    exec_id: Uuid,
) -> anyhow::Result<WorkflowOutcome> {
    let ctx = ToolContext::new(pool.clone(), ai.clone(), models.clone(), client);
    let installed_tools: Vec<Tool> = tools::all_tools();
    let chat_tools: Vec<ai::ChatTool> = installed_tools
        .iter()
        .map(|t| t.def.as_chat_tool())
        .collect();

    let system_prompt = prompt::build_system_prompt();
    let user_msg =
        prompt::build_context_message(client, history, new_message_count, new_messages_summary);

    // Cada imagem vira um par `InputText` (com o ID, batendo nas
    // referências `<attachment type="image" id="..."/>` do texto) +
    // `InputImage`, na ordem cronológica do histórico.
    let mut messages: Vec<ChatMessage> = Vec::new();
    messages.push(ChatMessage::InputText { text: user_msg });
    for img in history.iter().flat_map(|m| m.images.iter()) {
        messages.push(ChatMessage::InputText {
            text: format!("Imagem ID: {}", img.id),
        });
        messages.push(ChatMessage::InputImage {
            bytes: img.bytes.clone(),
            mime_type: img.mime_type.clone(),
        });
    }

    let mut memory = client.memory.clone();
    let mut executed_consequential = false;
    let mut done = false;
    let mut text_only_retries = 0;
    const MAX_TEXT_ONLY_RETRIES: u32 = 1;

    let max_iterations = 10;
    for iteration in 0..max_iterations {
        let mut must_reprompt = false;
        tracing::debug!(
            client_id = %client.id,
            iteration = iteration,
            "Chamando LLM"
        );
        let response = ai
            .chat(ChatRequest {
                model: &models.chat,
                system: &system_prompt,
                messages: &messages,
                tools: &chat_tools,
            })
            .await?;

        tracing::info!(
            client_id = %client.id,
            iteration = iteration,
            input_tokens = response.input_tokens,
            output_tokens = response.output_tokens,
            cost_usd = format!("{:.6}", response.cost),
            "Resposta do LLM"
        );

        // Texto solto é invisível pro cliente — a única forma de falar é
        // via `send_whatsapp_message`. Se o LLM devolveu só texto (ou uma
        // lista vazia de tool calls), injeta um lembrete e força mais uma
        // rodada. Se mesmo após o lembrete o LLM insistir em não chamar
        // tool, aborta o turno com erro pra não perder a mensagem do
        // cliente silenciosamente.
        let has_tool_calls = response
            .messages
            .iter()
            .any(|m| matches!(m, ChatMessage::ToolCall { .. }));
        if !has_tool_calls {
            let texto: String = response
                .messages
                .iter()
                .filter_map(|m| match m {
                    ChatMessage::OutputText { text, .. } => Some(text.as_str()),
                    _ => None,
                })
                .collect();
            tracing::warn!(
                client_id = %client.id,
                iteration = iteration,
                %texto,
                "LLM respondeu sem tool calls"
            );
            // Preserva o texto no log pra inspeção posterior, mesmo que
            // seja invisível pro cliente.
            messages.extend(response.messages);
            if text_only_retries >= MAX_TEXT_ONLY_RETRIES {
                save_client_memory(pool, client.id, &memory).await?;
                update_execution_messages(pool, exec_id, &messages).await?;
                return Err(anyhow::anyhow!(
                    "LLM respondeu só com texto mesmo após lembrete — turno abortado"
                ));
            }
            text_only_retries += 1;
            messages.push(ChatMessage::InputText {
                text: "Lembrete: texto solto não chega pro cliente. A ÚNICA forma de \
                 falar com ele é chamando `send_whatsapp_message`. Se precisa \
                 salvar algo, chame as tools `save_*`/`anotar`. Sempre termine \
                 o turno com `done()`."
                    .into(),
            });
            update_execution_messages(pool, exec_id, &messages).await?;
            continue;
        }

        let tool_names: Vec<&str> = response
            .messages
            .iter()
            .filter_map(|m| match m {
                ChatMessage::ToolCall { name, .. } => Some(name.as_str()),
                _ => None,
            })
            .collect();
        let tool_count = tool_names.len();
        tracing::info!(
            client_id = %client.id,
            iteration = iteration,
            tool_count,
            tools = ?tool_names,
            "LLM retornou tool calls"
        );

        // Empurra todas as `OutputText`/`ToolCall` no
        // histórico, na ordem em que o modelo emitiu. As tool calls
        // entram com `result: None` e vão ser preenchidas in-place
        // logo abaixo.
        let history_start = messages.len();
        messages.extend(response.messages);

        // Reordena as tool calls pra que `send_whatsapp_message` rode
        // antes de qualquer outra tool — assim a mensagem pro cliente
        // sai primeiro e as tools lentas/consequenciais que vierem na
        // mesma leva (consultas, auth gov.br, etc.) não atrasam o envio.
        // A ordem de execução é independente da ordem no histórico:
        // mutamos `messages[abs_idx]` no final pra preencher o result.
        let mut execution_order: Vec<usize> = (history_start..messages.len())
            .filter(|&i| matches!(messages[i], ChatMessage::ToolCall { .. }))
            .collect();
        execution_order.sort_by_key(|&i| match &messages[i] {
            ChatMessage::ToolCall { name, .. } if name == "send_whatsapp_message" => 0,
            _ => 1,
        });

        for abs_idx in execution_order {
            let (tool_name, tool_args) = match &messages[abs_idx] {
                ChatMessage::ToolCall {
                    name, arguments, ..
                } => (name.clone(), arguments.clone()),
                _ => unreachable!("execution_order contém apenas ToolCall"),
            };

            let installed = installed_tools.iter().find(|t| t.def.name == tool_name);
            // Tools desconhecidas são tratadas como consequenciais por segurança.
            let is_consequential = installed.map(|t| t.def.consequential).unwrap_or(true);

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
                save_client_memory(pool, client.id, &memory).await?;
                update_execution_messages(pool, exec_id, &messages).await?;
                return Ok(WorkflowOutcome::Restart);
            }

            if is_consequential {
                executed_consequential = true;
            }

            tracing::info!(client_id = %client.id, tool_name, "Executando tool");

            let Some(tool) = installed else {
                fill_tool_result(
                    &mut messages[abs_idx],
                    serde_json::json!({
                        "status": "erro",
                        "mensagem": format!("Ferramenta '{tool_name}' não reconhecida"),
                    })
                    .to_string(),
                );
                continue;
            };

            let out = (tool.handler)(ctx.clone(), tool_args, memory).await;
            memory = out.memory;

            fill_tool_result(&mut messages[abs_idx], out.value.to_string());

            if tool.must_use_tool_result || out.is_error {
                must_reprompt = true;
            }

            // `done` é a única tool que também controla o fluxo do loop.
            if tool_name == "done" {
                done = true;
            }
        }

        if done && !must_reprompt {
            break;
        }
        if done && must_reprompt {
            // Alguma tool da leva exige que o LLM veja o resultado antes
            // de encerrar — ignora o `done` e força mais uma iteração.
            tracing::info!(
                client_id = %client.id,
                iteration = iteration,
                "done ignorado: tool com must_use_tool_result na mesma leva"
            );
            done = false;
        }

        update_execution_messages(pool, exec_id, &messages).await?;
    }

    save_client_memory(pool, client.id, &memory).await?;

    Ok(WorkflowOutcome::Completed { llm_log: messages })
}

fn fill_tool_result(msg: &mut ChatMessage, content: String) {
    match msg {
        ChatMessage::ToolCall { result, .. } => {
            *result = Some(content);
        }
        _ => unreachable!("fill_tool_result chamado em variante não-ToolCall"),
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
