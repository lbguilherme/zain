use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

use super::{Tool, ToolDef, ToolOutput, params_for, typed_sync_handler};

#[derive(Deserialize, JsonSchema)]
struct Args {
    /// Motivo da recusa em linguagem direta (ex: 'CNPJ optante Simples Nacional, não SIMEI' ou 'atividade regulamentada não permitida pra MEI')
    motivo: String,
}

pub fn tool() -> Tool {
    Tool {
        def: ToolDef {
            name: "recusar_lead",
            description: "Marca o lead como recusado (salva em props.recusado). Use APENAS quando: (a) consultar_simei_cnpj retornou optante_simei=false, ou (b) buscar_cnae_por_atividade confirmou que a atividade da pessoa não é permitida pra MEI, ou (c) consultar_divida_pgfn retornou tem_divida=true com total_divida acima de R$ 15.000. Antes de chamar, envie uma mensagem gentil explicando o motivo pelo send_whatsapp_message.",
            consequential: true,
            parameters: params_for::<Args>(),
        },
        handler: typed_sync_handler(|args: Args, mut props, memory| {
            if let Some(obj) = props.as_object_mut() {
                obj.insert(
                    "recusado".into(),
                    json!({
                        "motivo": args.motivo,
                        "em": chrono::Utc::now().to_rfc3339(),
                    }),
                );
            }

            ToolOutput {
                value: json!({ "status": "ok", "recusado": true }),
                props,
                memory,
            }
        }),
    }
}
