use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

use super::{Tool, ToolDef, ToolOutput, params_for, typed_sync_handler};

#[derive(Deserialize, JsonSchema)]
struct Args {}

pub fn tool() -> Tool {
    Tool {
        def: ToolDef {
            name: "iniciar_pagamento",
            description: "Sinaliza que o lead está pronto pro cadastro de cartão de crédito. Requer nome, CPF e saber se tem MEI. Depois de chamar, seta a flag `pagamento_solicitado` em props.",
            consequential: true,
            parameters: params_for::<Args>(),
        },
        handler: typed_sync_handler(|_args: Args, mut props, memory| {
            let has_nome = props.get("nome").and_then(|v| v.as_str()).is_some();
            let has_cpf = props.get("cpf").and_then(|v| v.as_str()).is_some();
            let has_tem_mei = props.get("tem_mei").and_then(|v| v.as_bool()).is_some();

            if !has_nome || !has_cpf || !has_tem_mei {
                return ToolOutput {
                    value: json!({
                        "status": "erro",
                        "mensagem": "Dados insuficientes. Necessário: nome, CPF e saber se tem MEI."
                    }),
                    props,
                    memory,
                };
            }

            if let Some(obj) = props.as_object_mut() {
                obj.insert("pagamento_solicitado".into(), json!(true));
                obj.insert(
                    "pagamento_solicitado_em".into(),
                    json!(chrono::Utc::now().to_rfc3339()),
                );
            }

            ToolOutput {
                value: json!({ "status": "ok", "pagamento_solicitado": true }),
                props,
                memory,
            }
        }),
    }
}
