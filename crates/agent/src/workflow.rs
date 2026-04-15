use std::collections::HashSet;
use std::sync::Arc;

use ai::{ChatMessage, ChatRequest};
use chrono::{DateTime, Utc};
use cubos_sql::sql;
use deadpool_postgres::Pool;
use serde_json::Value;
use uuid::Uuid;

use crate::dispatch::{
    ClientRow, Models, WorkflowOutcome, fetch_client_row, save_client_memory,
    update_execution_messages,
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
    // Snapshot do cliente que segue *mutável* pelo turno — tools
    // consequenciais podem mexer em colunas que afetam `enabled_when`
    // de outras tools (ex: `abrir_empresa` persistindo `mei_ccmei_pdf`
    // e destravando `send_ccmei`), então ao fim de cada iteração
    // refetchamos o ClientRow do banco e recomputamos o set de tools.
    let mut current_client = client.clone();
    // Conjunto de tools que já foram chamadas neste turno. Uma tool
    // que já apareceu no histórico NÃO pode sumir do set mesmo que
    // o `enabled_when` atual diga que ela deveria estar desabilitada
    // — se o LLM enxergou a tool no último prompt, o histórico de
    // `ToolCall` referente a ela precisa continuar casando com uma
    // definição presente no set, senão o provider rejeita.
    let mut used_tool_names: HashSet<String> = HashSet::new();
    // Tools com `enabled_when` definido só entram no set do turno
    // quando o predicado bate com o estado atual do cliente. Isso
    // esconde do LLM tools que não fazem sentido no momento (ex:
    // `auth_govbr_otp` só aparece no meio de um fluxo de 2FA).
    let mut installed_tools = filter_enabled_tools(&current_client, &used_tool_names);

    let system_prompt = prompt::build_system_prompt(pool, client).await?;
    let user_msg =
        prompt::build_context_message(client, history, new_message_count, new_messages_summary);

    // Cada imagem vira um par `InputText` (com o ID que bate nas
    // referências `<attachment type="image" id="..."/>` do prompt) +
    // `InputImage`.
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
    let mut wait_called = false;
    // Fica `true` a partir do momento que o LLM chamou
    // `wait_client_message()` em *alguma* iteração, mesmo que a chamada
    // tenha sido ignorada porque uma tool `must_use_tool_result` forçou
    // mais uma rodada. Permite aceitar uma resposta vazia na rodada
    // seguinte como "já terminei".
    let mut wait_seen = false;
    let mut text_only_retries = 0;
    const MAX_TEXT_ONLY_RETRIES: u32 = 1;

    let max_iterations = 10;
    for iteration in 0..max_iterations {
        let mut must_reprompt = false;
        // Rebuild dos `ChatTool` a cada iteração — `chat_tools`
        // referencia `installed_tools`, que pode ter sido recomputado
        // no final da iteração anterior após o refetch do cliente.
        let chat_tools: Vec<ai::ChatTool> = installed_tools
            .iter()
            .map(|t| t.def.as_chat_tool())
            .collect();
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

        // Se o LLM já tinha chamado `wait_client_message()` antes
        // (ignorado por `must_use_tool_result`) e nessa rodada não
        // devolveu nada — nem texto, nem tool call — significa que ele
        // não tinha mais o que dizer sobre o resultado da tool forçada.
        // Aceita como encerramento implícito em vez de insistir.
        if wait_seen && response.messages.is_empty() {
            tracing::info!(
                client_id = %client.id,
                iteration = iteration,
                "LLM devolveu resposta vazia após wait_client_message anterior — encerrando turno"
            );
            break;
        }

        // Texto solto é invisível pro cliente — a única forma de falar
        // é via `send_whatsapp_message`. Se o LLM só devolveu texto,
        // injeta um lembrete e força mais uma rodada; se persistir,
        // aborta o turno pra não perder a mensagem silenciosamente.
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
                 o turno com `wait_client_message()`."
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

        // Tool calls entram no histórico com `result: None` e são
        // preenchidas in-place na ordem de execução abaixo.
        let history_start = messages.len();
        messages.extend(response.messages);

        // `send_whatsapp_message` roda antes de qualquer outra tool pra
        // que a resposta pro cliente saia rápido, sem ficar esperando
        // tools lentas (consultas externas, auth gov.br) da mesma leva.
        let mut execution_order: Vec<usize> = (history_start..messages.len())
            .filter(|&i| matches!(messages[i], ChatMessage::ToolCall { .. }))
            .collect();
        execution_order.sort_by_key(|&i| match &messages[i] {
            ChatMessage::ToolCall { name, .. } if name == "send_whatsapp_message" => 0,
            _ => 1,
        });

        for abs_idx in execution_order {
            // `result_slot` segura a borrow em `messages[abs_idx]` até
            // o fim da iteração; no branch de restart abaixo, NLL
            // libera a borrow porque o slot não é mais usado naquela
            // path, então `update_execution_messages` consegue tomar
            // `&messages`.
            let (tool_name, tool_args, result_slot): (String, Value, &mut Option<String>) =
                match &mut messages[abs_idx] {
                    ChatMessage::ToolCall {
                        name,
                        arguments,
                        result,
                        ..
                    } => (name.clone(), arguments.clone(), result),
                    _ => unreachable!("execution_order contém apenas ToolCall"),
                };

            let installed = installed_tools.iter().find(|t| t.def.name == tool_name);
            // Tools desconhecidas são tratadas como consequenciais por segurança.
            let is_consequential = installed.map(|t| t.def.consequential).unwrap_or(true);

            // Marca a tool como usada neste turno — independente de
            // ter sido executada com sucesso. O filtro subsequente usa
            // isso pra nunca remover do set uma tool que já apareceu
            // no histórico de tool calls.
            used_tool_names.insert(tool_name.clone());

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
                *result_slot = Some(
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

            *result_slot = Some(out.value.to_string());

            if tool.must_use_tool_result || out.is_error {
                must_reprompt = true;
            }

            // `wait_client_message` é a única tool que também controla
            // o fluxo do loop.
            if tool_name == "wait_client_message" {
                wait_called = true;
                wait_seen = true;
            }
        }

        if wait_called && !must_reprompt {
            break;
        }
        if wait_called && must_reprompt {
            // Alguma tool da leva exige que o LLM veja o resultado
            // antes de encerrar — ignora o `wait_client_message` e
            // força outra rodada.
            tracing::info!(
                client_id = %client.id,
                iteration = iteration,
                "wait_client_message ignorado: tool com must_use_tool_result na mesma leva"
            );
            wait_called = false;
        }

        update_execution_messages(pool, exec_id, &messages).await?;

        // Refetch do snapshot do cliente antes da próxima iteração.
        // As tools que acabaram de rodar podem ter mexido em colunas
        // que destravam/escondem outras tools (ex: `auth_govbr`
        // persistindo `mei_ccmei_pdf` e destravando `send_ccmei`).
        // Tools que já foram usadas neste turno permanecem no set —
        // ver comentário em `filter_enabled_tools`.
        match fetch_client_row(pool, client.id, exec_id).await {
            Ok(Some(refreshed)) => {
                current_client = refreshed;
                installed_tools = filter_enabled_tools(&current_client, &used_tool_names);
            }
            Ok(None) => {
                tracing::warn!(
                    client_id = %client.id,
                    "refetch do ClientRow devolveu None — mantendo snapshot anterior"
                );
            }
            Err(e) => {
                tracing::warn!(
                    client_id = %client.id,
                    error = %e,
                    "refetch do ClientRow falhou — mantendo snapshot anterior"
                );
            }
        }
    }

    save_client_memory(pool, client.id, &memory).await?;

    Ok(WorkflowOutcome::Completed { llm_log: messages })
}

/// Monta o set de tools expostas ao LLM para a iteração corrente.
/// Regras:
/// - Tools sem `enabled_when` entram sempre.
/// - Tools com `enabled_when` entram se o predicado bate com o
///   snapshot atual do cliente.
/// - Tools cujo nome já apareceu em `used_tool_names` entram SEMPRE,
///   mesmo que o predicado diga que deveriam estar escondidas —
///   remover do set depois que o LLM já emitiu uma `ToolCall` quebra
///   o histórico da conversa com o provider.
fn filter_enabled_tools(client: &ClientRow, used_tool_names: &HashSet<String>) -> Vec<Tool> {
    tools::all_tools()
        .into_iter()
        .filter(|t| {
            used_tool_names.contains(t.def.name)
                || t.enabled_when.map(|f| f(client)).unwrap_or(true)
        })
        .collect()
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
