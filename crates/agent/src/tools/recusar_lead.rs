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
            description: "Marca o lead como recusado (salva motivo em `recusa_motivo` e timestamp em `recusado_em`). Use APENAS quando: (a) save_cnpj retornou status=erro indicando que o CNPJ não é MEI, ou (b) buscar_cnae_por_atividade confirmou que a atividade da pessoa não é permitida pra MEI, ou (c) consultar_divida_pgfn retornou tem_divida=true com total_divida acima de R$ 15.000. Antes de chamar, envie uma mensagem gentil explicando o motivo pelo send_whatsapp_message.",
            consequential: true,
            parameters: params_for::<Args>(),
        },
        handler: typed_handler(|ctx: ToolContext, args: Args, memory| async move {
            let motivo: Option<&str> = Some(&args.motivo);
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
    }
}
