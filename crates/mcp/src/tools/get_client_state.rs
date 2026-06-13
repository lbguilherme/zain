//! Tool `get_client_state`: devolve o estado atual do cliente em
//! formato de prompt, pronto pra ser injetado no system/user prompt
//! do caller.
//!
//! Espelha o bloco que o agent original construía em
//! `format_dados_coletados` (`crates/agent/src/prompt.rs`): contato,
//! dados coletados, estado gov.br, MEI/CCMEI. Se o cliente tem CCMEI
//! salvo, o texto avisa que o PDF está disponível via tool `get_ccmei`.

use pgsafe::sql;
use rmcp::model::{CallToolResult, Content};
use rpa::govbr::Nivel;
use schemars::JsonSchema;
use serde::Deserialize;
use uuid::Uuid;

use super::{das, dasn};
use crate::errlog::ErrChain;
use crate::state::AppState;

#[derive(Deserialize, JsonSchema)]
pub struct Args {}

pub async fn run(state: &AppState, client_id: Uuid, _args: Args) -> CallToolResult {
    // Carimba a atividade do cliente: o `get_client_state` roda todo turno
    // em que o cliente interage, então é o melhor sinal de "cliente ativo".
    // Os workers usam isso pra espaçar a cadência de inativos. Best-effort:
    // falha aqui não pode atrapalhar a leitura do estado.
    if let Err(e) = sql!(
        &state.pool,
        "UPDATE zain.clients SET last_activity_at = now() WHERE id = $client_id"
    )
    .execute()
    .await
    {
        tracing::warn!(%client_id, error = %e.chain_string(), "get_client_state: falha ao carimbar last_activity_at");
    }

    let row = match sql!(
        &state.pool,
        "SELECT
            phone,
            name,
            cpf,
            cnpj,
            quer_abrir_mei,
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
            mei_consultado_em,
            das_consultado_em,
            dasn_consultado_em,
            mei_ccmei
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
            tracing::warn!(%client_id, error = %e.chain_string(), "get_client_state: falha ao ler cliente");
            return CallToolResult::structured_error(serde_json::json!({
                "status": "erro",
                "mensagem": "Não consegui ler o cadastro do cliente agora. Tente de novo em instantes.",
            }));
        }
    };

    // Situação DAS consolidada pelo worker `jobs::das_refresh`. Só faz
    // sentido pra quem tem CNPJ; sem linhas = ainda não consultado.
    let das_lines = if row.cnpj.is_some() {
        match load_das_lines(state, client_id, row.das_consultado_em.as_ref()).await {
            Ok(lines) => lines,
            Err(e) => {
                tracing::warn!(%client_id, error = %e.chain_string(), "get_client_state: falha ao ler situação DAS");
                Vec::new()
            }
        }
    } else {
        Vec::new()
    };

    // Situação DASN (declaração anual) consolidada pelo `jobs::dasn_refresh`,
    // cruzada com a vigência do MEI (do certificado) pra só marcar atraso em
    // ano realmente devido.
    let dasn_lines = if row.cnpj.is_some() {
        match dasn::load_dasn_rows(&state.pool, client_id).await {
            Ok(rows) => dasn::resumo_lines(
                &rows,
                row.mei_ccmei.as_ref(),
                row.dasn_consultado_em.as_ref(),
                chrono::Utc::now().date_naive(),
            ),
            Err(e) => {
                tracing::warn!(%client_id, error = %e.chain_string(), "get_client_state: falha ao ler situação DASN");
                Vec::new()
            }
        }
    } else {
        Vec::new()
    };

    let contact_name = row.name.as_deref().unwrap_or("(desconhecido)");
    let contact_phone = row.phone.as_deref().unwrap_or("(desconhecido)");
    let dados_coletados = format_dados_coletados(
        row.cpf.as_deref(),
        row.cnpj.as_deref(),
        row.quer_abrir_mei,
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

    let mut text = format!(
        "Informações do contato:\n\
         - Nome no WhatsApp: {contact_name}\n\
         - Telefone: {contact_phone}\n\
         \n\
         Dados coletados até agora:\n\
         {dados_coletados}"
    );
    if !das_lines.is_empty() {
        text.push('\n');
        text.push_str(&das_lines.join("\n"));
    }
    if !dasn_lines.is_empty() {
        text.push('\n');
        text.push_str(&dasn_lines.join("\n"));
    }

    CallToolResult::success(vec![Content::text(text)])
}

/// Resumo da situação DAS (consolidada em `zain.das_mensal` pelo worker
/// `jobs::das_refresh`): meses em atraso, próximo vencimento e quando a
/// consulta foi feita. Vazio no banco = ainda não consultado — e isso é
/// dito explicitamente, pra o agente não interpretar silêncio como
/// problema (nem recusar lead por isso).
async fn load_das_lines(
    state: &AppState,
    client_id: Uuid,
    das_consultado_em: Option<&chrono::DateTime<chrono::Utc>>,
) -> anyhow::Result<Vec<String>> {
    let rows = sql!(
        &state.pool,
        "SELECT competencia, situacao, total_cents, vencimento,
                (parcelado_em IS NOT NULL AND parcelado_em > now() - interval '30 days') AS parcelado
         FROM zain.das_mensal
         WHERE client_id = $client_id
         ORDER BY periodo"
    )
    .fetch_all()
    .await?;

    if rows.is_empty() {
        return Ok(vec![
            "- DAS (mensalidade do MEI): situação ainda não consultada — será verificada automaticamente em background. NÃO é sinal de problema nem motivo de recusa.".into(),
        ]);
    }

    let descrever = |r: &_DasRow| -> String {
        let mut s = r.competencia.clone();
        if let Some(total) = r.total_cents {
            s.push_str(&format!(" ({}", das::fmt_cents(total)));
            if let Some(v) = r.vencimento {
                s.push_str(&format!(", vence {}", v.format("%d/%m/%Y")));
            }
            s.push(')');
        } else if let Some(v) = r.vencimento {
            s.push_str(&format!(" (vence {})", v.format("%d/%m/%Y")));
        }
        s
    };
    let das_rows: Vec<_DasRow> = rows
        .into_iter()
        .map(|r| _DasRow {
            competencia: r.competencia,
            situacao: r.situacao,
            total_cents: r.total_cents,
            vencimento: r.vencimento,
            parcelado: r.parcelado.unwrap_or(false),
        })
        .collect();

    let mut lines = Vec::new();

    // Meses confirmados em PARCELAMENTO (detectado ao emitir) — pagam pela
    // parcela, não por guia. Saem das listas de atraso/em-aberto.
    let parcelados: Vec<String> = das_rows
        .iter()
        .filter(|r| r.parcelado)
        .map(|r| r.competencia.clone())
        .collect();
    if !parcelados.is_empty() {
        lines.push(format!(
            "- DAS em PARCELAMENTO ({}): {} — pague a parcela pelo app de parcelamento do MEI/Simples Nacional, NÃO por guia normal. Não ofereça `emitir_das` pra esses meses.",
            parcelados.len(),
            parcelados.join("; ")
        ));
    }

    // `devedor` = ano corrente, atraso confirmado (paga a guia normal).
    let devedores: Vec<String> = das_rows
        .iter()
        .filter(|r| r.situacao == "devedor" && !r.parcelado)
        .map(&descrever)
        .collect();
    if devedores.is_empty() {
        lines.push("- DAS (mensalidade do MEI): nenhum mês em atraso no ano corrente".into());
    } else {
        lines.push(format!(
            "- DAS em atraso ({}): {} — ofereça a guia atualizada com a tool `emitir_das` (multa/juros já recalculados)",
            devedores.len(),
            devedores.join("; ")
        ));
    }

    // `em_aberto` = anos anteriores com valor a regularizar (excluindo os já
    // confirmados parcelados). NÃO afirme "atraso/devedor" — os que ainda
    // não tentamos emitir podem estar parcelados. Sumariza.
    let abertos: Vec<&_DasRow> = das_rows
        .iter()
        .filter(|r| r.situacao == "em_aberto" && !r.parcelado)
        .collect();
    if !abertos.is_empty() {
        let total: i64 = abertos.iter().filter_map(|r| r.total_cents).sum();
        let mut anos: Vec<&str> = abertos
            .iter()
            .filter_map(|r| r.competencia.rsplit('/').next())
            .collect();
        anos.sort_unstable();
        anos.dedup();
        let faixa = match (anos.first(), anos.last()) {
            (Some(a), Some(b)) if a != b => format!("{a}–{b}"),
            (Some(a), _) => a.to_string(),
            _ => String::new(),
        };
        lines.push(format!(
            "- DAS em aberto de anos anteriores: {} mês(es) somando ~{} ({faixa}). Pode incluir meses já em PARCELAMENTO — não dá pra saber pela consulta, só ao emitir. Pra regularizar, chame `emitir_das` com o `periodo` (YYYYMM) do mês: ele devolve a guia OU avisa se está parcelado (aí o pagamento é pelo app de parcelamento). NÃO afirme que é atraso simples.",
            abertos.len(),
            das::fmt_cents(total)
        ));
    }

    if let Some(prox) = das_rows.iter().find(|r| r.situacao == "a_vencer") {
        lines.push(format!("- Próximo DAS: {}", descrever(prox)));
    }
    if let Some(em) = das_consultado_em {
        lines.push(format!("- Situação DAS consultada em: {}", em.to_rfc3339()));
    }
    Ok(lines)
}

struct _DasRow {
    competencia: String,
    situacao: String,
    total_cents: Option<i64>,
    vencimento: Option<chrono::NaiveDate>,
    parcelado: bool,
}

#[allow(clippy::too_many_arguments)]
fn format_dados_coletados(
    cpf: Option<&str>,
    cnpj: Option<&str>,
    quer_abrir_mei: Option<bool>,
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
        lines.push("- CCMEI disponível (PDF — chame a tool `get_ccmei` pra receber inline)".into());
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
    } else if govbr_autenticado {
        // Logado mas a consulta MEI ainda não concluiu (tipicamente SIMEI
        // instável). Sem esta linha o estado fica mudo sobre MEI e o agente
        // já interpretou o silêncio como impedimento, recusando lead bom.
        lines.push(
            "- MEI: situação ainda NÃO verificada — a consulta ao SIMEI não concluiu (sistema do governo possivelmente instável); será retentada automaticamente. Isso NÃO é impedimento nem motivo de recusa."
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
