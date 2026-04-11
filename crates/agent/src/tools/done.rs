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
            description: "Encerra o turno do agent e o coloca em estado de espera — como se fosse dormir até o cliente mandar a próxima mensagem. Chame done() quando você já fez tudo que precisava fazer agora (enviou a mensagem de resposta, salvou os dados coletados, disparou as consultas necessárias) e NÃO há mais nenhuma ação pendente do seu lado — só falta o cliente responder. Não chame done() se ainda tem tool pra rodar nesse turno (ex: você mandou uma mensagem de espera mas ainda não chamou a tool lenta, ou recebeu resultado de uma consulta e ainda não respondeu ao cliente). Enquanto done() não é chamado, o agent continua iterando.",
            consequential: false,
            parameters: params_for::<Args>(),
        },
        handler: typed_sync_handler(|_args: Args, memory| {
            ToolOutput::new(json!({ "status": "ok" }), memory)
        }),
        must_use_tool_result: false,
    }
}
