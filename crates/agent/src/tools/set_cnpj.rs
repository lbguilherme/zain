use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

use super::{Tool, ToolDef, ToolOutput, params_for, typed_sync_handler};
use crate::validators;

#[derive(Deserialize, JsonSchema)]
struct Args {
    /// CNPJ (apenas números, 14 dígitos)
    cnpj: String,
}

pub fn tool() -> Tool {
    Tool {
        def: ToolDef {
            name: "set_cnpj",
            description: "Salva o CNPJ do MEI existente.",
            consequential: false,
            parameters: params_for::<Args>(),
        },
        handler: typed_sync_handler(|args: Args, mut props, memory| {
            if !validators::validar_cnpj(&args.cnpj) {
                return ToolOutput {
                    value: json!({
                        "status": "erro",
                        "mensagem": "CNPJ inválido — os dígitos verificadores não batem. Peça o CNPJ correto ao cliente de forma amigável."
                    }),
                    props,
                    memory,
                };
            }
            let cnpj_digits: String = args.cnpj.chars().filter(|c| c.is_ascii_digit()).collect();
            props["cnpj"] = json!(cnpj_digits);
            ToolOutput {
                value: json!({ "status": "ok" }),
                props,
                memory,
            }
        }),
    }
}
