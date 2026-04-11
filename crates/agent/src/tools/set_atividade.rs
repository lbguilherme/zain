use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

use super::{Tool, ToolDef, ToolOutput, params_for, typed_sync_handler};

#[derive(Deserialize, JsonSchema)]
struct Args {
    /// Descrição da atividade (ex: 'vendo doces artesanais')
    descricao: String,
    /// Código CNAE, se conhecido
    #[serde(default)]
    cnae: Option<String>,
}

pub fn tool() -> Tool {
    Tool {
        def: ToolDef {
            name: "set_atividade",
            description: "Salva a descrição da atividade e opcionalmente o CNAE.",
            consequential: false,
            parameters: params_for::<Args>(),
        },
        handler: typed_sync_handler(|args: Args, mut props, memory| {
            props["atividade_descricao"] = json!(args.descricao);
            if let Some(cnae) = args.cnae {
                props["cnae"] = json!(cnae);
            }
            ToolOutput {
                value: json!({ "status": "ok" }),
                props,
                memory,
            }
        }),
    }
}
