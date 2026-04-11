use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

use super::{Tool, ToolDef, ToolOutput, params_for, typed_sync_handler};

#[derive(Deserialize, JsonSchema)]
struct Args {
    /// Endereço completo
    endereco: String,
}

pub fn tool() -> Tool {
    Tool {
        def: ToolDef {
            name: "set_endereco",
            description: "Salva o endereço do lead.",
            consequential: false,
            parameters: params_for::<Args>(),
        },
        handler: typed_sync_handler(|args: Args, mut props, memory| {
            props["endereco"] = json!(args.endereco);
            ToolOutput {
                value: json!({ "status": "ok" }),
                props,
                memory,
            }
        }),
    }
}
