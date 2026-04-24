use cubos_sql::sql;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

use super::{Tool, ToolContext, ToolDef, ToolOutput, params_for, typed_handler};

#[derive(Deserialize, JsonSchema)]
struct Args {
    /// `true` se a pessoa tem intenção de abrir um MEI novo (ou seja,
    /// ela ainda não possui CNPJ MEI). `false` se ela não quer abrir
    /// (já tem, ou desistiu). Registro de *intent*, não de posse.
    quer_abrir_mei: bool,
}

pub fn tool() -> Tool {
    Tool {
        def: ToolDef {
            name: "save_quer_abrir_mei",
            description: "Registra se a pessoa tem intenção de abrir um MEI novo. Use `true` quando ela disser que quer abrir/começar um MEI (e ainda não tem CNPJ). Use `false` quando ela desistir. Quando a pessoa diz que já tem MEI, NÃO chame esta tool — o `auth_govbr` já persiste o CNPJ automaticamente quando encontra um MEI ativo no CPF do cliente.",
            consequential: false,
            parameters: params_for::<Args>(),
        },
        handler: typed_handler(|ctx: ToolContext, args: Args, memory| async move {
            let quer_abrir_mei = args.quer_abrir_mei;
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
        enabled_when: None,
    }
}
