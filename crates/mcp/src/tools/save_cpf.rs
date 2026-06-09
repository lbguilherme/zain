use pgsafe::sql;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use super::pgfn;
use crate::errlog::ErrChain;
use crate::state::AppState;
use crate::validators;

#[derive(Deserialize, JsonSchema)]
pub struct Args {
    /// CPF (apenas números, 11 dígitos)
    pub cpf: String,
}

pub async fn run(state: &AppState, client_id: Uuid, args: Args) -> Value {
    if !validators::validar_cpf(&args.cpf) {
        return json!({
            "status": "erro",
            "mensagem": "CPF inválido — os dígitos verificadores não batem. Peça o CPF correto ao cliente de forma amigável."
        });
    }
    let cpf_digits: String = args.cpf.chars().filter(|c| c.is_ascii_digit()).collect();

    // 1) Checa PGFN ANTES de salvar. Se a pessoa tem pendência acima do
    //    limite, a gente recusa o lead sem deixar o CPF grudado no
    //    cadastro.
    if let Err(err_value) = pgfn::check_debt(&state.pool, client_id, &cpf_digits).await {
        return err_value;
    }

    // 2) PGFN ok — persiste o CPF.
    match sql!(
        &state.pool,
        "UPDATE zain.clients SET cpf = $cpf_digits, updated_at = now() WHERE id = $client_id"
    )
    .execute()
    .await
    {
        Ok(_) => json!({ "status": "ok" }),
        Err(e) => {
            tracing::warn!(%client_id, error = %e.chain_string(), "save_cpf: falha ao salvar");
            json!({ "status": "erro", "mensagem": "Não consegui salvar o CPF no banco agora. Tente de novo em instantes." })
        }
    }
}
