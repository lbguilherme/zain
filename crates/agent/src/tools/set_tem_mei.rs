use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

use super::{Tool, ToolDef, ToolOutput, params_for, typed_sync_handler};

#[derive(Deserialize, JsonSchema)]
struct Args {
    /// true se já tem MEI, false se não tem
    tem_mei: bool,
}

pub fn tool() -> Tool {
    Tool {
        def: ToolDef {
            name: "set_tem_mei",
            description: "Marca se a pessoa já possui MEI ou não.",
            consequential: false,
            parameters: params_for::<Args>(),
        },
        handler: typed_sync_handler(|args: Args, mut props, memory| {
            props["tem_mei"] = json!(args.tem_mei);
            ToolOutput {
                value: json!({ "status": "ok" }),
                props,
                memory,
            }
        }),
    }
}
