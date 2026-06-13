//! RPA do PGMEI — Programa Gerador do DAS do MEI.
//!
//! O PGMEI é o portal onde o MEI consulta e emite a guia mensal (DAS).
//! Diferente do CCMEI/inscrição, o acesso é **público por CNPJ** — não
//! exige sessão gov.br — mas a identificação roda um **hCaptcha
//! invisível** no submit (resolvido em background pela extensão NopeCHA,
//! como nos demais fluxos).
//!
//! Duas rotas:
//!
//! - [`consultar_das`]: lê a tabela de períodos da tela "Emitir Guia de
//!   Pagamento (DAS)" de um ano-calendário — situação mês a mês
//!   (Liquidado / Devedor / A Vencer / Não Optante), valores (principal,
//!   multa, juros, total), vencimento e acolhimento. É a fonte que o
//!   worker `das_refresh` consolida no banco.
//! - [`emitir_das`]: marca um período, clica "Apurar/Gerar DAS" e baixa
//!   o PDF da guia (com código de barras + QR PIX). A guia de mês em
//!   atraso é recalculada por dia (multa/juros) — por isso a emissão é
//!   sempre on-demand e o PDF nunca é cacheado.
//!
//! O portal é ASP.NET clássico (postbacks de página inteira) com máscara
//! jQuery no CNPJ — digitação precisa de teclas reais via CDP, e cada
//! submit é seguido de `wait_for_load` + DOM novo.

use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use chromium_driver::PageSession;
use serde::{Deserialize, Serialize};

use crate::govbr::launch;
use crate::sanity;

const IDENTIFICACAO_URL: &str =
    "https://www8.receita.fazenda.gov.br/SimplesNacional/Aplicacoes/ATSPO/pgmei.app/Identificacao";
const EMISSAO_URL: &str =
    "https://www8.receita.fazenda.gov.br/SimplesNacional/Aplicacoes/ATSPO/pgmei.app/emissao";
const IMPRIMIR_PATH: &str = "/SimplesNacional/Aplicacoes/ATSPO/pgmei.app/emissao/imprimir";

const TIMEOUT: Duration = Duration::from_secs(30);
/// O hCaptcha invisível roda entre o clique em "Continuar" e o redirect
/// pra Home — normalmente resolve em segundos, mas damos folga.
const CAPTCHA_TIMEOUT: Duration = Duration::from_secs(90);

/// Situação de um período de apuração, classificada do texto da célula.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SituacaoDas {
    /// DAS pago.
    Liquidado,
    /// Vencido e não pago — confirmado pela coluna "Situação" (ano corrente).
    Devedor,
    /// Há valor a regularizar, mas a situação EXATA não veio na tabela
    /// (anos passados não têm a coluna "Situação"). Pode ser devedor puro
    /// OU já parcelado — só dá pra saber ao emitir. NÃO afirme "devedor".
    EmAberto,
    /// Dentro do prazo, ainda não vencido.
    AVencer,
    /// Mês anterior ao enquadramento no SIMEI (não devido).
    NaoOptante,
    /// Texto não reconhecido — mantido cru em `situacao_texto`.
    Outra,
}

impl SituacaoDas {
    pub fn as_str(self) -> &'static str {
        match self {
            SituacaoDas::Liquidado => "liquidado",
            SituacaoDas::Devedor => "devedor",
            SituacaoDas::EmAberto => "em_aberto",
            SituacaoDas::AVencer => "a_vencer",
            SituacaoDas::NaoOptante => "nao_optante",
            SituacaoDas::Outra => "outra",
        }
    }

    fn classificar(texto: &str) -> Self {
        let t = texto.to_lowercase();
        if t.contains("liquidado") {
            SituacaoDas::Liquidado
        } else if t.contains("devedor") {
            SituacaoDas::Devedor
        } else if t.contains("a vencer") {
            SituacaoDas::AVencer
        } else if t.contains("não optante") || t.contains("nao optante") {
            SituacaoDas::NaoOptante
        } else {
            SituacaoDas::Outra
        }
    }
}

/// Um período de apuração (mês) da tabela de emissão do PGMEI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DasMensal {
    /// Competência no formato `YYYYMM` (value do checkbox do portal).
    pub periodo: String,
    /// Rótulo humano, ex: "Abril/2026".
    pub competencia: String,
    pub apurado: bool,
    pub situacao: SituacaoDas,
    /// Texto cru da célula de situação (ex: "Liquidado em 10/05/2026").
    pub situacao_texto: String,
    /// Se o checkbox do período está habilitado (dá pra gerar DAS).
    pub emissivel: bool,
    pub principal_cents: Option<i64>,
    pub multa_cents: Option<i64>,
    pub juros_cents: Option<i64>,
    pub total_cents: Option<i64>,
    /// Data de vencimento em ISO (YYYY-MM-DD).
    pub vencimento: Option<String>,
    /// Validade da guia se emitida hoje (data de acolhimento), ISO.
    pub acolhimento: Option<String>,
}

/// Guia DAS emitida: metadados da tela "DAS gerados" + PDF.
#[derive(Debug, Clone)]
pub struct GuiaDas {
    pub periodo: String,
    pub competencia: String,
    /// Número do documento (ex: "07.08.26163.3946856-5").
    pub numero_das: String,
    /// Vencimento original da competência, ISO.
    pub vencimento: Option<String>,
    /// "Pagar este documento até" extraído do PDF, ISO. Pra mês em
    /// atraso é tipicamente o próprio dia da emissão.
    pub pagar_ate: Option<String>,
    /// Total da guia em centavos (da tabela de períodos).
    pub total_cents: Option<i64>,
    /// Linha digitável do código de barras (48 dígitos), do PDF.
    pub linha_digitavel: Option<String>,
    /// PDF da guia — código de barras + QR code PIX.
    pub pdf: Vec<u8>,
}

/// Consulta a situação mensal do DAS de um CNPJ num ano-calendário.
pub async fn consultar_das(cnpj: &str, ano: i32) -> anyhow::Result<Vec<DasMensal>> {
    let digits = cnpj_digits(cnpj)?;
    let opts = launch::options_with_extensions().await?;
    let (mut process, browser) = chromium_driver::launch(opts).await?;

    let result = async {
        launch::configure_nopecha(&browser).await.ok();
        let page = browser.create_page("about:blank").await?.attach().await?;
        page.enable().await?;
        identificar(&page, &digits).await?;
        abrir_emissao_do_ano(&page, ano).await?;
        extrair_tabela(&page).await
    }
    .await;

    let _ = browser.close().await;
    let _ = process.wait().await;
    result
}

/// Consulta vários anos-calendário numa ÚNICA sessão (identifica uma vez,
/// depois troca o ano e re-extrai). Bem mais barato que um browser por
/// ano. Devolve só os anos extraídos com sucesso — um ano indisponível no
/// portal (fora da janela do dropdown) ou com glitch transitório é logado
/// e pulado, sem derrubar os demais. O `Err` externo é só pra falha de
/// sessão (launch/identificação) — aí nenhum ano foi lido.
pub async fn consultar_das_anos(
    cnpj: &str,
    anos: &[i32],
) -> anyhow::Result<Vec<(i32, Vec<DasMensal>)>> {
    let digits = cnpj_digits(cnpj)?;
    let opts = launch::options_with_extensions().await?;
    let (mut process, browser) = chromium_driver::launch(opts).await?;

    let result = async {
        launch::configure_nopecha(&browser).await.ok();
        let page = browser.create_page("about:blank").await?.attach().await?;
        page.enable().await?;
        // CNPJ digitado UMA vez; emissão aberta UMA vez. Os anos são só
        // trocas do dropdown lá dentro.
        identificar(&page, &digits).await?;
        ir_para_emissao(&page).await?;

        let mut out = Vec::new();
        for &ano in anos {
            let r = async {
                selecionar_ano(&page, ano).await?;
                extrair_tabela(&page).await
            }
            .await;
            match r {
                Ok(meses) => out.push((ano, meses)),
                Err(e) => tracing::warn!(ano, error = %e, "consultar_das_anos: ano pulado (indisponível ou falha transitória)"),
            }
        }
        Ok(out)
    }
    .await;

    let _ = browser.close().await;
    let _ = process.wait().await;
    result
}

/// Emite o DAS de uma competência (`YYYYMM`) e baixa o PDF da guia.
pub async fn emitir_das(cnpj: &str, periodo: &str) -> anyhow::Result<GuiaDas> {
    let digits = cnpj_digits(cnpj)?;
    let periodo: String = periodo.chars().filter(|c| c.is_ascii_digit()).collect();
    let ano: i32 = periodo
        .get(..4)
        .and_then(|a| a.parse().ok())
        .filter(|_| periodo.len() == 6)
        .ok_or_else(|| anyhow::anyhow!("período deve ser YYYYMM, recebido {periodo:?}"))?;

    let opts = launch::options_with_extensions().await?;
    let (mut process, browser) = chromium_driver::launch(opts).await?;

    let result = async {
        launch::configure_nopecha(&browser).await.ok();
        let page = browser.create_page("about:blank").await?.attach().await?;
        page.enable().await?;
        identificar(&page, &digits).await?;
        abrir_emissao_do_ano(&page, ano).await?;

        let meses = extrair_tabela(&page).await?;
        let Some(mes) = meses.iter().find(|m| m.periodo == periodo) else {
            anyhow::bail!(
                "período {periodo} não existe na tabela do PGMEI (ano {ano}); períodos: {}",
                meses
                    .iter()
                    .map(|m| m.periodo.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        };
        if !mes.emissivel {
            anyhow::bail!(
                "período {periodo} ({}) não está emissível no PGMEI — situação: {}",
                mes.competencia,
                mes.situacao_texto
            );
        }
        let mes = mes.clone();

        // Marca o período e gera o DAS. O click no checkbox dispara o JS
        // que preenche o "Resumo do DAS a ser gerado" — pequena folga.
        marcar_e_gerar(&page, &periodo).await?;

        // Tela "DAS gerados": extrai número + vencimento da linha.
        let (numero_das, vencimento_br) = extrair_resultado(&page).await?;

        // PDF via fetch in-page (mesma sessão/cookies do browser).
        let pdf = baixar_pdf(&page).await?;
        let (pagar_ate, linha_digitavel) = parse_pdf_info(&pdf);

        Ok(GuiaDas {
            periodo: periodo.clone(),
            competencia: mes.competencia,
            numero_das,
            vencimento: parse_date_br(&vencimento_br).or(mes.vencimento),
            pagar_ate,
            total_cents: mes.total_cents,
            linha_digitavel,
            pdf,
        })
    }
    .await;

    let _ = browser.close().await;
    let _ = process.wait().await;
    result
}

fn cnpj_digits(cnpj: &str) -> anyhow::Result<String> {
    let digits: String = cnpj.chars().filter(|c| c.is_ascii_digit()).collect();
    anyhow::ensure!(
        digits.len() == 14,
        "CNPJ deve ter 14 dígitos, recebido {}",
        digits.len()
    );
    Ok(digits)
}

/// Tela de identificação: digita o CNPJ (máscara jQuery — teclas reais),
/// clica "Continuar" e espera o redirect pra Home. O hCaptcha invisível
/// roda nesse intervalo; um toast de erro do portal aborta na hora.
async fn identificar(page: &PageSession, cnpj_digits: &str) -> anyhow::Result<()> {
    page.navigate(IDENTIFICACAO_URL).await?;
    page.wait_for_load(TIMEOUT).await.ok();

    let dom = page.dom().await?;
    sanity::wait_for(page, dom, "pgmei: identificação", "#cnpj", TIMEOUT).await?;

    // A máscara jQuery do #cnpj só anexa no `document.ready`; se a gente
    // digita durante um reflow ou antes do handler ligar, as primeiras
    // teclas se perdem e o campo fica com os últimos dígitos (visto em
    // prod: campo com "27"). Então: digita, lê de volta os dígitos, e
    // retenta limpando o campo até bater — com folga crescente.
    let mut ultimo_valor = String::new();
    let mut registrou = false;
    for tentativa in 1..=4 {
        tokio::time::sleep(Duration::from_millis(200 * tentativa)).await;

        // Foco + limpa via setter nativo (a máscara ignora value= via JS puro).
        page.eval_value(
            r#"(() => {
                const el = document.querySelector('#cnpj');
                el.focus();
                const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set;
                setter.call(el, '');
                el.dispatchEvent(new Event('input', { bubbles: true }));
                return 'ok';
            })()"#,
        )
        .await?;
        let el = dom.query_selector("#cnpj").await?;
        el.type_text(cnpj_digits).await?;

        ultimo_valor = page
            .eval_value("document.querySelector('#cnpj').value")
            .await?
            .as_str()
            .unwrap_or("")
            .chars()
            .filter(|c| c.is_ascii_digit())
            .collect();
        if ultimo_valor == cnpj_digits {
            registrou = true;
            break;
        }
        tracing::warn!(
            tentativa,
            campo = %ultimo_valor,
            "pgmei: máscara do CNPJ não bateu, retentando"
        );
    }
    if !registrou {
        return Err(sanity::fail(
            page,
            "pgmei: identificação",
            &format!(
                "máscara não registrou o CNPJ após 4 tentativas (campo ficou com {ultimo_valor:?})"
            ),
        )
        .await);
    }

    page.eval_value(
        r#"(() => {
            const b = document.querySelector('#continuar');
            if (!b) return 'not_found';
            b.click();
            return 'ok';
        })()"#,
    )
    .await?
    .as_str()
    .filter(|s| *s == "ok")
    .ok_or_else(|| anyhow::anyhow!("botão #continuar não encontrado na identificação"))?;

    let deadline = tokio::time::Instant::now() + CAPTCHA_TIMEOUT;
    loop {
        tokio::time::sleep(Duration::from_secs(2)).await;
        let url = page
            .eval_value("location.href")
            .await?
            .as_str()
            .unwrap_or("")
            .to_string();
        if !url.contains("Identificacao") {
            return Ok(());
        }
        let toast = page
            .eval_value(
                r#"(() => {
                    const t = document.querySelector('.toast-message');
                    return t ? (t.textContent || '').trim() : null;
                })()"#,
            )
            .await
            .ok()
            .and_then(|v| v.as_str().map(str::to_string))
            .filter(|t| !t.is_empty());
        if let Some(t) = toast {
            return Err(sanity::fail(
                page,
                "pgmei: identificação",
                &format!("portal rejeitou: {t}"),
            )
            .await);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(sanity::fail(
                page,
                "pgmei: identificação",
                "timeout aguardando sair da identificação (hCaptcha não resolvido?)",
            )
            .await);
        }
    }
}

/// Navega pra tela de emissão UMA vez (o CNPJ já vem identificado na
/// sessão). Depois é só trocar o ano com [`selecionar_ano`] — sem
/// re-navegar nem re-digitar o CNPJ.
async fn ir_para_emissao(page: &PageSession) -> anyhow::Result<()> {
    page.navigate(EMISSAO_URL).await?;
    page.wait_for_load(TIMEOUT).await.ok();
    let dom = page.dom().await?;
    sanity::wait_for(page, dom, "pgmei: emissão", "#anoCalendarioSelect", TIMEOUT).await?;
    Ok(())
}

/// Troca o ano-calendário pelo dropdown (postback) e espera a tabela do
/// ano PEDIDO hidratar. Assume que já estamos na tela de emissão. Confere
/// que a tabela é mesmo do ano alvo, pra não ler a do ano anterior que
/// ainda está no DOM enquanto o postback não terminou.
async fn selecionar_ano(page: &PageSession, ano: i32) -> anyhow::Result<()> {
    let dom = page.dom().await?;
    sanity::wait_for(
        page,
        dom,
        "pgmei: emissão (ano)",
        "#anoCalendarioSelect",
        TIMEOUT,
    )
    .await?;

    let escolhido = page
        .eval_value(&format!(
            r#"(() => {{
                const sel = document.querySelector('#anoCalendarioSelect');
                const alvo = String({ano});
                if (![...sel.options].some(o => o.value === alvo)) {{
                    return 'indisponivel: ' + [...sel.options].map(o => o.value).join(',');
                }}
                sel.value = alvo;
                sel.dispatchEvent(new Event('change', {{ bubbles: true }}));
                const form = sel.closest('form');
                const btn = form && form.querySelector('button[type=submit]');
                if (!btn) return 'sem_submit';
                btn.click();
                return 'ok';
            }})()"#
        ))
        .await?
        .as_str()
        .unwrap_or("")
        .to_string();
    if escolhido != "ok" {
        return Err(sanity::fail(
            page,
            "pgmei: emissão (ano)",
            &format!("não consegui selecionar o ano {ano}: {escolhido}"),
        )
        .await);
    }

    page.wait_for_load(TIMEOUT).await.ok();
    // Espera a tabela do ANO pedido (a do ano anterior pode ficar no DOM
    // até o postback completar — checamos a competência da 1ª linha).
    let deadline = tokio::time::Instant::now() + TIMEOUT;
    loop {
        let pronto = page
            .eval_value(&format!(
                r#"(() => {{
                    const rows = [...document.querySelectorAll('table.emissao tbody tr.pa')];
                    if (!rows.length) return false;
                    const comp = (rows[0].querySelectorAll('td')[1] || {{}}).textContent || '';
                    return comp.includes('/{ano}');
                }})()"#
            ))
            .await?
            .as_bool()
            .unwrap_or(false);
        if pronto {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(sanity::fail(
                page,
                "pgmei: emissão (tabela de períodos)",
                &format!("a tabela não atualizou pro ano {ano} a tempo"),
            )
            .await);
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
}

/// Conveniência pra um único ano: navega pra emissão e seleciona o ano.
async fn abrir_emissao_do_ano(page: &PageSession, ano: i32) -> anyhow::Result<()> {
    ir_para_emissao(page).await?;
    selecionar_ano(page, ano).await
}

/// Extrai a tabela de períodos num único eval (evita N roundtrips CDP).
async fn extrair_tabela(page: &PageSession) -> anyhow::Result<Vec<DasMensal>> {
    let v = page
        .eval_value(
            r#"(() => {
                const limpar = el => el ? (el.textContent || '').replace(/\s+/g, ' ').trim() : '';
                return [...document.querySelectorAll('table.emissao tbody tr.pa')].map(tr => {
                    const cb = tr.querySelector('input.paSelecionado');
                    const tds = [...tr.querySelectorAll('td')];
                    return {
                        periodo: cb ? cb.value : null,
                        emissivel: cb ? !cb.disabled : false,
                        competencia: limpar(tds[1]),
                        apurado: limpar(tds[2]) === 'Sim',
                        situacao_texto: limpar(tr.querySelector('td.situacaoPa')),
                        principal: limpar(tr.querySelector('td.principal')),
                        multa: limpar(tr.querySelector('td.multa')),
                        juros: limpar(tr.querySelector('td.juros')),
                        total: limpar(tr.querySelector('td.total')),
                        vencimento: limpar(tr.querySelector('td.vencimento')),
                        acolhimento: limpar(tr.querySelector('td.acolhimento')),
                    };
                });
            })()"#,
        )
        .await?;

    let Some(rows) = v.as_array() else {
        return Err(sanity::fail(
            page,
            "pgmei: extrair tabela de períodos",
            "eval não devolveu array",
        )
        .await);
    };

    let mut meses = Vec::with_capacity(rows.len());
    for row in rows {
        let get = |k: &str| -> String {
            row.get(k)
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string()
        };
        let Some(periodo) = row.get("periodo").and_then(|p| p.as_str()) else {
            continue;
        };
        let situacao_texto = get("situacao_texto");
        let apurado = row
            .get("apurado")
            .and_then(|a| a.as_bool())
            .unwrap_or(false);
        let total_cents = parse_money_cents(&get("total"));
        // A coluna "Situação" só existe no ano corrente; em anos passados
        // (layout sem a coluna) `situacao_texto` vem vazio → `classificar`
        // devolve `Outra`. Aí derivamos dos campos que SEMPRE vêm:
        // - "-" + apurado    → liquidado (pago).
        // - "-" + não apurado → não optante (antes da vigência do MEI).
        // - tem total        → `em_aberto`: há valor a regularizar, mas NÃO
        //   dá pra afirmar "devedor" — pode estar parcelado (o portal mostra
        //   o valor na tabela mesmo quando parcelado). Só ao emitir se sabe.
        let situacao = match SituacaoDas::classificar(&situacao_texto) {
            SituacaoDas::Outra if total_cents.is_some() => SituacaoDas::EmAberto,
            SituacaoDas::Outra if apurado => SituacaoDas::Liquidado,
            SituacaoDas::Outra => SituacaoDas::NaoOptante,
            s => s,
        };
        meses.push(DasMensal {
            periodo: periodo.to_string(),
            competencia: get("competencia"),
            apurado,
            situacao,
            situacao_texto,
            emissivel: row
                .get("emissivel")
                .and_then(|e| e.as_bool())
                .unwrap_or(false),
            principal_cents: parse_money_cents(&get("principal")),
            multa_cents: parse_money_cents(&get("multa")),
            juros_cents: parse_money_cents(&get("juros")),
            total_cents,
            vencimento: parse_date_br(&get("vencimento")),
            acolhimento: parse_date_br(&get("acolhimento")),
        });
    }
    if meses.is_empty() {
        return Err(sanity::fail(
            page,
            "pgmei: extrair tabela de períodos",
            "tabela de períodos vazia",
        )
        .await);
    }
    Ok(meses)
}

/// Marca o checkbox do período e clica "Apurar/Gerar DAS", esperando a
/// tela "DAS gerados" (link de impressão presente).
async fn marcar_e_gerar(page: &PageSession, periodo: &str) -> anyhow::Result<()> {
    let marcado = page
        .eval_value_with_args(
            r#"(periodo) => {
                const cb = document.querySelector(`input.paSelecionado[value="${periodo}"]`);
                if (!cb) return 'not_found';
                if (cb.disabled) return 'disabled';
                if (!cb.checked) cb.click();
                return 'ok';
            }"#,
            &[serde_json::json!(periodo)],
        )
        .await?
        .as_str()
        .unwrap_or("")
        .to_string();
    if marcado != "ok" {
        return Err(sanity::fail(
            page,
            "pgmei: marcar período",
            &format!("checkbox do período {periodo}: {marcado}"),
        )
        .await);
    }
    // O click dispara o cálculo do "Resumo do DAS a ser gerado".
    tokio::time::sleep(Duration::from_millis(700)).await;

    page.eval_value(
        r#"(() => {
            const b = document.querySelector('#btnEmitirDas');
            if (!b) return 'not_found';
            b.click();
            return 'ok';
        })()"#,
    )
    .await?
    .as_str()
    .filter(|s| *s == "ok")
    .ok_or_else(|| anyhow::anyhow!("botão #btnEmitirDas não encontrado"))?;

    page.wait_for_load(TIMEOUT).await.ok();

    // Lê o RESULTADO + os toasts. Três desfechos:
    // - PARCELADO: o mês está num parcelamento; o portal gera a guia (toast
    //   verde de sucesso + link de imprimir) MAS mostra um toast laranja
    //   `.toast-warning` "…parcelados e devem ser pagos por meio de DAS
    //   gerado no aplicativo de parcelamento". Essa guia NÃO deve ser paga —
    //   tem prioridade sobre o sucesso.
    // - ERRO: toast vermelho `.toast-error` (ex.: "23998 - … limite diário
    //   excedido!"). Falha na hora com a mensagem real.
    // - SUCESSO: link de imprimir presente e nenhum toast de parcelamento.
    //   Como o toast de parcelamento chega na MESMA resposta do sucesso,
    //   damos ~1s de folga depois de ver o link antes de concluir, pra não
    //   correr na frente do toast.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    let mut primeiro_ok: Option<tokio::time::Instant> = None;
    loop {
        let estado = page
            .eval_value(
                r#"(() => {
                    const msgs = [...document.querySelectorAll('.toast-message')]
                        .map(t => (t.textContent || '').trim()).filter(Boolean);
                    const parcel = msgs.find(t => /parcelad/i.test(t));
                    if (parcel) return 'parcelado:' + parcel;
                    const err = [...document.querySelectorAll('.toast-error .toast-message')]
                        .map(t => (t.textContent || '').trim()).find(Boolean);
                    if (err) return 'erro:' + err;
                    if (document.querySelector('a[href*="emissao/imprimir"]')) return 'ok';
                    return 'pending';
                })()"#,
            )
            .await?
            .as_str()
            .unwrap_or("pending")
            .to_string();

        if let Some(msg) = estado.strip_prefix("parcelado:") {
            // Sinaliza com a palavra "parcelado" pra o caller (mcp) tratar.
            return Err(sanity::fail(
                page,
                "pgmei: gerar DAS",
                &format!("período parcelado: {msg}"),
            )
            .await);
        }
        if let Some(msg) = estado.strip_prefix("erro:") {
            return Err(
                sanity::fail(page, "pgmei: gerar DAS", &format!("portal recusou: {msg}")).await,
            );
        }
        if estado == "ok" {
            match primeiro_ok {
                None => primeiro_ok = Some(tokio::time::Instant::now()),
                Some(t0) if t0.elapsed() >= Duration::from_millis(1000) => return Ok(()),
                _ => {}
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(sanity::fail(
                page,
                "pgmei: gerar DAS",
                "timeout aguardando a guia ser gerada (portal lento/instável)",
            )
            .await);
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
}

/// Extrai (número do DAS, vencimento BR) da tabela da tela "DAS gerados".
async fn extrair_resultado(page: &PageSession) -> anyhow::Result<(String, String)> {
    let v = page
        .eval_value(
            r#"(() => {
                const link = document.querySelector('a[href*="emissao/imprimir"]');
                const panel = link ? link.closest('.panel') : null;
                const tr = (panel || document).querySelector('tbody tr');
                if (!tr) return null;
                const tds = [...tr.querySelectorAll('td')].map(td => (td.textContent || '').replace(/\s+/g, ' ').trim());
                return { numero_das: tds[2] || '', vencimento: tds[3] || '' };
            })()"#,
        )
        .await?;
    let Some(obj) = v.as_object() else {
        return Err(sanity::fail(
            page,
            "pgmei: resultado da emissão",
            "tabela 'DAS gerados' não encontrada",
        )
        .await);
    };
    let numero = obj
        .get("numero_das")
        .and_then(|n| n.as_str())
        .unwrap_or("")
        .to_string();
    let vencimento = obj
        .get("vencimento")
        .and_then(|n| n.as_str())
        .unwrap_or("")
        .to_string();
    if numero.is_empty() {
        return Err(sanity::fail(
            page,
            "pgmei: resultado da emissão",
            "número do DAS vazio na tela de resultado",
        )
        .await);
    }
    Ok((numero, vencimento))
}

/// Baixa o PDF da guia via `fetch` dentro da página (reusa cookies da
/// sessão do portal) e devolve os bytes.
async fn baixar_pdf(page: &PageSession) -> anyhow::Result<Vec<u8>> {
    let v = page
        .eval_value_async(&format!(
            r#"(async () => {{
                const r = await fetch('{IMPRIMIR_PATH}', {{ credentials: 'include' }});
                if (!r.ok) return 'HTTP ' + r.status;
                const buf = await r.arrayBuffer();
                const bytes = new Uint8Array(buf);
                let bin = '';
                const chunk = 0x8000;
                for (let i = 0; i < bytes.length; i += chunk) {{
                    bin += String.fromCharCode.apply(null, bytes.subarray(i, i + chunk));
                }}
                return btoa(bin);
            }})()"#
        ))
        .await?;
    let b64 = v.as_str().unwrap_or("");
    if b64.starts_with("HTTP ") || b64.is_empty() {
        return Err(sanity::fail(
            page,
            "pgmei: baixar PDF",
            &format!("download do PDF da guia falhou: {b64:?}"),
        )
        .await);
    }
    let pdf = BASE64_STANDARD.decode(b64)?;
    anyhow::ensure!(
        pdf.starts_with(b"%PDF"),
        "conteúdo baixado não é PDF ({} bytes)",
        pdf.len()
    );
    Ok(pdf)
}

/// Extrai do texto do PDF da guia o "pagar até" (ISO) e a linha digitável
/// (48 dígitos). Best-effort: o PDF anexado já resolve o cliente; estes
/// campos são conveniência pro agente colar em texto.
fn parse_pdf_info(pdf: &[u8]) -> (Option<String>, Option<String>) {
    let Ok(doc) = lopdf::Document::load_mem(pdf) else {
        return (None, None);
    };
    let pages: Vec<u32> = doc.get_pages().keys().copied().collect();
    let Ok(texto) = doc.extract_text(&pages) else {
        return (None, None);
    };
    (extrair_pagar_ate(&texto), extrair_linha_digitavel(&texto))
}

/// Procura "Pagar este documento até" (ou "Pagar até") e devolve a
/// primeira data DD/MM/YYYY logo em seguida, em ISO. A varredura é por
/// bytes (datas são ASCII puro) — slicing por índice de char quebraria
/// nos acentos do texto ao redor.
fn extrair_pagar_ate(texto: &str) -> Option<String> {
    let idx = texto
        .find("Pagar este documento até")
        .or_else(|| texto.find("Pagar até"))?;
    let tail = &texto.as_bytes()[idx..];
    let janela = &tail[..tail.len().min(120)];
    for w in janela.windows(10) {
        let data_ascii = w.iter().enumerate().all(|(j, b)| {
            if j == 2 || j == 5 {
                *b == b'/'
            } else {
                b.is_ascii_digit()
            }
        });
        if data_ascii && let Ok(s) = std::str::from_utf8(w) {
            return parse_date_br(s);
        }
    }
    None
}

/// A linha digitável do DAS (arrecadação) são 4 blocos de 11 dígitos,
/// cada um com 1 dígito verificador: `858…3 934…8 630…9 394…3`. No texto
/// extraído do PDF eles aparecem como runs de dígitos separados por
/// espaço — procuramos a primeira sequência de runs com comprimentos
/// [11,1,11,1,11,1,11,1] e concatenamos (48 dígitos).
fn extrair_linha_digitavel(texto: &str) -> Option<String> {
    let mut runs: Vec<&str> = Vec::new();
    let mut start = None;
    for (i, c) in texto.char_indices() {
        if c.is_ascii_digit() {
            if start.is_none() {
                start = Some(i);
            }
        } else if let Some(s) = start.take() {
            runs.push(&texto[s..i]);
        }
    }
    if let Some(s) = start {
        runs.push(&texto[s..]);
    }

    const PADRAO: [usize; 8] = [11, 1, 11, 1, 11, 1, 11, 1];
    for janela in runs.windows(PADRAO.len()) {
        if janela
            .iter()
            .zip(PADRAO.iter())
            .all(|(run, len)| run.len() == *len)
        {
            return Some(janela.concat());
        }
    }
    None
}

/// "DD/MM/YYYY" → "YYYY-MM-DD"; "-"/vazio → None; outro formato → None.
fn parse_date_br(s: &str) -> Option<String> {
    let s = s.trim();
    let parts: Vec<&str> = s.split('/').collect();
    if parts.len() == 3
        && parts[0].len() == 2
        && parts[1].len() == 2
        && parts[2].len() == 4
        && parts.iter().all(|p| p.chars().all(|c| c.is_ascii_digit()))
    {
        Some(format!("{}-{}-{}", parts[2], parts[1], parts[0]))
    } else {
        None
    }
}

/// "R$ 93,44" → 9344; "R$ 1.234,56" → 123456; "-"/vazio → None.
fn parse_money_cents(s: &str) -> Option<i64> {
    let s = s.trim();
    if s.is_empty() || s == "-" {
        return None;
    }
    // Exige o padrão centavos (vírgula + 2 dígitos) pra não interpretar
    // texto arbitrário como dinheiro.
    let (inteiro, centavos) = s.rsplit_once(',')?;
    if centavos.len() != 2 || !centavos.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let inteiro: String = inteiro.chars().filter(|c| c.is_ascii_digit()).collect();
    if inteiro.is_empty() {
        return None;
    }
    let reais: i64 = inteiro.parse().ok()?;
    let cents: i64 = centavos.parse().ok()?;
    Some(reais * 100 + cents)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifica_situacoes() {
        assert_eq!(SituacaoDas::classificar("Devedor"), SituacaoDas::Devedor);
        assert_eq!(SituacaoDas::classificar("A Vencer"), SituacaoDas::AVencer);
        assert_eq!(
            SituacaoDas::classificar("Liquidado em 10/05/2026"),
            SituacaoDas::Liquidado
        );
        assert_eq!(
            SituacaoDas::classificar("Não Optante"),
            SituacaoDas::NaoOptante
        );
        assert_eq!(SituacaoDas::classificar("???"), SituacaoDas::Outra);
    }

    #[test]
    fn parse_money() {
        assert_eq!(parse_money_cents("R$ 93,44"), Some(9344));
        assert_eq!(parse_money_cents("R$ 86,05"), Some(8605));
        assert_eq!(parse_money_cents("R$ 1.234,56"), Some(123456));
        assert_eq!(parse_money_cents("-"), None);
        assert_eq!(parse_money_cents(""), None);
        assert_eq!(parse_money_cents("texto"), None);
    }

    #[test]
    fn parse_dates() {
        assert_eq!(parse_date_br("20/05/2026"), Some("2026-05-20".into()));
        assert_eq!(parse_date_br("-"), None);
        assert_eq!(parse_date_br(""), None);
    }

    #[test]
    fn linha_digitavel_do_texto() {
        let texto = "blah 12/06/2026 15:55:37\n85800000000 3 93440328261 8 63070826163 9 39468565126 3 AUTENTICAÇÃO MECÂNICA";
        assert_eq!(
            extrair_linha_digitavel(texto).as_deref(),
            Some("858000000003934403282618630708261639394685651263")
        );
        assert_eq!(extrair_linha_digitavel("nada aqui 123"), None);
    }

    #[test]
    fn pagar_ate_do_texto() {
        let texto = "Número do Documento\n07.08.26163.3946856-5\nPagar este documento até\n12/06/2026\nObservações";
        assert_eq!(extrair_pagar_ate(texto).as_deref(), Some("2026-06-12"));
        assert_eq!(extrair_pagar_ate("sem data"), None);
    }
}
