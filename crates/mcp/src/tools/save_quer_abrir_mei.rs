use pgsafe::sql;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::errlog::ErrChain;
use crate::state::AppState;

#[derive(Deserialize, JsonSchema)]
pub struct Args {
    /// `true` se a pessoa tem intenção de abrir um MEI novo (ou seja,
    /// ela ainda não possui CNPJ MEI). `false` se ela não quer abrir
    /// (já tem, ou desistiu). Registro de *intent*, não de posse.
    pub quer_abrir_mei: bool,
}

pub async fn run(state: &AppState, client_id: Uuid, args: Args) -> Value {
    let quer_abrir_mei = args.quer_abrir_mei;
    match sql!(
        &state.pool,
        "UPDATE zain.clients
         SET quer_abrir_mei = $quer_abrir_mei, updated_at = now()
         WHERE id = $client_id"
    )
    .execute()
    .await
    {
        Ok(_) => json!({ "status": "ok" }),
        Err(e) => {
            tracing::warn!(%client_id, error = %e.chain_string(), "save_quer_abrir_mei: falha ao salvar");
            json!({ "status": "erro", "mensagem": "Não consegui salvar no banco agora. Tente de novo em instantes." })
        }
    }
}
