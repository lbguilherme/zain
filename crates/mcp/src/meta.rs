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
/// Útil em endpoints como `tools/list` que devem responder
/// gracefully sem identidade (devolvendo a lista sem filtro).
pub fn extract_client_id_opt(meta: &Meta) -> Option<Uuid> {
    meta.get("cubos-agent")?
        .get("userId")?
        .as_str()
        .and_then(|s| Uuid::parse_str(s).ok())
}

/// Dados de contato que o canal envia em
/// `_meta.cubos-agent.conversationMetadata` (canal whapi/WhatsApp).
/// Ausentes em canais que não os fornecem.
pub struct ContactInfo {
    pub phone: Option<String>,
    pub name: Option<String>,
}

/// Lê `whapi_phone`/`whapi_name` do `conversationMetadata`, quando
/// presentes. Nunca falha — meta sem os campos vira `None`.
pub fn extract_contact_info(meta: &Meta) -> ContactInfo {
    let conv = meta
        .get("cubos-agent")
        .and_then(|a| a.get("conversationMetadata"));
    let get = |key: &str| {
        conv.and_then(|c| c.get(key))
            .and_then(|v| v.as_str())
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };
    ContactInfo {
        phone: get("whapi_phone"),
        name: get("whapi_name"),
    }
}

/// Garante que existe uma linha em `zain.clients` com `id = client_id`
/// e mantém `phone`/`name` atualizados com o que o canal informou.
/// Idempotente: contato ausente não apaga o que já está salvo
/// (COALESCE), e linha já em dia vira no-op (não toca `updated_at`).
pub async fn ensure_client_exists(
    pool: &Pool,
    client_id: Uuid,
    contact: &ContactInfo,
) -> anyhow::Result<()> {
    let phone = contact.phone.as_deref();
    let name = contact.name.as_deref();
    sql!(
        pool,
        "INSERT INTO zain.clients (id, phone, name)
         VALUES ($client_id, $phone, $name)
         ON CONFLICT (id) DO UPDATE
         SET phone      = COALESCE(EXCLUDED.phone, zain.clients.phone),
             name       = COALESCE(EXCLUDED.name,  zain.clients.name),
             updated_at = now()
         WHERE zain.clients.phone IS DISTINCT FROM COALESCE(EXCLUDED.phone, zain.clients.phone)
            OR zain.clients.name  IS DISTINCT FROM COALESCE(EXCLUDED.name,  zain.clients.name)"
    )
    .execute()
    .await?;
    Ok(())
}

/// Atalho usado pelos handlers de tool: extrai o `userId` do meta,
/// auto-cria o cliente correspondente caso ainda não exista no banco e
/// preenche `phone`/`name` com o que o canal mandou no meta.
/// Falha de inserção é convertida pra `ErrorData::internal_error` pra
/// o caller MCP receber o motivo direto.
pub async fn extract_and_ensure_client_id(pool: &Pool, meta: &Meta) -> Result<Uuid, ErrorData> {
    let client_id = extract_client_id(meta)?;
    let contact = extract_contact_info(meta);
    ensure_client_exists(pool, client_id, &contact)
        .await
        .map_err(|e| {
            tracing::warn!(%client_id, error = %crate::errlog::anyhow_chain(&e), "ensure_client_exists falhou");
            ErrorData::internal_error("Não consegui registrar o cliente no banco agora.".to_string(), None)
        })?;
    Ok(client_id)
}
