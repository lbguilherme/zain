use cubos_sql::sql;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

use super::{Tool, ToolContext, ToolDef, ToolOutput, params_for, typed_handler};

#[derive(Deserialize, JsonSchema)]
struct Args {
    /// Motivo da recusa em linguagem direta (ex: 'CNPJ optante Simples Nacional, não SIMEI' ou 'atividade regulamentada não permitida pra MEI')
    motivo: String,
}

pub fn tool() -> Tool {
    Tool {
        def: ToolDef {
            name: "recusar_lead",
            description: "Marca o lead como recusado. Use apenas quando você tiver sinal claro de que a Zain não vai atender esse lead (ex: alguma tool retornou pedindo pra recusar, ou a atividade não é permitida pra MEI). Antes de chamar, envie uma mensagem gentil explicando o motivo via `send_whatsapp_message`.",
            consequential: true,
            parameters: params_for::<Args>(),
        },
        handler: typed_handler(|ctx: ToolContext, args: Args, memory| async move {
            let motivo = &args.motivo;
            let client_id = ctx.client_id;
            match sql!(
                &ctx.pool,
                "UPDATE zain.clients
                 SET recusa_motivo = $motivo,
                     recusado_em   = now(),
                     updated_at    = now()
                 WHERE id = $client_id"
            )
            .execute()
            .await
            {
                Ok(_) => ToolOutput::new(json!({ "status": "ok", "recusado": true }), memory),
                Err(e) => {
                    tracing::warn!(client_id = %ctx.client_id, error = %e, "recusar_lead: falha ao salvar");
                    ToolOutput::err(
                        json!({ "status": "erro", "mensagem": format!("Falha ao salvar: {e}") }),
                        memory,
                    )
                }
            }
        }),
        must_use_tool_result: false,
        enabled_when: None,
    }
}
