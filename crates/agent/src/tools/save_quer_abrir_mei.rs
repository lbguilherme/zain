use cubos_sql::sql;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

use super::{Tool, ToolContext, ToolDef, ToolOutput, params_for, typed_handler};

#[derive(Deserialize, JsonSchema)]
struct Args {
    /// `true` se a pessoa quer abrir um MEI novo (ainda não é MEI).
    /// `false` se a pessoa já é MEI (não precisa abrir um novo).
    /// Registra a situação de MEI do cliente.
    quer_abrir_mei: bool,
}

pub fn tool() -> Tool {
    Tool {
        def: ToolDef {
            name: "save_quer_abrir_mei",
            description: "Registra a situação de MEI do cliente. Use `true` quando ela disser que quer abrir/começar um MEI (ainda não é MEI). Use `false` quando ela disser que já é MEI. Chame esta tool depois que o gov.br estiver autenticado e o cliente responder a pergunta 'já é MEI ou quer abrir?'. A tool `iniciar_pagamento` exige que esse campo esteja definido.",
            consequential: false,
            parameters: params_for::<Args>(),
        },
        handler: typed_handler(|ctx: ToolContext, args: Args, memory| async move {
            let quer_abrir_mei: Option<bool> = Some(args.quer_abrir_mei);
            let client_id = ctx.client_id;
            match sql!(
                &ctx.pool,
                "UPDATE zain.clients
                 SET quer_abrir_mei = $quer_abrir_mei, updated_at = now()
                 WHERE id = $client_id"
            )
            .execute()
            .await
            {
                Ok(_) => ToolOutput::new(json!({ "status": "ok" }), memory),
                Err(e) => {
                    tracing::warn!(client_id = %ctx.client_id, error = %e, "save_quer_abrir_mei: falha ao salvar");
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
