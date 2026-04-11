use cubos_sql::sql;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

use super::{Tool, ToolContext, ToolDef, ToolOutput, params_for, pgfn, typed_handler};
use crate::validators;

#[derive(Deserialize, JsonSchema)]
struct Args {
    /// CPF (apenas números, 11 dígitos)
    cpf: String,
}

pub fn tool() -> Tool {
    Tool {
        def: ToolDef {
            name: "save_cpf",
            description: "Valida o CPF, consulta pendências cadastrais na PGFN e, **só se não houver pendência acima do limite**, salva o CPF no cadastro. A consulta PGFN é cacheada por 48h (chamadas repetidas do mesmo CPF são instantâneas). Demora 15-30s no cache miss — chame na MESMA resposta que o send_whatsapp_message de espera, em sequência, SEM done() no meio. Retorna `status: ok` quando salvou; `status: erro` + `motivo` quando o CPF não passou (inválido, pendência cadastral acima do limite, ou consulta falhou). No caso de pendência cadastral, recuse o lead gentilmente com `recusar_lead` e **nunca** mencione PGFN/dívida ativa/valor pro cliente — diga apenas 'pendência cadastral'.",
            consequential: true,
            parameters: params_for::<Args>(),
        },
        handler: typed_handler(|ctx: ToolContext, args: Args, memory| async move {
            if !validators::validar_cpf(&args.cpf) {
                return ToolOutput::err(
                    json!({
                        "status": "erro",
                        "mensagem": "CPF inválido — os dígitos verificadores não batem. Peça o CPF correto ao cliente de forma amigável."
                    }),
                    memory,
                );
            }
            let cpf_digits: String = args.cpf.chars().filter(|c| c.is_ascii_digit()).collect();

            // 1) Checa PGFN ANTES de salvar. Se a pessoa tem pendência
            //    acima do limite, a gente recusa o lead sem deixar o
            //    CPF grudado no cadastro.
            if let Err(err_value) = pgfn::check_debt(&ctx.pool, ctx.client_id, &cpf_digits).await {
                return ToolOutput::err(err_value, memory);
            }

            // 2) PGFN ok — persiste o CPF.
            let cpf: Option<&str> = Some(&cpf_digits);
            let client_id = ctx.client_id;
            match sql!(
                &ctx.pool,
                "UPDATE zain.clients SET cpf = $cpf, updated_at = now() WHERE id = $client_id"
            )
            .execute()
            .await
            {
                Ok(_) => ToolOutput::new(json!({ "status": "ok" }), memory),
                Err(e) => {
                    tracing::warn!(client_id = %ctx.client_id, error = %e, "save_cpf: falha ao salvar");
                    ToolOutput::err(
                        json!({ "status": "erro", "mensagem": format!("Falha ao salvar: {e}") }),
                        memory,
                    )
                }
            }
        }),
        must_use_tool_result: true,
    }
}
