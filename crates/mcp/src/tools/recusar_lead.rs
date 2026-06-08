use pgsafe::sql;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::state::AppState;

#[derive(Deserialize, JsonSchema)]
pub struct Args {
    /// Motivo da recusa em linguagem direta (ex: 'CNPJ optante Simples Nacional, não SIMEI' ou 'atividade regulamentada não permitida pra MEI')
    pub motivo: String,
}

pub async fn run(state: &AppState, client_id: Uuid, args: Args) -> Value {
    let motivo = &args.motivo;
    match sql!(
        &state.pool,
        "UPDATE zain.clients
         SET recusa_motivo = $motivo,
             recusado_em   = now(),
             updated_at    = now()
         WHERE id = $client_id"
    )
    .execute()
    .await
    {
        Ok(_) => json!({ "status": "ok", "recusado": true }),
        Err(e) => {
            tracing::warn!(%client_id, error = %e, "recusar_lead: falha ao salvar");
            json!({ "status": "erro", "mensagem": format!("Falha ao salvar: {e}") })
        }
    }
}
