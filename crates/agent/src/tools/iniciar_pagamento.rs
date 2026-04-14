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
            description: "Sinaliza que o lead está pronto pro cadastro de cartão de crédito. Requer CPF salvo, gov.br autenticado (via auth_govbr com sucesso) e `quer_abrir_mei` definido (save_quer_abrir_mei chamado com true ou false). Depois de chamar, seta `pagamento_solicitado_em` no cliente.",
            consequential: true,
            parameters: params_for::<Args>(),
        },
        handler: typed_handler(|ctx: ToolContext, _args: Args, memory| async move {
            let client_id = ctx.client_id;
            let row = match sql!(
                &ctx.pool,
                "SELECT cpf, quer_abrir_mei, govbr_session_valid_at
                 FROM zain.clients WHERE id = $client_id"
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

            // Gov.br tem que estar autenticado antes do pagamento: se
            // o login não funciona, não faz sentido cobrar a pessoa.
            // `govbr_session_valid_at` só é preenchido quando uma
            // autenticação real retornou `status: ok`.
            if row.govbr_session_valid_at.is_none() {
                return ToolOutput::err(
                    json!({
                        "status": "erro",
                        "mensagem": "Gov.br ainda não autenticado. Chame auth_govbr (e auth_govbr_otp se necessário) antes de iniciar o pagamento."
                    }),
                    memory,
                );
            }

            // Agente precisa ter estabelecido a situação de MEI
            // (sim ou não) via save_quer_abrir_mei. Sem esse sinal,
            // não dá pra saber se é caso de abertura ou gestão.
            if row.quer_abrir_mei.is_none() {
                return ToolOutput::err(
                    json!({
                        "status": "erro",
                        "mensagem": "Situação de MEI não confirmada. Pergunte ao cliente se já é MEI ou quer abrir, e chame save_quer_abrir_mei antes de iniciar o pagamento."
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
