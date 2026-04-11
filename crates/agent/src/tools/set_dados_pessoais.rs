use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

use super::{Tool, ToolDef, ToolOutput, params_for, typed_sync_handler};
use crate::validators;

#[derive(Deserialize, JsonSchema)]
struct Args {
    /// Nome completo da pessoa
    #[serde(default)]
    nome: Option<String>,
    /// CPF (apenas números, 11 dígitos)
    #[serde(default)]
    cpf: Option<String>,
}

pub fn tool() -> Tool {
    Tool {
        def: ToolDef {
            name: "set_dados_pessoais",
            description: "Salva nome e/ou CPF do lead. Chame quando a pessoa informar esses dados.",
            consequential: false,
            parameters: params_for::<Args>(),
        },
        handler: typed_sync_handler(|args: Args, mut props, memory| {
            if let Some(nome) = args.nome {
                props["nome"] = json!(nome);
            }
            if let Some(cpf) = args.cpf {
                if !validators::validar_cpf(&cpf) {
                    return ToolOutput {
                        value: json!({
                            "status": "erro",
                            "mensagem": "CPF inválido — os dígitos verificadores não batem. Peça o CPF correto ao cliente de forma amigável."
                        }),
                        props,
                        memory,
                    };
                }
                let cpf_digits: String = cpf.chars().filter(|c| c.is_ascii_digit()).collect();
                props["cpf"] = json!(cpf_digits);
            }
            ToolOutput {
                value: json!({ "status": "ok", "dados_salvos": true }),
                props,
                memory,
            }
        }),
    }
}
