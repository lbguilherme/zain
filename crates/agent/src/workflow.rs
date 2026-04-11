use std::sync::Arc;

use ai::ChatMessage;
use chrono::{DateTime, Utc};
use cubos_sql::sql;
use deadpool_postgres::Pool;
use serde_json::Value;
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
    let tools_json: Vec<Value> = installed_tools
        .iter()
        .map(|t| t.def.to_ollama_json())
        .collect();

    let system_prompt = prompt::build_system_prompt();
    let user_msg =
        prompt::build_context_message(client, history, new_message_count, new_messages_summary);

    // Reúne todas as imagens do histórico na ordem cronológica em que
    // aparecem. Cada uma vai como uma `ChatImage` anexada à user message,
    // com `label` trazendo o ID — isso bate com as referências
    // `<attachment type="image" id="..."/>` espalhadas no texto.
    let chat_images: Vec<ai::ChatImage> = history
        .iter()
        .flat_map(|m| m.images.iter())
        .map(|img| {
            ai::ChatImage::with_label(
                img.bytes.clone(),
                img.mime_type.clone(),
                format!("Imagem ID: {}", img.id),
            )
        })
        .collect();

    let user_chat_msg = if chat_images.is_empty() {
        ChatMessage::user(user_msg)
    } else {
        ChatMessage::user_with_images(user_msg, chat_images)
    };

    let mut messages = vec![ChatMessage::system(system_prompt), user_chat_msg];

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
        let response = ai.chat(&models.chat, &messages, &tools_json).await?;

        tracing::info!(
            client_id = %client.id,
            iteration = iteration,
            input_tokens = response.usage.input_tokens,
            output_tokens = response.usage.output_tokens,
            cost_usd = format!("{:.6}", response.usage.cost),
            "Resposta do LLM"
        );

        // Texto solto é invisível pro cliente — a única forma de falar é
        // via `send_whatsapp_message`. Se o LLM devolveu só texto (ou uma
        // lista vazia de tool calls), injeta um lembrete e força mais uma
        // rodada. Se mesmo após o lembrete o LLM insistir em não chamar
        // tool, aborta o turno com erro pra não perder a mensagem do
        // cliente silenciosamente.
        let has_tool_calls = response
            .message
            .tool_calls
            .as_ref()
            .is_some_and(|c| !c.is_empty());
        if !has_tool_calls {
            tracing::warn!(
                client_id = %client.id,
                iteration = iteration,
                texto = %response.message.content,
                "LLM respondeu sem tool calls"
            );
            // Preserva o texto no log pra inspeção posterior, mesmo que
            // seja invisível pro cliente.
            messages.push(ChatMessage::assistant(response.message.content));
            if text_only_retries >= MAX_TEXT_ONLY_RETRIES {
                save_client_memory(pool, client.id, &memory).await?;
                update_execution_messages(pool, exec_id, &messages).await?;
                return Err(anyhow::anyhow!(
                    "LLM respondeu só com texto mesmo após lembrete — turno abortado"
                ));
            }
            text_only_retries += 1;
            messages.push(ChatMessage::user(
                "Lembrete: texto solto não chega pro cliente. A ÚNICA forma de \
                 falar com ele é chamando `send_whatsapp_message`. Se precisa \
                 salvar algo, chame as tools `save_*`/`anotar`. Sempre termine \
                 o turno com `done()`."
                    .into(),
            ));
            update_execution_messages(pool, exec_id, &messages).await?;
            continue;
        }
        let tool_calls = response.message.tool_calls.as_ref().unwrap();

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
        messages.push(ChatMessage::assistant_tool_calls(
            response.message.content.clone(),
            tool_calls,
        ));

        // Reordena as tool calls pra que `send_whatsapp_message` rode
        // antes de qualquer outra tool — assim a mensagem pro cliente
        // sai primeiro e as tools lentas/consequenciais que vierem na
        // mesma leva (consultas, auth gov.br, etc.) não atrasam o envio.
        let mut ordered_calls: Vec<_> = tool_calls.iter().collect();
        ordered_calls.sort_by_key(|c| {
            if c.function.name == "send_whatsapp_message" {
                0
            } else {
                1
            }
        });

        for call in ordered_calls {
            let tool_name = &call.function.name;
            let tool_args = &call.function.arguments;
            let call_id = &call.id;

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
                messages.push(ChatMessage::tool(
                    tool_name.clone(),
                    call_id.clone(),
                    serde_json::json!({
                        "status": "erro",
                        "mensagem": format!("Ferramenta '{tool_name}' não reconhecida"),
                    })
                    .to_string(),
                ));
                continue;
            };

            let out = (tool.handler)(ctx.clone(), tool_args.clone(), memory).await;
            memory = out.memory;

            messages.push(ChatMessage::tool(
                tool_name.clone(),
                call_id.clone(),
                out.value.to_string(),
            ));

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
