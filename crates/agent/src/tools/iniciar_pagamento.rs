use cubos_sql::sql;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

use super::{Tool, ToolContext, ToolDef, ToolOutput, params_for, typed_handler};

#[derive(Deserialize, JsonSchema)]
struct Args {}

pub fn tool() -> Tool {
    Tool {
        def: ToolDef {
            name: "iniciar_pagamento",
            description: "Sinaliza que o lead está pronto pro cadastro de cartão de crédito. Requer CPF salvo e que o lead esteja num estado qualificado: ou já tem CNPJ MEI salvo (via save_cnpj com sucesso), ou `quer_abrir_mei = true`. Depois de chamar, seta `pagamento_solicitado_em` no cliente.",
            consequential: true,
            parameters: params_for::<Args>(),
        },
        handler: typed_handler(|ctx: ToolContext, _args: Args, memory| async move {
            let client_id = ctx.client_id;
            let row = match sql!(
                &ctx.pool,
                "SELECT cpf, cnpj, quer_abrir_mei FROM zain.clients WHERE id = $client_id"
            )
            .fetch_one()
            .await
            {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!(client_id = %ctx.client_id, error = %e, "iniciar_pagamento: falha ao ler cliente");
                    return ToolOutput::err(
                        json!({
                            "status": "erro",
                            "mensagem": format!("Falha ao ler cliente: {e}")
                        }),
                        memory,
                    );
                }
            };

            if row.cpf.is_none() {
                return ToolOutput::err(
                    json!({
                        "status": "erro",
                        "mensagem": "Dados insuficientes. Necessário salvar o CPF antes."
                    }),
                    memory,
                );
            }

            // Qualificação: o lead precisa estar num estado onde faz
            // sentido cadastrar o cartão. Ou já tem CNPJ MEI salvo
            // (save_cnpj só salva quando SIMEI confirma), ou declarou
            // intenção de abrir um novo MEI.
            let tem_cnpj = row.cnpj.is_some();
            let quer_abrir = row.quer_abrir_mei == Some(true);
            if !tem_cnpj && !quer_abrir {
                return ToolOutput::err(
                    json!({
                        "status": "erro",
                        "mensagem": "Lead não qualificado. Precisa ter CNPJ MEI salvo (save_cnpj) OU declarar intenção de abrir MEI (save_quer_abrir_mei=true)."
                    }),
                    memory,
                );
            }

            if let Err(e) = sql!(
                &ctx.pool,
                "UPDATE zain.clients
                 SET pagamento_solicitado_em = now(), updated_at = now()
                 WHERE id = $client_id"
            )
            .execute()
            .await
            {
                tracing::warn!(client_id = %ctx.client_id, error = %e, "iniciar_pagamento: falha ao marcar");
                return ToolOutput::err(
                    json!({
                        "status": "erro",
                        "mensagem": format!("Falha ao salvar: {e}")
                    }),
                    memory,
                );
            }

            ToolOutput::new(
                json!({ "status": "ok", "pagamento_solicitado": true }),
                memory,
            )
        }),
        must_use_tool_result: false,
    }
}
