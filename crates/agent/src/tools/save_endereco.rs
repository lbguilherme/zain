use cubos_sql::sql;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

use super::{Tool, ToolContext, ToolDef, ToolOutput, params_for, typed_handler};

#[derive(Deserialize, JsonSchema)]
struct Args {
    /// Endereço completo
    endereco: String,
}

pub fn tool() -> Tool {
    Tool {
        def: ToolDef {
            name: "save_endereco",
            description: "Salva o endereço do lead.",
            consequential: false,
            parameters: params_for::<Args>(),
        },
        handler: typed_handler(|ctx: ToolContext, args: Args, memory| async move {
            let endereco: Option<&str> = Some(&args.endereco);
            let client_id = ctx.client_id;
            match sql!(
                &ctx.pool,
                "UPDATE zain.clients SET endereco = $endereco, updated_at = now() WHERE id = $client_id"
            )
            .execute()
            .await
            {
                Ok(_) => ToolOutput::new(json!({ "status": "ok" }), memory),
                Err(e) => {
                    tracing::warn!(client_id = %ctx.client_id, error = %e, "save_endereco: falha ao salvar");
                    ToolOutput::err(
                        json!({ "status": "erro", "mensagem": format!("Falha ao salvar: {e}") }),
                        memory,
                    )
                }
            }
        }),
        must_use_tool_result: false,
    }
}
