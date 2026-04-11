use cubos_sql::sql;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

use super::{Tool, ToolContext, ToolDef, ToolOutput, params_for, typed_handler};

#[derive(Deserialize, JsonSchema)]
struct Args {
    /// Código CNAE da atividade da pessoa (ex: "4520-0/01" ou "4520001").
    /// Precisa bater com uma subclasse existente — vem normalmente de
    /// `buscar_cnae_por_atividade` ou `consultar_cnae_por_codigo`.
    cnae: String,
}

pub fn tool() -> Tool {
    Tool {
        def: ToolDef {
            name: "save_atividade",
            description: "Salva o código CNAE da atividade do lead. A descrição da atividade é derivada via join com a tabela de CNAEs — não precisa passar descrição. Use depois que `buscar_cnae_por_atividade` ou `consultar_cnae_por_codigo` retornar um CNAE MEI-compatível.",
            consequential: false,
            parameters: params_for::<Args>(),
        },
        handler: typed_handler(|ctx: ToolContext, args: Args, memory| async move {
            let cnae_digits: String = args.cnae.chars().filter(|c| c.is_ascii_digit()).collect();
            if cnae_digits.is_empty() {
                return ToolOutput::err(
                    json!({
                        "status": "erro",
                        "mensagem": "CNAE vazio — passe um código válido (ex: '4520-0/01')."
                    }),
                    memory,
                );
            }
            let cnae_opt: Option<&str> = Some(&cnae_digits);
            let client_id = ctx.client_id;
            match sql!(
                &ctx.pool,
                "UPDATE zain.clients SET cnae = $cnae_opt, updated_at = now() WHERE id = $client_id"
            )
            .execute()
            .await
            {
                Ok(_) => ToolOutput::new(json!({ "status": "ok" }), memory),
                Err(e) => {
                    tracing::warn!(client_id = %ctx.client_id, error = %e, "save_atividade: falha ao salvar");
                    ToolOutput::err(
                        json!({ "status": "erro", "mensagem": format!("Falha ao salvar: {e}") }),
                        memory,
                    )
                }
            }
        }),
        must_use_tool_result: false,
    }
}
