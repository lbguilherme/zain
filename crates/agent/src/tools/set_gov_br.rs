use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

use super::{Tool, ToolDef, ToolOutput, params_for, typed_sync_handler};

#[derive(Deserialize, JsonSchema)]
struct Args {
    /// Usuário Gov.br (geralmente CPF)
    usuario: String,
    /// Senha Gov.br
    senha: String,
}

pub fn tool() -> Tool {
    Tool {
        def: ToolDef {
            name: "set_gov_br",
            description: "Salva as credenciais Gov.br do lead. Colete somente quando a pessoa fornecer voluntariamente.",
            consequential: false,
            parameters: params_for::<Args>(),
        },
        handler: typed_sync_handler(|args: Args, mut props, memory| {
            props["gov_br_usuario"] = json!(args.usuario);
            props["gov_br_senha"] = json!(args.senha);
            ToolOutput {
                value: json!({ "status": "ok", "credenciais_salvas": true }),
                props,
                memory,
            }
        }),
    }
}
