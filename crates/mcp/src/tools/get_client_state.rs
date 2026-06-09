//! Tool `get_client_state`: devolve o estado atual do cliente em
//! formato de prompt, pronto pra ser injetado no system/user prompt
//! do caller.
//!
//! Espelha o bloco que o agent original construía em
//! `format_dados_coletados` (`crates/agent/src/prompt.rs`): contato,
//! dados coletados, estado gov.br, MEI/CCMEI. Se o cliente tem CCMEI
//! salvo, o texto aponta pra URI do resource MCP
//! (`zain://mei/<cnpj>/ccmei.pdf`) pra o caller saber que pode baixar
//! via `resources/read`.

use pgsafe::sql;
use rmcp::model::{CallToolResult, Content};
use rpa::govbr::Nivel;
use schemars::JsonSchema;
use serde::Deserialize;
use uuid::Uuid;

use crate::resources::ccmei;
use crate::state::AppState;

#[derive(Deserialize, JsonSchema)]
pub struct Args {}

pub async fn run(state: &AppState, client_id: Uuid, _args: Args) -> CallToolResult {
    let row = match sql!(
        &state.pool,
        "SELECT
            phone,
            name,
            cpf,
            cnpj,
            quer_abrir_mei,
            pagamento_solicitado_em,
            recusa_motivo,
            recusado_em,
            govbr_nome,
            govbr_nivel,
            (govbr_session  IS NOT NULL) AS govbr_autenticado,
            (govbr_password IS NOT NULL) AS govbr_has_password,
            govbr_otp_pendente,
            (mei_ccmei_pdf  IS NOT NULL) AS has_mei_ccmei_pdf,
            mei_pode_abrir,
            mei_impedimento_motivo,
            mei_consultado_em
         FROM zain.clients
         WHERE id = $client_id"
    )
    .fetch_optional()
    .await
    {
        Ok(Some(r)) => r,
        Ok(None) => {
            return CallToolResult::structured_error(serde_json::json!({
                "status": "erro",
                "motivo": "cliente_nao_encontrado",
                "mensagem": format!("Cliente {client_id} não encontrado no cadastro."),
            }));
        }
        Err(e) => {
            tracing::warn!(%client_id, error = %e, "get_client_state: falha ao ler cliente");
            return CallToolResult::structured_error(serde_json::json!({
                "status": "erro",
                "mensagem": format!("Falha ao ler cliente: {e}"),
            }));
        }
    };

    let contact_name = row.name.as_deref().unwrap_or("(desconhecido)");
    let contact_phone = row.phone.as_deref().unwrap_or("(desconhecido)");
    let dados_coletados = format_dados_coletados(
        row.cpf.as_deref(),
        row.cnpj.as_deref(),
        row.quer_abrir_mei,
        row.pagamento_solicitado_em.as_ref().map(|t| t.to_rfc3339()),
        row.recusa_motivo.as_deref(),
        row.recusado_em.as_ref().map(|t| t.to_rfc3339()),
        row.govbr_autenticado,
        row.govbr_has_password,
        row.govbr_otp_pendente,
        row.govbr_nome.as_deref(),
        row.govbr_nivel,
        row.has_mei_ccmei_pdf,
        row.mei_pode_abrir,
        row.mei_impedimento_motivo.as_deref(),
        row.mei_consultado_em.as_ref().map(|t| t.to_rfc3339()),
    );

    let text = format!(
        "Informações do contato:\n\
         - Nome no WhatsApp: {contact_name}\n\
         - Telefone: {contact_phone}\n\
         \n\
         Dados coletados até agora:\n\
         {dados_coletados}"
    );

    CallToolResult::success(vec![Content::text(text)])
}

#[allow(clippy::too_many_arguments)]
fn format_dados_coletados(
    cpf: Option<&str>,
    cnpj: Option<&str>,
    quer_abrir_mei: Option<bool>,
    pagamento_solicitado_em: Option<String>,
    recusa_motivo: Option<&str>,
    recusado_em: Option<String>,
    govbr_autenticado: bool,
    govbr_has_password: bool,
    govbr_otp_pendente: bool,
    govbr_nome: Option<&str>,
    govbr_nivel: Option<Nivel>,
    has_mei_ccmei_pdf: bool,
    mei_pode_abrir: Option<bool>,
    mei_impedimento_motivo: Option<&str>,
    mei_consultado_em: Option<String>,
) -> String {
    let mut lines: Vec<String> = Vec::new();

    if let Some(cpf) = cpf {
        lines.push(format!("- CPF: {cpf}"));
        if govbr_autenticado {
            let mut detalhes: Vec<String> = Vec::new();
            if let Some(nome) = govbr_nome {
                detalhes.push(format!("nome \"{nome}\""));
            }
            if let Some(nivel) = govbr_nivel {
                detalhes.push(format!("nível {}", nivel.as_str()));
            }
            if detalhes.is_empty() {
                lines.push("- gov.br: autenticado".into());
            } else {
                lines.push(format!("- gov.br: autenticado ({})", detalhes.join(", ")));
            }
        } else if govbr_otp_pendente {
            // Sessão limpa + flag setada = último login parou no 2FA. O
            // worker de background não vai relogar sozinho; depende do
            // cliente gerar o código no app gov.br.
            lines.push(
                "- gov.br: deslogado — aguardando código OTP (peça ao cliente o código do app gov.br e chame `auth_govbr_otp`)"
                    .into(),
            );
        } else if govbr_has_password {
            // Senha salva, sem sessão e sem OTP pendente: a sessão
            // expirou mas o background consegue revalidar sozinho com a
            // senha no próximo ciclo.
            lines.push(
                "- gov.br: sessão expirada (senha já salva; será revalidada automaticamente)"
                    .into(),
            );
        } else {
            lines.push("- gov.br: não autenticado".into());
        }
    }
    if let Some(cnpj) = cnpj {
        lines.push(format!("- CNPJ: {cnpj}"));
    }

    // Situação MEI (mantida fresca pelo worker `jobs::mei_refresh`).
    if has_mei_ccmei_pdf {
        lines.push("- MEI: já tem MEI ativo".into());
        // O CCMEI fica disponível como MCP resource indexado por CNPJ.
        // Aviso o caller sobre a URI exata pra baixar via `resources/read`.
        if let Some(cnpj) = cnpj {
            lines.push(format!(
                "- CCMEI disponível em `{}` (use `resources/read`)",
                ccmei::uri(cnpj)
            ));
        } else {
            lines.push("- CCMEI disponível (resource MCP)".into());
        }
    } else if mei_pode_abrir == Some(false) {
        let motivo = mei_impedimento_motivo.unwrap_or("(motivo não informado)");
        lines.push(format!(
            "- MEI: **impedido de abrir MEI** — motivo: {motivo}"
        ));
    } else if mei_pode_abrir == Some(true) {
        lines.push("- MEI: sem MEI ativo, mas elegível a abrir um".into());
    } else if mei_consultado_em.is_some() {
        lines.push(
            "- MEI: sem MEI ativo; elegibilidade ainda não verificada (precisa de login gov.br)"
                .into(),
        );
    }
    if let Some(em) = mei_consultado_em {
        lines.push(format!("- Situação MEI verificada em: {em}"));
    }

    if let Some(quer_abrir_mei) = quer_abrir_mei {
        lines.push(format!(
            "- Quer abrir MEI novo: {}",
            if quer_abrir_mei { "sim" } else { "não" }
        ));
    }
    if let Some(em) = pagamento_solicitado_em {
        lines.push(format!("- Pagamento solicitado em: {em}"));
    }
    if let (Some(motivo), Some(em)) = (recusa_motivo, recusado_em) {
        // "Recusado" = a Zain decidiu NÃO atender esse lead pelo
        // motivo registrado. Tratar como caso encerrado.
        lines.push(format!(
            "- **Recusado** em {em} (a Zain NÃO vai atender esse lead). Motivo: {motivo}"
        ));
    }
    if lines.is_empty() {
        "(nenhum dado coletado ainda)".into()
    } else {
        lines.join("\n")
    }
}
