//! DAS (a mensalidade do MEI): consolidação em background + emissão de
//! guia on-demand.
//!
//! - [`refresh_das_status`]: consulta o PGMEI (via `rpa::pgmei`) e faz
//!   upsert da situação mês a mês em `zain.das_mensal`, pra que o
//!   `get_client_state` reporte atraso/próximo vencimento como leitura
//!   SQL pura. Chamado pelo worker `jobs::das_refresh`.
//! - [`run_emitir`] (tool `emitir_das`): gera a guia de uma competência
//!   na hora e devolve o PDF (código de barras + QR PIX) como resource
//!   inline. Sempre on-demand: guia de mês em atraso é recalculada por
//!   dia (multa/juros), então nunca servimos PDF velho.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use chrono::{Datelike, NaiveDate};
use deadpool_postgres::Pool;
use pgsafe::sql;
use rmcp::model::{CallToolResult, Content, ResourceContents};
use schemars::JsonSchema;
use serde::Deserialize;
use uuid::Uuid;

use crate::errlog::{self, ErrChain};
use crate::state::AppState;

// ── Consolidação (worker) ──────────────────────────────────────────────

/// Atualiza a situação DAS do cliente no banco, mantendo o histórico
/// DENSO. Varre do ano corrente até ~5 anos atrás (limite da prescrição /
/// da janela do PGMEI), mas **pula anos já congelados** — anos passados
/// 100% liquidados (ou só não-optante) nunca mudam, então não vale
/// reconsultar. O corrente sempre entra; um ano passado entra se tiver
/// algum mês devedor/a_vencer ou ainda estiver incompleto no banco. Tudo
/// numa única sessão de browser (identifica uma vez).
///
/// Sucesso carimba `das_consultado_em`, zera o backoff e agenda a próxima
/// consulta para o **menor vencimento futuro** entre os meses `a_vencer`
/// (+margem) em vez de um TTL fixo. Falha de sessão (PGMEI instável /
/// captcha) aplica backoff exponencial e propaga o erro pro worker logar.
pub(crate) async fn refresh_das_status(
    state: &AppState,
    client_id: Uuid,
    cnpj: &str,
) -> anyhow::Result<()> {
    let hoje = chrono::Utc::now().date_naive();
    let ano_atual = hoje.year();
    // Piso: não antes da abertura do MEI nem além de 5 anos (prescrição /
    // janela do portal). Sem certificado, usa só o piso de prescrição.
    let piso_prescricao = ano_atual - 5;
    let piso = match load_mei_inicio_ano(&state.pool, client_id).await {
        Ok(Some(m)) => m.max(piso_prescricao),
        _ => piso_prescricao,
    };

    // Quais anos varrer: o corrente sempre; passados só se NÃO congelados.
    let mut anos_scan = Vec::new();
    for ano in piso..=ano_atual {
        let varrer = ano == ano_atual
            || !ano_congelado(&state.pool, client_id, ano)
                .await
                .unwrap_or(false);
        if varrer {
            anos_scan.push(ano);
        }
    }

    let resultados = match rpa::pgmei::consultar_das_anos(cnpj, &anos_scan).await {
        Ok(r) => r,
        Err(e) => {
            bump_das_backoff(&state.pool, client_id).await;
            return Err(e.context("consultar_das_anos"));
        }
    };
    if resultados.is_empty() {
        // Nem o ano corrente voltou — trata como instabilidade da sessão.
        bump_das_backoff(&state.pool, client_id).await;
        anyhow::bail!("consultar_das_anos não devolveu nenhum ano (anos pedidos: {anos_scan:?})");
    }

    for (ano, meses) in &resultados {
        if let Err(e) = upsert_meses(&state.pool, client_id, meses).await {
            // Erro de banco não é culpa do portal: não mexe no backoff,
            // só propaga (o worker loga e tenta no próximo ciclo).
            return Err(e.context("persistir situação DAS"));
        }
        tracing::info!(%client_id, ano, n = meses.len(), "das_refresh: situação DAS atualizada");
    }

    // Cadência do das_refresh (ver "Cadência das crons" no FLUXOS.md):
    // ÂNCORA = menor vencimento futuro entre os `a_vencer` + 3 dias de margem
    // (compensação bancária D+1/D+2 + fim de semana). Sem vencimento futuro,
    // TTL de segurança de 24h.
    // ATIVIDADE: cliente inativo NÃO precisa de recheck no passo do
    // vencimento — esticamos a âncora em +14 dias por nível de inatividade
    // (ativo +0; morno +14; esfriando +42; inativo +70). O fator de
    // atividade (1/2/4/6) menos 1 dá esses "passos".
    sql!(
        &state.pool,
        "UPDATE zain.clients
         SET das_consultado_em        = now(),
             das_refresh_falhas       = 0,
             das_proxima_tentativa_em = COALESCE(
                 ((SELECT MIN(vencimento)
                   FROM zain.das_mensal
                   WHERE client_id = $client_id
                     AND situacao  = 'a_vencer'
                     AND vencimento >= now()::date) + interval '3 days')::timestamptz,
                 now() + interval '24 hours'
             ) + ((CASE
                 WHEN last_activity_at IS NULL                      THEN 6
                 WHEN last_activity_at > now() - interval '7 days'  THEN 1
                 WHEN last_activity_at > now() - interval '30 days' THEN 2
                 WHEN last_activity_at > now() - interval '90 days' THEN 4
                 ELSE 6
             END) - 1) * interval '14 days',
             updated_at               = now()
         WHERE id = $client_id"
    )
    .execute()
    .await?;
    Ok(())
}

async fn upsert_meses(
    pool: &Pool,
    client_id: Uuid,
    meses: &[rpa::pgmei::DasMensal],
) -> anyhow::Result<()> {
    for mes in meses {
        let periodo = &mes.periodo;
        let competencia = &mes.competencia;
        let apurado = mes.apurado;
        let situacao = mes.situacao.as_str();
        let situacao_texto = &mes.situacao_texto;
        let principal_cents = mes.principal_cents;
        let multa_cents = mes.multa_cents;
        let juros_cents = mes.juros_cents;
        let total_cents = mes.total_cents;
        let vencimento = mes
            .vencimento
            .as_deref()
            .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok());
        let acolhimento = mes
            .acolhimento
            .as_deref()
            .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok());
        sql!(
            pool,
            "INSERT INTO zain.das_mensal
                (client_id, periodo, competencia, apurado, situacao, situacao_texto,
                 principal_cents, multa_cents, juros_cents, total_cents,
                 vencimento, acolhimento, consultado_em)
             VALUES
                ($client_id, $periodo, $competencia, $apurado, $situacao, $situacao_texto,
                 $principal_cents, $multa_cents, $juros_cents, $total_cents,
                 $vencimento, $acolhimento, now())
             ON CONFLICT (client_id, periodo) DO UPDATE
             SET competencia     = EXCLUDED.competencia,
                 apurado         = EXCLUDED.apurado,
                 situacao        = EXCLUDED.situacao,
                 situacao_texto  = EXCLUDED.situacao_texto,
                 principal_cents = EXCLUDED.principal_cents,
                 multa_cents     = EXCLUDED.multa_cents,
                 juros_cents     = EXCLUDED.juros_cents,
                 total_cents     = EXCLUDED.total_cents,
                 vencimento      = EXCLUDED.vencimento,
                 acolhimento     = EXCLUDED.acolhimento,
                 consultado_em   = now()"
        )
        .execute()
        .await?;
    }
    Ok(())
}

/// Ano de início da vigência do MEI (menor `inicio` dos períodos do
/// certificado salvo). `None` se não há certificado — aí o caller usa só
/// o piso de prescrição.
async fn load_mei_inicio_ano(pool: &Pool, client_id: Uuid) -> anyhow::Result<Option<i32>> {
    let row = sql!(
        pool,
        "SELECT mei_ccmei FROM zain.clients WHERE id = $client_id"
    )
    .fetch_optional()
    .await?;
    let Some(Some(cert)) = row.map(|r| r.mei_ccmei) else {
        return Ok(None);
    };
    let menor = cert
        .periodos_mei
        .iter()
        .filter_map(|p| p.inicio.get(..4).and_then(|s| s.parse::<i32>().ok()))
        .min();
    Ok(menor)
}

/// Um ano passado está "congelado" quando já temos os 12 meses salvos e
/// nenhum está aberto (`devedor`/`a_vencer`) — só liquidado/não-optante,
/// que nunca mais mudam. Esses não precisam ser reconsultados.
async fn ano_congelado(pool: &Pool, client_id: Uuid, ano: i32) -> anyhow::Result<bool> {
    let padrao = format!("{ano}%");
    let row = sql!(
        pool,
        "SELECT
            count(*)                                                                   AS n,
            count(*) FILTER (WHERE situacao IN ('devedor', 'em_aberto', 'a_vencer'))    AS abertos
         FROM zain.das_mensal
         WHERE client_id = $client_id AND periodo LIKE $padrao"
    )
    .fetch_one()
    .await?;
    Ok(row.n >= 12 && row.abertos == 0)
}

/// Backoff exponencial do worker `das_refresh`: 1h, 2h, 4h, … teto 72h.
/// Mesmo padrão do `mei_refresh` (ver `bump_refresh_backoff` em
/// `tools::govbr`): só afeta a seleção do worker; o fluxo interativo
/// (`emitir_das`) nunca consulta essas colunas. Best-effort.
async fn bump_das_backoff(pool: &Pool, client_id: Uuid) {
    let res = sql!(
        pool,
        "UPDATE zain.clients
         SET das_refresh_falhas       = das_refresh_falhas + 1,
             das_proxima_tentativa_em = now() + LEAST(
                 interval '1 hour' * power(2, das_refresh_falhas),
                 interval '72 hours'
             ),
             updated_at = now()
         WHERE id = $client_id"
    )
    .execute()
    .await;
    if let Err(e) = res {
        tracing::warn!(%client_id, error = %e.chain_string(), "das_refresh: falha ao gravar backoff");
    }
}

// ── Tool `emitir_das` ──────────────────────────────────────────────────

#[derive(Deserialize, JsonSchema)]
pub struct EmitirArgs {
    /// Competência no formato YYYYMM (ex: '202604' = abril/2026).
    /// Omita pra emitir o mês mais antigo em atraso (ou, sem atraso, o
    /// próximo a vencer).
    pub periodo: Option<String>,
}

pub async fn run_emitir(state: &AppState, client_id: Uuid, args: EmitirArgs) -> CallToolResult {
    let cnpj = match load_cnpj(&state.pool, client_id).await {
        Ok(Some(c)) => c,
        Ok(None) => {
            return CallToolResult::structured_error(serde_json::json!({
                "status": "erro",
                "motivo": "sem_cnpj",
                "mensagem": "Este lead não tem CNPJ salvo — o DAS só existe pra quem já tem MEI.",
            }));
        }
        Err(e) => {
            tracing::warn!(%client_id, error = %errlog::anyhow_chain(&e), "emitir_das: falha ao ler CNPJ");
            return CallToolResult::structured_error(serde_json::json!({
                "status": "erro",
                "mensagem": "Não consegui ler o cadastro agora. Tente de novo em instantes.",
            }));
        }
    };

    let periodo = match resolver_periodo(&state.pool, client_id, args.periodo.as_deref()).await {
        Ok(p) => p,
        Err(resp) => return resp,
    };

    // Cache same-day: o valor de uma competência não muda dentro do mesmo
    // dia, e o PGMEI tem limite diário de gerações por CNPJ. Se já emitimos
    // esta competência hoje, serve o PDF cacheado — guia idêntica, sem
    // gastar uma geração do limite.
    match load_guia_cache(&state.pool, client_id, &periodo).await {
        Ok(Some(g)) => {
            tracing::info!(%client_id, %periodo, "emitir_das: cache same-day — servindo guia já emitida hoje");
            return montar_resposta(&cnpj, &periodo, &g);
        }
        Ok(None) => {}
        Err(e) => {
            // Falha de leitura do cache não impede emitir — só loga e segue.
            tracing::warn!(%client_id, %periodo, error = %errlog::anyhow_chain(&e), "emitir_das: falha ao ler cache (seguindo pra emissão)");
        }
    }

    tracing::info!(%client_id, %periodo, "emitir_das: emitindo guia no PGMEI");
    let guia = match rpa::pgmei::emitir_das(&cnpj, &periodo).await {
        Ok(g) => g,
        Err(e) => {
            let msg = e.to_string();
            tracing::warn!(%client_id, %periodo, error = %errlog::anyhow_chain(&e), "emitir_das: falha");
            // Bails de domínio do rpa::pgmei são acionáveis pelo agente
            // (período inexistente/não emissível); o resto é instabilidade.
            if msg.contains("não está emissível") || msg.contains("não existe na tabela") {
                return CallToolResult::structured_error(serde_json::json!({
                    "status": "erro",
                    "motivo": "periodo_nao_emissivel",
                    "mensagem": msg,
                }));
            }
            // Mês em PARCELAMENTO: o portal até gera uma guia, mas ela NÃO
            // deve ser paga — a dívida foi negociada e a parcela se paga
            // pelo app de parcelamento. Não entregamos essa guia.
            if msg.contains("parcelad") {
                return CallToolResult::structured_error(serde_json::json!({
                    "status": "erro",
                    "motivo": "periodo_parcelado",
                    "mensagem": format!("Este mês ({periodo}) está num PARCELAMENTO — a dívida já foi negociada em parcelas. A guia normal do DAS NÃO serve (pagar por ela não quita a parcela). Explique ao cliente que esse mês se paga pelo aplicativo de parcelamento do MEI/Simples Nacional, não por essa guia. Os meses que NÃO estão parcelados continuam sendo emitidos normalmente."),
                }));
            }
            // Limite diário de geração de DAS por CNPJ (código 23998 do
            // PGMEI). É transitório, mas reseta só no dia seguinte —
            // retentar em minutos NÃO resolve.
            if msg.contains("limite diário") || msg.contains("23998") {
                return CallToolResult::structured_error(serde_json::json!({
                    "status": "erro",
                    "motivo": "limite_diario_excedido",
                    "mensagem": "O portal atingiu o limite diário de emissões de guia DAS pra este CNPJ. NÃO é problema do cliente. O limite reseta amanhã — avise que você gera a guia no próximo dia (agende `schedule_retentar` pra amanhã, não pra daqui a pouco), ou peça pra ele te chamar amanhã.",
                }));
            }
            return CallToolResult::structured_error(serde_json::json!({
                "status": "erro",
                "motivo": "pgmei_instavel",
                "mensagem": "Não consegui emitir a guia agora — o portal do Simples Nacional (PGMEI) pode estar instável. NÃO é impedimento do cliente: agende uma retentativa com `schedule_retentar` e avise que você retoma sozinho.",
            }));
        }
    };

    let cache = GuiaCache {
        competencia: guia.competencia,
        numero_das: guia.numero_das,
        total_cents: guia.total_cents,
        vencimento: guia.vencimento.as_deref().and_then(parse_iso),
        pagar_ate: guia.pagar_ate.as_deref().and_then(parse_iso),
        linha_digitavel: guia.linha_digitavel,
        pdf: guia.pdf,
    };
    tracing::info!(%client_id, %periodo, pdf_bytes = cache.pdf.len(), "emitir_das: guia emitida");
    // Grava no cache (best-effort: falha não derruba a resposta).
    if let Err(e) = store_guia_cache(&state.pool, client_id, &periodo, &cache).await {
        tracing::warn!(%client_id, %periodo, error = %errlog::anyhow_chain(&e), "emitir_das: falha ao gravar cache");
    }
    montar_resposta(&cnpj, &periodo, &cache)
}

/// Dados de uma guia DAS — origem da emissão fresca ou do cache same-day.
struct GuiaCache {
    competencia: String,
    numero_das: String,
    total_cents: Option<i64>,
    vencimento: Option<NaiveDate>,
    pagar_ate: Option<NaiveDate>,
    linha_digitavel: Option<String>,
    pdf: Vec<u8>,
}

fn parse_iso(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}

/// Monta o resultado da tool (texto + PDF inline) a partir de uma guia,
/// venha ela da emissão na hora ou do cache same-day.
fn montar_resposta(cnpj: &str, periodo: &str, g: &GuiaCache) -> CallToolResult {
    let mut texto = format!(
        "Guia DAS de {} emitida (documento {}).",
        g.competencia, g.numero_das
    );
    if let Some(total) = g.total_cents {
        texto.push_str(&format!(" Valor: {}.", fmt_cents(total)));
    }
    if let Some(v) = g.vencimento {
        texto.push_str(&format!(" Vencimento original: {}.", v.format("%d/%m/%Y")));
    }
    if let Some(p) = g.pagar_ate {
        texto.push_str(&format!(" Pagar até: {}.", p.format("%d/%m/%Y")));
    }
    if let Some(l) = &g.linha_digitavel {
        texto.push_str(&format!("\nLinha digitável (código de barras): {l}"));
    }
    texto.push_str(
        "\nO PDF anexado tem o código de barras e o QR code PIX — envie pro cliente escolher como pagar.",
    );

    let uri = format!("zain://mei/{cnpj}/das/{periodo}.pdf");
    let blob = BASE64_STANDARD.encode(&g.pdf);
    let contents = ResourceContents::blob(blob, &uri).with_mime_type("application/pdf");
    CallToolResult::success(vec![Content::text(texto), Content::resource(contents)])
}

/// Lê a guia cacheada SE foi gerada no mesmo dia (fuso America/Sao_Paulo,
/// o do portal). Dia diferente = `None` (o valor mudou, tem que reemitir).
async fn load_guia_cache(
    pool: &Pool,
    client_id: Uuid,
    periodo: &str,
) -> anyhow::Result<Option<GuiaCache>> {
    let row = sql!(
        pool,
        "SELECT competencia, numero_das, total_cents, vencimento, pagar_ate, linha_digitavel, pdf
         FROM zain.das_guia_cache
         WHERE client_id = $client_id
           AND periodo   = $periodo
           AND (gerado_em AT TIME ZONE 'America/Sao_Paulo')::date
             = (now()     AT TIME ZONE 'America/Sao_Paulo')::date"
    )
    .fetch_optional()
    .await?;
    Ok(row.map(|r| GuiaCache {
        competencia: r.competencia,
        numero_das: r.numero_das,
        total_cents: r.total_cents,
        vencimento: r.vencimento,
        pagar_ate: r.pagar_ate,
        linha_digitavel: r.linha_digitavel,
        pdf: r.pdf,
    }))
}

async fn store_guia_cache(
    pool: &Pool,
    client_id: Uuid,
    periodo: &str,
    g: &GuiaCache,
) -> anyhow::Result<()> {
    let competencia = &g.competencia;
    let numero_das = &g.numero_das;
    let total_cents = g.total_cents;
    let vencimento = g.vencimento;
    let pagar_ate = g.pagar_ate;
    let linha_digitavel = g.linha_digitavel.as_deref();
    let pdf = g.pdf.as_slice();
    sql!(
        pool,
        "INSERT INTO zain.das_guia_cache
            (client_id, periodo, gerado_em, competencia, numero_das, total_cents,
             vencimento, pagar_ate, linha_digitavel, pdf)
         VALUES ($client_id, $periodo, now(), $competencia, $numero_das, $total_cents,
             $vencimento, $pagar_ate, $linha_digitavel, $pdf)
         ON CONFLICT (client_id, periodo) DO UPDATE
         SET gerado_em       = now(),
             competencia     = EXCLUDED.competencia,
             numero_das      = EXCLUDED.numero_das,
             total_cents     = EXCLUDED.total_cents,
             vencimento      = EXCLUDED.vencimento,
             pagar_ate       = EXCLUDED.pagar_ate,
             linha_digitavel = EXCLUDED.linha_digitavel,
             pdf             = EXCLUDED.pdf"
    )
    .execute()
    .await?;
    Ok(())
}

// ── Tool `consultar_das` ────────────────────────────────────────────────

#[derive(Deserialize, JsonSchema)]
pub struct ConsultarArgs {}

/// Reconsulta a situação do DAS ao vivo no PGMEI e atualiza o banco,
/// devolvendo o resumo fresco (meses em atraso + próximo vencimento).
/// Reusa o mesmo `refresh_das_status` do worker — então também reancora
/// o agendamento da próxima consulta automática.
pub async fn run_consultar(
    state: &AppState,
    client_id: Uuid,
    _args: ConsultarArgs,
) -> CallToolResult {
    let cnpj = match load_cnpj(&state.pool, client_id).await {
        Ok(Some(c)) => c,
        Ok(None) => {
            return CallToolResult::structured_error(serde_json::json!({
                "status": "erro",
                "motivo": "sem_cnpj",
                "mensagem": "Este lead não tem CNPJ salvo — o DAS só existe pra quem já tem MEI.",
            }));
        }
        Err(e) => {
            tracing::warn!(%client_id, error = %errlog::anyhow_chain(&e), "consultar_das: falha ao ler CNPJ");
            return CallToolResult::structured_error(serde_json::json!({
                "status": "erro",
                "mensagem": "Não consegui ler o cadastro agora. Tente de novo em instantes.",
            }));
        }
    };

    tracing::info!(%client_id, "consultar_das: reconsultando situação no PGMEI");
    if let Err(e) = refresh_das_status(state, client_id, &cnpj).await {
        tracing::warn!(%client_id, error = %errlog::anyhow_chain(&e), "consultar_das: falha ao reconsultar");
        return CallToolResult::structured_error(serde_json::json!({
            "status": "erro",
            "motivo": "pgmei_instavel",
            "mensagem": "Não consegui reconsultar a situação do DAS agora — o portal do Simples Nacional (PGMEI) pode estar instável. NÃO é problema do cliente: agende uma retentativa com `schedule_retentar` e avise que você retoma sozinho.",
        }));
    }

    match resumo_das(&state.pool, client_id).await {
        Ok(v) => CallToolResult::structured(v),
        Err(e) => {
            tracing::warn!(%client_id, error = %errlog::anyhow_chain(&e), "consultar_das: falha ao ler resumo");
            CallToolResult::structured_error(serde_json::json!({
                "status": "erro",
                "mensagem": "Consultei o portal, mas não consegui ler o resultado do banco agora. Tente de novo em instantes.",
            }))
        }
    }
}

/// Resumo estruturado da situação DAS consolidada: meses em atraso e o
/// próximo a vencer. Lê do banco — quem chama já atualizou.
async fn resumo_das(pool: &Pool, client_id: Uuid) -> anyhow::Result<serde_json::Value> {
    let rows = sql!(
        pool,
        "SELECT competencia, situacao, total_cents, vencimento
         FROM zain.das_mensal
         WHERE client_id = $client_id
         ORDER BY periodo"
    )
    .fetch_all()
    .await?;

    let item =
        |competencia: &str, total: Option<i64>, venc: Option<NaiveDate>| -> serde_json::Value {
            serde_json::json!({
                "competencia": competencia,
                "valor": total.map(fmt_cents),
                "vencimento": venc.map(|d| d.format("%d/%m/%Y").to_string()),
            })
        };

    // `devedor` = ano corrente, atraso confirmado (paga a guia normal).
    // `em_aberto` = anos anteriores com valor — pode incluir PARCELADOS
    // (não dá pra distinguir pela consulta; só ao emitir).
    let mut em_atraso: Vec<serde_json::Value> = Vec::new();
    let mut em_aberto: Vec<serde_json::Value> = Vec::new();
    let mut proximo = serde_json::Value::Null;
    for r in &rows {
        match r.situacao.as_str() {
            "devedor" => em_atraso.push(item(&r.competencia, r.total_cents, r.vencimento)),
            "em_aberto" => em_aberto.push(item(&r.competencia, r.total_cents, r.vencimento)),
            "a_vencer" if proximo.is_null() => {
                proximo = item(&r.competencia, r.total_cents, r.vencimento)
            }
            _ => {}
        }
    }

    let em_dia = em_atraso.is_empty() && em_aberto.is_empty();
    let mensagem = if em_dia {
        "Situação reconsultada ao vivo no portal: nenhum mês em aberto."
    } else if em_aberto.is_empty() {
        "Situação reconsultada ao vivo. Lembre: pagamento leva 1–2 dias úteis pra compensar — se o cliente acabou de pagar e o mês ainda consta em atraso, é normal."
    } else {
        "Situação reconsultada ao vivo. ATENÇÃO aos meses em `em_aberto` (anos anteriores): podem estar em PARCELAMENTO — não dá pra saber pela consulta, só ao emitir. Pra cada mês, `emitir_das` devolve a guia OU avisa se está parcelado (aí o pagamento é pelo app de parcelamento)."
    };
    Ok(serde_json::json!({
        "status": "ok",
        "em_dia": em_dia,
        "meses_em_atraso": em_atraso,
        "em_aberto_anos_anteriores": em_aberto,
        "proximo_vencimento": proximo,
        "mensagem": mensagem,
    }))
}

/// Normaliza o `periodo` do argumento ou escolhe um a partir do banco:
/// o mês mais antigo em atraso, senão o próximo a vencer. `Err` devolve
/// a resposta de erro pronta pro caller.
async fn resolver_periodo(
    pool: &Pool,
    client_id: Uuid,
    arg: Option<&str>,
) -> Result<String, CallToolResult> {
    if let Some(raw) = arg {
        let digits: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
        let normalizado = match digits.len() {
            // YYYYMM direto, ou MMYYYY (ex: "06/2026" → "062026").
            6 if digits[..4]
                .parse::<u32>()
                .is_ok_and(|y| (2000..2100).contains(&y))
                && digits[4..]
                    .parse::<u32>()
                    .is_ok_and(|m| (1..=12).contains(&m)) =>
            {
                Some(digits.clone())
            }
            6 if digits[..2]
                .parse::<u32>()
                .is_ok_and(|m| (1..=12).contains(&m))
                && digits[2..]
                    .parse::<u32>()
                    .is_ok_and(|y| (2000..2100).contains(&y)) =>
            {
                Some(format!("{}{}", &digits[2..], &digits[..2]))
            }
            _ => None,
        };
        return normalizado.ok_or_else(|| {
            CallToolResult::structured_error(serde_json::json!({
                "status": "erro",
                "motivo": "periodo_invalido",
                "mensagem": format!("Período {raw:?} inválido — use o formato YYYYMM (ex: '202604' = abril/2026)."),
            }))
        });
    }

    // Sem argumento: mês mais antigo em atraso, senão o próximo a vencer.
    let row = sql!(
        pool,
        "SELECT periodo
         FROM zain.das_mensal
         WHERE client_id = $client_id
           AND situacao IN ('devedor', 'a_vencer')
         ORDER BY (situacao != 'devedor'), periodo
         LIMIT 1"
    )
    .fetch_optional()
    .await;
    match row {
        Ok(Some(r)) => Ok(r.periodo),
        Ok(None) => Err(CallToolResult::structured_error(serde_json::json!({
            "status": "erro",
            "motivo": "sem_periodo",
            "mensagem": "Não há situação DAS consolidada pra escolher o mês automaticamente. Informe o `periodo` no formato YYYYMM (ex: '202604' = abril/2026).",
        }))),
        Err(e) => {
            tracing::warn!(%client_id, error = %e.chain_string(), "emitir_das: falha ao escolher período");
            Err(CallToolResult::structured_error(serde_json::json!({
                "status": "erro",
                "mensagem": "Não consegui consultar a situação DAS no banco agora. Tente de novo em instantes.",
            })))
        }
    }
}

async fn load_cnpj(pool: &Pool, client_id: Uuid) -> anyhow::Result<Option<String>> {
    let row = sql!(pool, "SELECT cnpj FROM zain.clients WHERE id = $client_id")
        .fetch_optional()
        .await?;
    Ok(row.and_then(|r| r.cnpj))
}

/// 9344 → "R$ 93,44".
pub(crate) fn fmt_cents(cents: i64) -> String {
    format!("R$ {},{:02}", cents / 100, cents % 100)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formata_dinheiro() {
        assert_eq!(fmt_cents(9344), "R$ 93,44");
        assert_eq!(fmt_cents(8605), "R$ 86,05");
        assert_eq!(fmt_cents(123456), "R$ 1234,56");
    }
}

#[cfg(test)]
mod manual_tests {
    //! Harness manual contra o banco real + RPA no PGMEI. Roda a tool
    //! `consultar_das` pra UM cliente e imprime o JSON que o agente recebe.
    //! Ignorado por default; rode com:
    //!
    //!   CLIENT_ID=<uuid> cargo test -p mcp tools::das::manual_tests::consultar_real -- --ignored --nocapture
    use std::sync::Arc;

    use uuid::Uuid;

    use super::*;
    use crate::state::{AppState, Models};

    fn build_state() -> AppState {
        dotenvy::dotenv_override().ok();
        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL não definido");
        let mut cfg = deadpool_postgres::Config::new();
        cfg.url = Some(database_url);
        let pool = cfg
            .create_pool(
                Some(deadpool_postgres::Runtime::Tokio1),
                tokio_postgres::NoTls,
            )
            .expect("criar pool");
        let ai = Arc::new(ai::Client::from_env());
        let models = Arc::new(Models::from_env().expect("Models::from_env"));
        AppState { pool, ai, models }
    }

    #[ignore]
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn consultar_real() {
        let _ = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .with_test_writer()
            .try_init();
        let state = build_state();
        let client_id: Uuid = std::env::var("CLIENT_ID")
            .expect("set CLIENT_ID=<uuid>")
            .parse()
            .expect("CLIENT_ID uuid inválido");

        let r = run_consultar(&state, client_id, ConsultarArgs {}).await;
        let v = serde_json::to_value(&r).unwrap_or_default();
        println!(
            "\n===== resultado consultar_das =====\n{}",
            serde_json::to_string_pretty(&v).unwrap_or_default()
        );
    }

    /// Roda `run_emitir` pra um cliente/período. Mostra o JSON que o agente
    /// recebe (cache hit, guia emitida, ou erro classificado — ex.: limite
    /// diário). Rode com:
    ///   CLIENT_ID=<uuid> PERIODO=202604 cargo test -p mcp tools::das::manual_tests::emitir_real -- --ignored --nocapture
    #[ignore]
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn emitir_real() {
        let _ = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .with_test_writer()
            .try_init();
        let state = build_state();
        let client_id: Uuid = std::env::var("CLIENT_ID")
            .expect("set CLIENT_ID=<uuid>")
            .parse()
            .expect("CLIENT_ID uuid inválido");
        let periodo = std::env::var("PERIODO").ok();

        let r = run_emitir(&state, client_id, EmitirArgs { periodo }).await;
        let v = serde_json::to_value(&r).unwrap_or_default();
        // Resume o resultado: erro estruturado vem em structuredContent; a
        // guia (sucesso) vem em content[text] + content[resource].
        let sc = v.get("structuredContent").cloned().unwrap_or_default();
        let texto = v
            .get("content")
            .and_then(|c| c.as_array())
            .and_then(|a| a.iter().find_map(|x| x.get("text")))
            .and_then(|t| t.as_str())
            .unwrap_or("(sem texto)");
        let tem_pdf = v
            .get("content")
            .and_then(|c| c.as_array())
            .map(|a| a.iter().any(|x| x.get("resource").is_some()))
            .unwrap_or(false);
        println!("\n===== resultado emitir_das =====");
        println!(
            "structuredContent (erro, se houver):\n{}",
            serde_json::to_string_pretty(&sc).unwrap_or_default()
        );
        println!("texto: {texto}");
        println!("tem_pdf_anexado: {tem_pdf}");
    }

    /// Valida o cache same-day SEM emitir fresh: insere um PDF real no cache
    /// e confirma que `run_emitir` serve dele (sem RPA, sem gastar o limite
    /// diário). Rode com:
    ///   CLIENT_ID=<uuid> PERIODO=202604 PDF=/tmp/das-202604.pdf cargo test -p mcp tools::das::manual_tests::cache_roundtrip -- --ignored --nocapture
    #[ignore]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn cache_roundtrip() {
        let _ = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .with_test_writer()
            .try_init();
        let state = build_state();
        let client_id: Uuid = std::env::var("CLIENT_ID")
            .expect("set CLIENT_ID=<uuid>")
            .parse()
            .expect("CLIENT_ID uuid inválido");
        let periodo = std::env::var("PERIODO").expect("set PERIODO=YYYYMM");
        let pdf_path = std::env::var("PDF").expect("set PDF=<caminho>");
        let pdf = std::fs::read(&pdf_path).expect("ler PDF");

        let cache = GuiaCache {
            competencia: "Abril/2026".into(),
            numero_das: "TESTE-CACHE".into(),
            total_cents: Some(9344),
            vencimento: parse_iso("2026-05-20"),
            pagar_ate: parse_iso("2026-06-12"),
            linha_digitavel: Some("85800000000".into()),
            pdf,
        };
        store_guia_cache(&state.pool, client_id, &periodo, &cache)
            .await
            .expect("store cache");

        // Agora run_emitir deve servir do cache (sem subir browser).
        let t0 = std::time::Instant::now();
        let r = run_emitir(
            &state,
            client_id,
            EmitirArgs {
                periodo: Some(periodo),
            },
        )
        .await;
        let elapsed = t0.elapsed();
        let v = serde_json::to_value(&r).unwrap_or_default();
        let tem_pdf = v
            .get("content")
            .and_then(|c| c.as_array())
            .map(|a| a.iter().any(|x| x.get("resource").is_some()))
            .unwrap_or(false);
        let is_error = v.get("isError").and_then(|b| b.as_bool()).unwrap_or(false);
        println!("\n===== cache_roundtrip =====");
        println!("tempo: {elapsed:?} (cache hit deve ser <1s; emitir fresh seria ~30-60s)");
        println!("is_error: {is_error}  tem_pdf_anexado: {tem_pdf}");
    }
}
