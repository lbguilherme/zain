//! Tool `send_ccmei` — envia o PDF do Certificado de MEI (CCMEI) do
//! cliente via WhatsApp, lendo os bytes direto de
//! `zain.clients.mei_ccmei_pdf`. Fica escondida quando o cliente ainda
//! não tem um CCMEI salvo (a tool `auth_govbr` persiste o PDF quando
//! identifica MEI ativo, e `abrir_empresa` persiste após uma abertura
//! bem-sucedida).

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use cubos_sql::sql;
use deadpool_postgres::Pool;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use super::{Tool, ToolContext, ToolDef, ToolOutput, params_for, typed_handler};
use crate::dispatch::ClientRow;

/// Predicado: a tool só fica visível quando o cliente tem um CCMEI
/// (PDF) salvo. Ou seja, quando `auth_govbr` identificou MEI ativo ou
/// `abrir_empresa` acabou de gerar um CNPJ e persistiu o certificado.
pub fn is_enabled(client: &ClientRow) -> bool {
    client.has_mei_ccmei_pdf
}

#[derive(Deserialize, JsonSchema)]
struct Args {}

pub fn tool() -> Tool {
    Tool {
        def: ToolDef {
            name: "send_ccmei",
            description: "Envia o PDF do Certificado de MEI (CCMEI) do cliente pelo WhatsApp. Use logo após o cliente ser identificado como MEI pela primeira vez (via `auth_govbr`) ou logo após uma abertura nova (via `abrir_empresa`). Não precisa de argumentos — o PDF e o CNPJ são lidos do cadastro.",
            consequential: true,
            parameters: params_for::<Args>(),
        },
        handler: typed_handler(|ctx: ToolContext, _args: Args, memory| async move {
            match load_ccmei(&ctx.pool, ctx.client_id).await {
                Ok(Some((pdf, cnpj))) => {
                    let filename = match cnpj.as_deref() {
                        Some(c) if !c.is_empty() => format!("CCMEI_{c}.pdf"),
                        _ => "CCMEI.pdf".to_string(),
                    };
                    let media_data_url = format!(
                        "data:application/pdf;base64,{}",
                        BASE64_STANDARD.encode(&pdf)
                    );
                    match write_outbox_document(&ctx.pool, &ctx.chat_id, &media_data_url, &filename)
                        .await
                    {
                        Ok(()) => {
                            tracing::info!(
                                client_id = %ctx.client_id,
                                pdf_bytes = pdf.len(),
                                "send_ccmei: documento enfileirado no outbox"
                            );
                            ToolOutput::new(
                                json!({
                                    "status": "ok",
                                    "filename": filename,
                                }),
                                memory,
                            )
                        }
                        Err(e) => {
                            tracing::warn!(client_id = %ctx.client_id, error = %e, "send_ccmei: falha ao escrever outbox");
                            ToolOutput::err(
                                json!({
                                    "status": "erro",
                                    "mensagem": format!("Falha ao enfileirar CCMEI: {e}"),
                                }),
                                memory,
                            )
                        }
                    }
                }
                Ok(None) => ToolOutput::err(
                    json!({
                        "status": "erro",
                        "motivo": "ccmei_ausente",
                        "mensagem": "Nenhum CCMEI salvo pra este cliente. Rode `auth_govbr` ou `abrir_empresa` primeiro pra gerar o certificado.",
                    }),
                    memory,
                ),
                Err(e) => {
                    tracing::warn!(client_id = %ctx.client_id, error = %e, "send_ccmei: falha ao ler PDF");
                    ToolOutput::err(
                        json!({
                            "status": "erro",
                            "mensagem": format!("Falha ao ler CCMEI do banco: {e}"),
                        }),
                        memory,
                    )
                }
            }
        }),
        must_use_tool_result: false,
        enabled_when: Some(is_enabled),
    }
}

async fn load_ccmei(
    pool: &Pool,
    client_id: Uuid,
) -> anyhow::Result<Option<(Vec<u8>, Option<String>)>> {
    let row = sql!(
        pool,
        "SELECT mei_ccmei_pdf, cnpj
         FROM zain.clients
         WHERE id = $client_id"
    )
    .fetch_optional()
    .await?;
    Ok(row.and_then(|r| r.mei_ccmei_pdf.map(|pdf| (pdf, r.cnpj))))
}

async fn write_outbox_document(
    pool: &Pool,
    chat_id: &str,
    media: &str,
    filename: &str,
) -> anyhow::Result<()> {
    let content: Value = json!({
        "media": media,
        "filename": filename,
    });
    let content_type = "document";
    sql!(
        pool,
        "INSERT INTO whatsapp.outbox (chat_id, content_type, content)
         VALUES ($chat_id, $content_type, $content)"
    )
    .execute()
    .await?;
    Ok(())
}
