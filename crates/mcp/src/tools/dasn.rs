//! DASN-SIMEI (declaração anual do MEI): consolidação em background +
//! resumo cruzando com a vigência do MEI.
//!
//! - [`refresh_dasn_status`]: lê o status por ano no portal (via
//!   `rpa::dasn`) e faz upsert em `zain.dasn_anual`. Chamado pelo worker
//!   `jobs::dasn_refresh` (cadência bem longa — a DASN muda ~1x/ano).
//! - [`run_consultar`] (tool `consultar_dasn`): reconsulta ao vivo e
//!   devolve o resumo (anos em atraso / a declarar / entregues).
//! - [`resumo_lines`]: monta o bloco de texto do `get_client_state`.
//!
//! O atraso NÃO sai pronto do portal: a tabela do portal lista uma janela
//! fixa de anos, então `Original` ≠ atraso (pode ser ano anterior à
//! vigência do MEI). Aqui cruzamos com os `periodos_mei` do certificado
//! salvo pra só marcar atraso em ano realmente devido.

use chrono::{Datelike, NaiveDate};
use deadpool_postgres::Pool;
use pgsafe::sql;
use rmcp::model::CallToolResult;
use rpa::mei::CertificadoMei;
use schemars::JsonSchema;
use serde::Deserialize;
use uuid::Uuid;

use crate::errlog::{self, ErrChain};
use crate::state::AppState;

// ── Consolidação (worker) ──────────────────────────────────────────────

/// Reconsulta o status da DASN no portal e faz upsert em `zain.dasn_anual`.
/// Sucesso carimba `dasn_consultado_em`, zera o backoff e agenda a próxima
/// consulta bem no futuro (a DASN muda raríssimo — 1x/ano). Falha aplica
/// backoff exponencial e propaga o erro.
pub(crate) async fn refresh_dasn_status(
    state: &AppState,
    client_id: Uuid,
    cnpj: &str,
) -> anyhow::Result<()> {
    let anos = match rpa::dasn::consultar_dasn(cnpj).await {
        Ok(a) => a,
        Err(e) => {
            bump_dasn_backoff(&state.pool, client_id).await;
            return Err(e.context("consultar_dasn"));
        }
    };

    for ano in &anos {
        let ano_num = ano.ano;
        let entregue = ano.entregue;
        let tipo = &ano.tipo;
        let situacao_especial = ano.situacao_especial.as_deref();
        let situacao_especial_evento = ano.situacao_especial_evento.as_deref();
        sql!(
            &state.pool,
            "INSERT INTO zain.dasn_anual
                (client_id, ano, entregue, tipo, situacao_especial, situacao_especial_evento, consultado_em)
             VALUES ($client_id, $ano_num, $entregue, $tipo, $situacao_especial, $situacao_especial_evento, now())
             ON CONFLICT (client_id, ano) DO UPDATE
             SET entregue                 = EXCLUDED.entregue,
                 tipo                     = EXCLUDED.tipo,
                 situacao_especial        = EXCLUDED.situacao_especial,
                 situacao_especial_evento = EXCLUDED.situacao_especial_evento,
                 consultado_em            = now()"
        )
        .execute()
        .await?;
    }
    tracing::info!(%client_id, n = anos.len(), "dasn_refresh: status DASN atualizado");

    // Cadência do dasn_refresh (ver "Cadência das crons" no FLUXOS.md):
    // base 30 dias (a DASN muda ~1x/ano; o que muda fora disso o cliente
    // declarando dispara via `consultar_dasn` na hora), multiplicado pelo
    // fator de atividade (1/2/4/6) → 30d ativo … 180d inativo.
    sql!(
        &state.pool,
        "UPDATE zain.clients
         SET dasn_consultado_em        = now(),
             dasn_refresh_falhas       = 0,
             dasn_proxima_tentativa_em = now() + (30 * (CASE
                 WHEN last_activity_at IS NULL                      THEN 6
                 WHEN last_activity_at > now() - interval '7 days'  THEN 1
                 WHEN last_activity_at > now() - interval '30 days' THEN 2
                 WHEN last_activity_at > now() - interval '90 days' THEN 4
                 ELSE 6
             END)) * interval '1 day',
             updated_at                = now()
         WHERE id = $client_id"
    )
    .execute()
    .await?;
    Ok(())
}

/// Backoff exponencial do worker `dasn_refresh`: 1h, 2h, … teto 72h.
async fn bump_dasn_backoff(pool: &Pool, client_id: Uuid) {
    let res = sql!(
        pool,
        "UPDATE zain.clients
         SET dasn_refresh_falhas       = dasn_refresh_falhas + 1,
             dasn_proxima_tentativa_em = now() + LEAST(
                 interval '1 hour' * power(2, dasn_refresh_falhas),
                 interval '72 hours'
             ),
             updated_at = now()
         WHERE id = $client_id"
    )
    .execute()
    .await;
    if let Err(e) = res {
        tracing::warn!(%client_id, error = %e.chain_string(), "dasn_refresh: falha ao gravar backoff");
    }
}

// ── Resumo (cruzando vigência) ─────────────────────────────────────────

/// Uma linha consolidada de `zain.dasn_anual`.
pub(crate) struct DasnRow {
    pub ano: i32,
    pub entregue: bool,
}

/// Classificação de um ano da DASN frente à vigência do MEI e ao prazo.
struct Classificacao {
    atrasados: Vec<i32>,
    a_declarar: Vec<i32>,
    entregues_no_periodo: usize,
    primeiro_ano_mei: Option<i32>,
}

/// O prazo da DASN do ano-calendário `ano` é 31/05 do ano seguinte.
fn prazo_vencido(ano: i32, hoje: NaiveDate) -> bool {
    match NaiveDate::from_ymd_opt(ano + 1, 5, 31) {
        Some(prazo) => hoje > prazo,
        None => false,
    }
}

/// Anos em que o CPF/CNPJ esteve enquadrado como MEI (qualquer parte do
/// ano conta), derivados dos períodos do certificado. Sem certificado,
/// devolve `None` — aí não dá pra afirmar atraso com segurança.
fn anos_de_vigencia(cert: Option<&CertificadoMei>, hoje: NaiveDate) -> Option<Vec<i32>> {
    let cert = cert?;
    let ano_de = |iso: &str| iso.get(..4).and_then(|s| s.parse::<i32>().ok());
    let mut anos = Vec::new();
    for p in &cert.periodos_mei {
        let Some(ini) = ano_de(&p.inicio) else {
            continue;
        };
        let fim = p.fim.as_deref().and_then(ano_de).unwrap_or(hoje.year());
        for y in ini..=fim {
            if !anos.contains(&y) {
                anos.push(y);
            }
        }
    }
    Some(anos)
}

fn classificar(rows: &[DasnRow], cert: Option<&CertificadoMei>, hoje: NaiveDate) -> Classificacao {
    let vigencia = anos_de_vigencia(cert, hoje);
    let primeiro_ano_mei = vigencia.as_ref().and_then(|v| v.iter().min().copied());
    let devido = |ano: i32| vigencia.as_ref().is_some_and(|v| v.contains(&ano));

    let mut atrasados = Vec::new();
    let mut a_declarar = Vec::new();
    let mut entregues_no_periodo = 0;
    for r in rows {
        if !devido(r.ano) {
            continue;
        }
        if r.entregue {
            entregues_no_periodo += 1;
        } else if prazo_vencido(r.ano, hoje) {
            atrasados.push(r.ano);
        } else {
            a_declarar.push(r.ano);
        }
    }
    atrasados.sort_unstable();
    a_declarar.sort_unstable();
    Classificacao {
        atrasados,
        a_declarar,
        entregues_no_periodo,
        primeiro_ano_mei,
    }
}

/// Bloco de texto da DASN pro `get_client_state`. Vazio se não houver nada
/// consolidado (sem linhas = ainda não consultado, dito explicitamente).
pub(crate) fn resumo_lines(
    rows: &[DasnRow],
    cert: Option<&CertificadoMei>,
    consultado_em: Option<&chrono::DateTime<chrono::Utc>>,
    hoje: NaiveDate,
) -> Vec<String> {
    if rows.is_empty() {
        return vec![
            "- DASN (declaração anual): situação ainda não consultada — será verificada automaticamente em background. NÃO é sinal de problema.".into(),
        ];
    }

    let c = classificar(rows, cert, hoje);
    let mut lines = Vec::new();

    if !c.atrasados.is_empty() {
        lines.push(format!(
            "- DASN **em atraso**: {} — a declaração anual do MEI vence 31/05 do ano seguinte e esses anos passaram sem entrega. Oriente o cliente a regularizar (declaração anual de faturamento; entrega atrasada gera multa mínima de R$ 50). A Zain ainda NÃO transmite por ele.",
            lista_anos(&c.atrasados)
        ));
    }
    if !c.a_declarar.is_empty() {
        lines.push(format!(
            "- DASN a declarar (dentro do prazo): {} (vence 31/05 do ano seguinte)",
            lista_anos(&c.a_declarar)
        ));
    }
    if c.atrasados.is_empty() && c.a_declarar.is_empty() {
        if c.entregues_no_periodo > 0 {
            lines.push("- DASN (declaração anual): em dia".into());
        } else if let Some(ano1) = c.primeiro_ano_mei {
            // MEI recém-aberto: nenhuma DASN devida ainda. A 1ª será a do
            // ano de abertura, no prazo do ano seguinte.
            lines.push(format!(
                "- DASN (declaração anual): sem pendência — a 1ª declaração será a de {ano1} (vence 31/05/{})",
                ano1 + 1
            ));
        } else {
            lines.push("- DASN (declaração anual): sem pendência identificada".into());
        }
    }
    if let Some(em) = consultado_em {
        lines.push(format!(
            "- Situação DASN consultada em: {}",
            em.to_rfc3339()
        ));
    }
    lines
}

fn lista_anos(anos: &[i32]) -> String {
    anos.iter()
        .map(i32::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

// ── Tool `consultar_dasn` ──────────────────────────────────────────────

#[derive(Deserialize, JsonSchema)]
pub struct ConsultarArgs {}

pub async fn run_consultar(
    state: &AppState,
    client_id: Uuid,
    _args: ConsultarArgs,
) -> CallToolResult {
    let (cnpj, cert) = match load_cnpj_cert(&state.pool, client_id).await {
        Ok(Some(c)) => c,
        Ok(None) => {
            return CallToolResult::structured_error(serde_json::json!({
                "status": "erro",
                "motivo": "sem_cnpj",
                "mensagem": "Este lead não tem CNPJ salvo — a DASN só existe pra quem já tem MEI.",
            }));
        }
        Err(e) => {
            tracing::warn!(%client_id, error = %errlog::anyhow_chain(&e), "consultar_dasn: falha ao ler CNPJ");
            return CallToolResult::structured_error(serde_json::json!({
                "status": "erro",
                "mensagem": "Não consegui ler o cadastro agora. Tente de novo em instantes.",
            }));
        }
    };

    tracing::info!(%client_id, "consultar_dasn: reconsultando status no portal");
    if let Err(e) = refresh_dasn_status(state, client_id, &cnpj).await {
        tracing::warn!(%client_id, error = %errlog::anyhow_chain(&e), "consultar_dasn: falha ao reconsultar");
        return CallToolResult::structured_error(serde_json::json!({
            "status": "erro",
            "motivo": "portal_instavel",
            "mensagem": "Não consegui consultar a DASN agora — o portal do Simples Nacional pode estar instável. NÃO é problema do cliente: agende uma retentativa com `schedule_retentar`.",
        }));
    }

    let rows = match load_dasn_rows(&state.pool, client_id).await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(%client_id, error = %errlog::anyhow_chain(&e), "consultar_dasn: falha ao ler resumo");
            return CallToolResult::structured_error(serde_json::json!({
                "status": "erro",
                "mensagem": "Consultei o portal, mas não consegui ler o resultado do banco agora. Tente de novo em instantes.",
            }));
        }
    };
    let hoje = chrono::Utc::now().date_naive();
    let c = classificar(&rows, cert.as_ref(), hoje);
    CallToolResult::structured(serde_json::json!({
        "status": "ok",
        "em_atraso": c.atrasados,
        "a_declarar": c.a_declarar,
        "anos_entregues": rows.iter().filter(|r| r.entregue).map(|r| r.ano).collect::<Vec<_>>(),
        "mensagem": if !c.atrasados.is_empty() {
            "Há DASN em atraso. Oriente o cliente a regularizar (a Zain ainda não transmite a declaração por ele)."
        } else {
            "Nenhuma DASN em atraso dentro da vigência do MEI."
        },
    }))
}

// ── DB helpers ─────────────────────────────────────────────────────────

pub(crate) async fn load_dasn_rows(pool: &Pool, client_id: Uuid) -> anyhow::Result<Vec<DasnRow>> {
    let rows = sql!(
        pool,
        "SELECT ano, entregue FROM zain.dasn_anual WHERE client_id = $client_id ORDER BY ano DESC"
    )
    .fetch_all()
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| DasnRow {
            ano: r.ano,
            entregue: r.entregue,
        })
        .collect())
}

async fn load_cnpj_cert(
    pool: &Pool,
    client_id: Uuid,
) -> anyhow::Result<Option<(String, Option<CertificadoMei>)>> {
    let row = sql!(
        pool,
        "SELECT cnpj, mei_ccmei FROM zain.clients WHERE id = $client_id"
    )
    .fetch_optional()
    .await?;
    Ok(row.and_then(|r| r.cnpj.map(|c| (c, r.mei_ccmei))))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    fn rows(pairs: &[(i32, bool)]) -> Vec<DasnRow> {
        pairs
            .iter()
            .map(|(ano, entregue)| DasnRow {
                ano: *ano,
                entregue: *entregue,
            })
            .collect()
    }

    fn cert_com_periodo(inicio: &str, fim: Option<&str>) -> CertificadoMei {
        CertificadoMei {
            nome_empresario: String::new(),
            cpf: String::new(),
            cnpj: String::new(),
            data_abertura: String::new(),
            nome_empresarial: String::new(),
            capital_social: String::new(),
            situacao_cadastral: String::new(),
            data_situacao_cadastral: String::new(),
            endereco_comercial: rpa::mei::EnderecoCertificado {
                cep: String::new(),
                logradouro: String::new(),
                numero: String::new(),
                complemento: None,
                bairro: String::new(),
                municipio: String::new(),
                uf: String::new(),
            },
            situacao_atual: String::new(),
            periodos_mei: vec![rpa::mei::PeriodoMei {
                periodo: "1° Período".into(),
                inicio: inicio.into(),
                fim: fim.map(str::to_string),
            }],
            forma_atuacao: String::new(),
            ocupacao_principal: String::new(),
            atividade_principal: String::new(),
            pdf: Vec::new(),
        }
    }

    #[test]
    fn prazo() {
        // DASN/2024 vence 31/05/2025.
        assert!(prazo_vencido(2024, d("2025-06-01")));
        assert!(!prazo_vencido(2024, d("2025-05-31")));
        assert!(!prazo_vencido(2025, d("2026-05-30")));
    }

    #[test]
    fn atraso_so_dentro_da_vigencia() {
        // MEI desde 2024; 2025 não entregue e prazo vencido → atraso.
        // 2021–2023 não entregues mas FORA da vigência → ignorados.
        let cert = cert_com_periodo("2024-03-01", None);
        let rows = rows(&[
            (2025, false),
            (2024, true),
            (2023, false),
            (2022, false),
            (2021, false),
        ]);
        let c = classificar(&rows, Some(&cert), d("2026-06-12"));
        assert_eq!(c.atrasados, vec![2025]);
        assert!(c.a_declarar.is_empty());
        assert_eq!(c.entregues_no_periodo, 1);
    }

    #[test]
    fn mei_novo_sem_pendencia() {
        // MEI desde 2026; nenhum dos anos listados é devido → sem atraso.
        let cert = cert_com_periodo("2026-04-01", None);
        let rows = rows(&[(2025, false), (2024, false), (2021, false)]);
        let c = classificar(&rows, Some(&cert), d("2026-06-12"));
        assert!(c.atrasados.is_empty());
        assert!(c.a_declarar.is_empty());
        assert_eq!(c.primeiro_ano_mei, Some(2026));
    }
}
