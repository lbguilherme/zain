use cubos_sql::sql;
use deadpool_postgres::Pool;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

use super::{Tool, ToolContext, ToolDef, ToolOutput, params_for, typed_handler};

#[derive(Deserialize, JsonSchema)]
struct Args {
    /// Texto da mensagem a enviar para o cliente
    message: String,
}

pub fn tool() -> Tool {
    Tool {
        def: ToolDef {
            name: "send_whatsapp_message",
            description: "Envia uma mensagem de texto para o cliente no WhatsApp. Esta é a ÚNICA forma de se comunicar com o cliente. Toda resposta deve ser enviada através desta ferramenta.",
            consequential: true,
            parameters: params_for::<Args>(),
        },
        handler: typed_handler(|ctx: ToolContext, args: Args, memory| async move {
            if args.message.is_empty() {
                return ToolOutput::new(
                    json!({ "status": "ok", "mensagem_enviada": false }),
                    memory,
                );
            }
            match write_outbox(&ctx.pool, &ctx.chat_id, &args.message).await {
                Ok(()) => {
                    ToolOutput::new(json!({ "status": "ok", "mensagem_enviada": true }), memory)
                }
                Err(e) => {
                    tracing::warn!(chat_id = %ctx.chat_id, error = %e, "Falha ao escrever no outbox");
                    ToolOutput::err(
                        json!({ "status": "erro", "mensagem": format!("Falha ao enviar: {e}") }),
                        memory,
                    )
                }
            }
        }),
        must_use_tool_result: false,
    }
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
