use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

use super::{Tool, ToolDef, ToolOutput, params_for, typed_sync_handler};

#[derive(Deserialize, JsonSchema)]
struct Args {}

/// Sinaliza o fim do turno do LLM. O workflow detecta pelo nome
/// (`"done"`) e quebra o loop de tool calls — o handler em si é uma
/// no-op que só ecoa um status.
pub fn tool() -> Tool {
    Tool {
        def: ToolDef {
            name: "done",
            description: "Chame esta ferramenta quando terminar de agir. Depois de enviar sua(s) mensagem(ns) ao cliente e salvar os dados necessários, chame done() para encerrar.",
            consequential: false,
            parameters: params_for::<Args>(),
        },
        handler: typed_sync_handler(|_args: Args, props, memory| ToolOutput {
            value: json!({ "status": "ok" }),
            props,
            memory,
        }),
    }
}
