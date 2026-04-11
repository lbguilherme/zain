use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

use super::{Tool, ToolDef, ToolOutput, params_for, typed_sync_handler};

#[derive(Deserialize, JsonSchema)]
struct Args {
    /// Texto da anotação
    texto: String,
}

pub fn tool() -> Tool {
    Tool {
        def: ToolDef {
            name: "anotar",
            description: "Salva uma anotação livre sobre o cliente na memória. Use para registrar contexto relevante da conversa.",
            consequential: false,
            parameters: params_for::<Args>(),
        },
        handler: typed_sync_handler(|args: Args, props, mut memory| {
            let existing = memory
                .get("anotacoes")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let updated = if existing.is_empty() {
                args.texto
            } else {
                format!("{existing}\n{}", args.texto)
            };
            memory["anotacoes"] = json!(updated);
            ToolOutput {
                value: json!({ "status": "ok", "anotacao_salva": true }),
                props,
                memory,
            }
        }),
    }
}
