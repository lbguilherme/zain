use std::sync::Arc;

use ai::ChatMessage;
use chrono::{DateTime, Utc};
use cubos_sql::sql;
use deadpool_postgres::Pool;
use serde_json::Value;
use uuid::Uuid;

use crate::dispatch::{
    ClientRow, Models, WorkflowOutcome, save_client_props, update_execution_messages,
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

    let mut props = client.props.clone();
    let mut memory = client.memory.clone();
    let mut executed_consequential = false;
    let mut done = false;

    let max_iterations = 10;
    for iteration in 0..max_iterations {
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

        let Some(ref tool_calls) = response.message.tool_calls else {
            // LLM respondeu com texto sem chamar tools — isso conta como done
            break;
        };

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
                save_client_props(pool, client.id, &props, &memory).await?;
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

            let out = (tool.handler)(ctx.clone(), tool_args.clone(), props, memory).await;
            props = out.props;
            memory = out.memory;

            messages.push(ChatMessage::tool(
                tool_name.clone(),
                call_id.clone(),
                out.value.to_string(),
            ));

            // `done` é a única tool que também controla o fluxo do loop.
            if tool_name == "done" {
                done = true;
            }
        }

        if done {
            break;
        }

        update_execution_messages(pool, exec_id, &messages).await?;
    }

    save_client_props(pool, client.id, &props, &memory).await?;

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
