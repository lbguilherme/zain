use pgsafe::sql;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::errlog::ErrChain;
use crate::state::AppState;

#[derive(Deserialize, JsonSchema)]
pub struct Args {}

pub async fn run(state: &AppState, client_id: Uuid, _args: Args) -> Value {
    let row = match sql!(
        &state.pool,
        "SELECT cpf, cnpj, quer_abrir_mei FROM zain.clients WHERE id = $client_id"
    )
    .fetch_one()
    .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(%client_id, error = %e.chain_string(), "iniciar_pagamento: falha ao ler cliente");
            return json!({
                "status": "erro",
                "mensagem": "Não consegui ler o cadastro do cliente agora. Tente de novo em instantes."
            });
        }
    };

    if row.cpf.is_none() {
        return json!({
            "status": "erro",
            "mensagem": "Dados insuficientes. Necessário salvar o CPF antes."
        });
    }

    // Qualificação: o lead precisa estar num estado onde faz sentido
    // cadastrar o cartão. Ou já tem CNPJ MEI salvo (persistido
    // automaticamente pelo `auth_govbr` quando a consulta CCMEI
    // confirma MEI ativo), ou declarou intenção de abrir um novo MEI.
    let tem_cnpj = row.cnpj.is_some();
    let quer_abrir = row.quer_abrir_mei == Some(true);
    if !tem_cnpj && !quer_abrir {
        return json!({
            "status": "erro",
            "mensagem": "Lead não qualificado. Precisa ter MEI confirmado (o `auth_govbr` persiste o CNPJ automaticamente quando encontra um MEI ativo) OU declarar intenção de abrir MEI (save_quer_abrir_mei=true)."
        });
    }

    if let Err(e) = sql!(
        &state.pool,
        "UPDATE zain.clients
         SET pagamento_solicitado_em = now(), updated_at = now()
         WHERE id = $client_id"
    )
    .execute()
    .await
    {
        tracing::warn!(%client_id, error = %e.chain_string(), "iniciar_pagamento: falha ao marcar");
        return json!({
            "status": "erro",
            "mensagem": "Não consegui salvar no banco agora. Tente de novo em instantes."
        });
    }

    json!({ "status": "ok", "pagamento_solicitado": true })
}
