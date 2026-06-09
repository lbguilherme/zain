//! Resource MCP do Certificado de MEI (CCMEI).
//!
//! URI canônica: `zain://mei/<cnpj>/ccmei.pdf`. O conteúdo é o PDF do
//! certificado, em base64, mime `application/pdf`.
//!
//! Ownership: tanto a listagem (`resources/list`) quanto a leitura
//! (`resources/read`) exigem `_meta.client_id` na request. O `read`
//! valida que o `client_id` é dono daquele CNPJ
//! (`WHERE id = $client_id AND cnpj = $cnpj`), impedindo um caller de
//! pegar o CCMEI de outro lead só pela URI.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use deadpool_postgres::Pool;
use pgsafe::sql;
use rmcp::ErrorData;
use rmcp::model::{AnnotateAble, RawResource, ReadResourceResult, Resource, ResourceContents};
use uuid::Uuid;

use crate::state::AppState;

/// URI canônica do CCMEI por CNPJ. O parser
/// [`parse_uri`] é o inverso desta função.
pub fn uri(cnpj: &str) -> String {
    format!("zain://mei/{cnpj}/ccmei.pdf")
}

/// Extrai o CNPJ (string de dígitos) de uma URI no formato
/// `zain://mei/<cnpj>/ccmei.pdf`. Retorna `None` pra qualquer outro
/// formato ou se o CNPJ não for puramente numérico.
pub fn parse_uri(uri: &str) -> Option<String> {
    let cnpj = uri
        .strip_prefix("zain://mei/")?
        .strip_suffix("/ccmei.pdf")?;
    if cnpj.is_empty() || !cnpj.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(cnpj.to_string())
}

/// Lista o CCMEI do cliente pra `resources/list`. Devolve um único
/// Resource quando o cliente tem CCMEI salvo; vazio caso contrário.
pub async fn list_for_client(
    state: &AppState,
    client_id: Uuid,
) -> Result<Vec<Resource>, ErrorData> {
    let meta = match load_meta(&state.pool, client_id).await {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!(%client_id, error = %crate::errlog::anyhow_chain(&e), "ccmei list: falha ao ler metadata");
            return Err(ErrorData::internal_error(
                "Não consegui listar o CCMEI no banco agora.".to_string(),
                None,
            ));
        }
    };
    let Some(meta) = meta else {
        return Ok(Vec::new());
    };

    let filename = format!("CCMEI_{}.pdf", meta.cnpj);
    let pdf_size = u32::try_from(meta.pdf_size).unwrap_or(u32::MAX);
    let resource = RawResource::new(uri(&meta.cnpj), filename.clone())
        .with_title(format!("Certificado MEI ({filename})"))
        .with_description("PDF do Certificado da Condição de Microempreendedor Individual (CCMEI).")
        .with_mime_type("application/pdf")
        .with_size(pdf_size);
    Ok(vec![resource.no_annotation()])
}

/// `resources/read` pro CCMEI. Valida ownership: só devolve o PDF se
/// `client_id` (vindo do `_meta` da request) é o dono daquele CNPJ.
pub async fn read(
    state: &AppState,
    client_id: Uuid,
    cnpj: &str,
) -> Result<ReadResourceResult, ErrorData> {
    let pdf = match load_pdf_owned(&state.pool, client_id, cnpj).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            // Não distinguimos entre "CNPJ não existe" e "cliente não é
            // dono" — a mensagem genérica evita oracle de existência de
            // CCMEI por CNPJ.
            tracing::warn!(
                %client_id,
                cnpj,
                "ccmei read: não encontrado ou cliente não é dono"
            );
            return Err(ErrorData::invalid_params(
                format!("CCMEI não disponível pra {cnpj}"),
                None,
            ));
        }
        Err(e) => {
            tracing::warn!(%client_id, cnpj, error = %crate::errlog::anyhow_chain(&e), "ccmei read: falha ao ler PDF");
            return Err(ErrorData::internal_error(
                "Não consegui ler o CCMEI do banco agora.".to_string(),
                None,
            ));
        }
    };

    let uri = uri(cnpj);
    let blob = BASE64_STANDARD.encode(&pdf);
    tracing::info!(
        %client_id,
        cnpj,
        pdf_bytes = pdf.len(),
        %uri,
        "ccmei read: servindo blob"
    );
    let contents = ResourceContents::blob(blob, uri).with_mime_type("application/pdf");
    Ok(ReadResourceResult::new(vec![contents]))
}

struct CcmeiMeta {
    cnpj: String,
    pdf_size: i32,
}

async fn load_meta(pool: &Pool, client_id: Uuid) -> anyhow::Result<Option<CcmeiMeta>> {
    let row = sql!(
        pool,
        "SELECT cnpj, octet_length(mei_ccmei_pdf) AS pdf_size
         FROM zain.clients
         WHERE id = $client_id
           AND cnpj IS NOT NULL
           AND mei_ccmei_pdf IS NOT NULL"
    )
    .fetch_optional()
    .await?;
    Ok(row.and_then(|r| match (r.cnpj, r.pdf_size) {
        (Some(cnpj), Some(pdf_size)) => Some(CcmeiMeta { cnpj, pdf_size }),
        _ => None,
    }))
}

/// Lookup com check de ownership numa única query: só retorna o PDF se
/// `client_id` é dono de `cnpj`. Sem JOIN duplo, sem TOCTOU.
async fn load_pdf_owned(
    pool: &Pool,
    client_id: Uuid,
    cnpj: &str,
) -> anyhow::Result<Option<Vec<u8>>> {
    let row = sql!(
        pool,
        "SELECT mei_ccmei_pdf
         FROM zain.clients
         WHERE id   = $client_id
           AND cnpj = $cnpj"
    )
    .fetch_optional()
    .await?;
    Ok(row.and_then(|r| r.mei_ccmei_pdf))
}
