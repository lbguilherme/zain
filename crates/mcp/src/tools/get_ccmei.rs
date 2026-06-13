//! Tool `get_ccmei`: devolve o PDF do Certificado da Condição de MEI
//! (CCMEI) do próprio lead como resource inline (embedded) no
//! resultado — o caller recebe o blob direto, sem precisar de
//! `resources/read`.
//!
//! Ownership é implícita: o PDF sai da linha do `client_id` extraído
//! do `_meta` da chamada — não há como pedir o CCMEI de outro lead.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use deadpool_postgres::Pool;
use pgsafe::sql;
use rmcp::model::{CallToolResult, Content, ResourceContents};
use schemars::JsonSchema;
use serde::Deserialize;
use uuid::Uuid;

use crate::errlog::ErrChain;
use crate::state::AppState;

#[derive(Deserialize, JsonSchema)]
pub struct Args {}

/// URI identificadora do CCMEI no resource embutido. Não é mais
/// servida via `resources/read` — existe só como identificador estável
/// do anexo dentro do resultado da tool.
fn uri(cnpj: &str) -> String {
    format!("zain://mei/{cnpj}/ccmei.pdf")
}

pub async fn run(state: &AppState, client_id: Uuid, _args: Args) -> CallToolResult {
    let row = match load_ccmei(&state.pool, client_id).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            return CallToolResult::structured_error(serde_json::json!({
                "status": "erro",
                "motivo": "ccmei_indisponivel",
                "mensagem": "Este lead não tem CCMEI salvo. O PDF só fica disponível depois que o `auth_govbr` encontra um MEI ativo ou o `abrir_empresa` conclui a inscrição.",
            }));
        }
        Err(e) => {
            tracing::warn!(%client_id, error = %e.chain_string(), "get_ccmei: falha ao ler PDF");
            return CallToolResult::structured_error(serde_json::json!({
                "status": "erro",
                "mensagem": "Não consegui ler o CCMEI do banco agora. Tente de novo em instantes.",
            }));
        }
    };

    let pdf_bytes = row.pdf.len();
    let uri = uri(&row.cnpj);
    tracing::info!(%client_id, cnpj = %row.cnpj, pdf_bytes, "get_ccmei: servindo PDF inline");

    let blob = BASE64_STANDARD.encode(&row.pdf);
    let contents = ResourceContents::blob(blob, &uri).with_mime_type("application/pdf");
    CallToolResult::success(vec![Content::resource(contents)])
}

struct CcmeiRow {
    cnpj: String,
    pdf: Vec<u8>,
}

async fn load_ccmei(pool: &Pool, client_id: Uuid) -> anyhow::Result<Option<CcmeiRow>> {
    let row = sql!(
        pool,
        "SELECT cnpj, mei_ccmei_pdf
         FROM zain.clients
         WHERE id = $client_id
           AND cnpj IS NOT NULL
           AND mei_ccmei_pdf IS NOT NULL"
    )
    .fetch_optional()
    .await?;
    Ok(row.and_then(|r| match (r.cnpj, r.mei_ccmei_pdf) {
        (Some(cnpj), Some(pdf)) => Some(CcmeiRow { cnpj, pdf }),
        _ => None,
    }))
}
