use deadpool_postgres::Pool;
use pgsafe::sql;
use rmcp::ErrorData;
use rmcp::model::Meta;
use uuid::Uuid;

/// Extrai o identificador interno do usuário a partir do `_meta` da
/// chamada MCP. O caller (cubos-agent) envia identidade aninhada:
///
/// ```json
/// {
///   "cubos-agent": {
///     "userId": "<uuid>",
///     "userExternalId": "...",
///     "conversationId": "...",
///     "tenantSlug": "...",
///     "channelType": "...",
///     "agentSlug": "...",
///     "conversationMetadata": { ... }
///   }
/// }
/// ```
///
/// Esse `userId` é usado diretamente como `client_id` na tabela
/// `zain.clients` — quem garante a unicidade é o agente, não a gente.
pub fn extract_client_id(meta: &Meta) -> Result<Uuid, ErrorData> {
    let agent = meta.get("cubos-agent").ok_or_else(|| {
        ErrorData::invalid_params(
            "Meta deve conter `cubos-agent.userId` (UUID) identificando o cliente",
            None,
        )
    })?;
    let raw = agent.get("userId").ok_or_else(|| {
        ErrorData::invalid_params(
            "Meta deve conter `cubos-agent.userId` (UUID) identificando o cliente",
            None,
        )
    })?;
    let s = raw.as_str().ok_or_else(|| {
        ErrorData::invalid_params("Meta.cubos-agent.userId deve ser string (UUID)", None)
    })?;
    Uuid::parse_str(s).map_err(|e| {
        ErrorData::invalid_params(format!("Meta.cubos-agent.userId inválido: {e}"), None)
    })
}

/// Versão opcional de [`extract_client_id`]: devolve `None` quando o
/// caminho `_meta.cubos-agent.userId` está ausente ou malformado.
/// Útil em endpoints como `resources/list` que devem responder
/// gracefully sem identidade (devolvendo lista vazia, por exemplo).
pub fn extract_client_id_opt(meta: &Meta) -> Option<Uuid> {
    meta.get("cubos-agent")?
        .get("userId")?
        .as_str()
        .and_then(|s| Uuid::parse_str(s).ok())
}

/// Garante que existe uma linha em `zain.clients` com `id = client_id`.
/// Idempotente: se já existir, não toca em nada. Se for criação nova,
/// fica só com o `id` preenchido — `chat_id`, `phone` e `name` ficam
/// nulos até alguma origem (WhatsApp, conversa) preencher.
pub async fn ensure_client_exists(pool: &Pool, client_id: Uuid) -> anyhow::Result<()> {
    sql!(
        pool,
        "INSERT INTO zain.clients (id)
         VALUES ($client_id)
         ON CONFLICT (id) DO NOTHING"
    )
    .execute()
    .await?;
    Ok(())
}

/// Atalho usado pelos handlers de tool: extrai o `userId` do meta e
/// auto-cria o cliente correspondente caso ainda não exista no banco.
/// Falha de inserção é convertida pra `ErrorData::internal_error` pra
/// o caller MCP receber o motivo direto.
pub async fn extract_and_ensure_client_id(pool: &Pool, meta: &Meta) -> Result<Uuid, ErrorData> {
    let client_id = extract_client_id(meta)?;
    ensure_client_exists(pool, client_id).await.map_err(|e| {
        tracing::warn!(%client_id, error = %crate::errlog::anyhow_chain(&e), "ensure_client_exists falhou");
        ErrorData::internal_error("Não consegui registrar o cliente no banco agora.".to_string(), None)
    })?;
    Ok(client_id)
}
