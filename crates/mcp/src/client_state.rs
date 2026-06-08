//! Snapshot do estado do cliente + predicados de disponibilidade de
//! tool, usados em duas frentes:
//!
//! - **`tools/list` filtrado** ([`ServerHandler::list_tools`]): o caller
//!   recebe só as tools que fazem sentido pro `client_id` corrente.
//! - **Defesa-em-profundidade** ([`require_enabled`]): cada handler
//!   reconfere antes de executar. Caller bem-comportado nunca cai
//!   nesse erro — o check existe pra caller mal-comportado ou pra
//!   reagir a race condition entre `list_tools` e `tools/call`.

use deadpool_postgres::Pool;
use pgsafe::sql;
use rmcp::model::CallToolResult;
use serde_json::json;
use uuid::Uuid;

use crate::state::AppState;

/// Flags do cliente necessárias pra decidir disponibilidade de tools.
/// Lê só o mínimo pra cada predicado caber numa única query barata.
/// O `Default` representa um lead novo vazio — usado em `tools/list`
/// quando o `client_id` é desconhecido.
#[derive(Debug, Clone, Default)]
pub struct ClientSnapshot {
    pub has_cpf: bool,
    pub has_cnpj: bool,
    pub quer_abrir_mei: Option<bool>,
    pub govbr_autenticado: bool,
    pub govbr_has_password: bool,
    pub recusado: bool,
}

pub async fn load_snapshot(pool: &Pool, client_id: Uuid) -> anyhow::Result<Option<ClientSnapshot>> {
    let row = sql!(
        pool,
        "SELECT
            (cpf            IS NOT NULL) AS has_cpf,
            (cnpj           IS NOT NULL) AS has_cnpj,
            quer_abrir_mei,
            (govbr_session  IS NOT NULL) AS govbr_autenticado,
            (govbr_password IS NOT NULL) AS govbr_has_password,
            (recusado_em    IS NOT NULL) AS recusado
         FROM zain.clients
         WHERE id = $client_id"
    )
    .fetch_optional()
    .await?;
    Ok(row.map(|r| ClientSnapshot {
        has_cpf: r.has_cpf,
        has_cnpj: r.has_cnpj,
        quer_abrir_mei: r.quer_abrir_mei,
        govbr_autenticado: r.govbr_autenticado,
        govbr_has_password: r.govbr_has_password,
        recusado: r.recusado,
    }))
}

/// Decide se uma tool faz sentido pro estado atual do cliente.
/// Replica os predicados `enabled_when` do agent original com as
/// extensões naturais (ex.: bloquear tools que mexem no lead depois
/// dele ter sido recusado).
pub fn tool_enabled(name: &str, s: &ClientSnapshot) -> bool {
    match name {
        // Sempre disponíveis.
        "get_client_state" | "save_cpf" | "buscar_cnae" => true,

        // Só faz sentido enquanto o lead não foi recusado.
        "save_quer_abrir_mei" => !s.recusado && !s.has_cnpj,
        "iniciar_pagamento" => {
            !s.recusado && s.has_cpf && (s.has_cnpj || s.quer_abrir_mei == Some(true))
        }
        "recusar_lead" => !s.recusado && s.has_cpf,

        // gov.br: equivalentes diretos dos predicados originais.
        "auth_govbr" => s.has_cpf && !s.govbr_autenticado,
        "auth_govbr_otp" => s.govbr_has_password && !s.govbr_autenticado,

        // abrir_empresa: copia `abrir_empresa::is_enabled` do agent.
        "abrir_empresa" => s.quer_abrir_mei == Some(true) && !s.has_cnpj && s.govbr_autenticado,

        // Tool desconhecida: deixa passar — o tool_router devolve o erro
        // padrão de "tool não encontrada" no `call_tool`.
        _ => true,
    }
}

/// Defesa-em-profundidade: chamado no início de cada handler de tool
/// com predicado restrito. Devolve `None` se a chamada pode prosseguir
/// e `Some(CallToolResult)` com erro estruturado caso contrário.
///
/// Caller que respeita `tools/list` filtrado nunca cai aqui — o check
/// é pra blindar contra caller mal-comportado e contra race entre
/// `list_tools` e `tools/call`.
pub async fn require_enabled(
    state: &AppState,
    tool: &str,
    client_id: Uuid,
) -> Option<CallToolResult> {
    let snapshot = match load_snapshot(&state.pool, client_id).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            return Some(CallToolResult::structured_error(json!({
                "status": "erro",
                "motivo": "cliente_nao_encontrado",
                "mensagem": format!("Cliente {client_id} não encontrado no cadastro."),
            })));
        }
        Err(e) => {
            tracing::warn!(%client_id, tool, error = %e, "require_enabled: falha ao carregar snapshot");
            return Some(CallToolResult::structured_error(json!({
                "status": "erro",
                "mensagem": format!("Falha ao validar pré-requisitos: {e}"),
            })));
        }
    };
    if tool_enabled(tool, &snapshot) {
        None
    } else {
        tracing::info!(
            %client_id,
            tool,
            ?snapshot,
            "require_enabled: tool não disponível no estado atual"
        );
        Some(CallToolResult::structured_error(json!({
            "status": "erro",
            "motivo": "pre_requisito_nao_atendido",
            "mensagem": format!(
                "Tool `{tool}` não está disponível no estado atual do cliente. Chame `get_client_state` ou `tools/list` (com `_meta.client_id`) pra ver o que faz sentido agora."
            ),
        })))
    }
}
