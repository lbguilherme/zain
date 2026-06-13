//! RPA da DASN-SIMEI — Declaração Anual do Simples Nacional do MEI.
//!
//! O MEI precisa entregar, todo ano, a DASN-SIMEI declarando a receita
//! bruta do ano-calendário anterior (prazo: 31/05). Este módulo lê o
//! STATUS por ano no portal `dasnsimei.app` — quais anos já foram
//! entregues e quais não. (O preenchimento/transmissão é uma fase
//! futura; ver [`consultar_dasn`].)
//!
//! Acesso, igual ao PGMEI/DAS: **público por CNPJ** (não exige gov.br),
//! com **hCaptcha invisível** no submit (resolvido pela extensão NopeCHA).
//!
//! Como o status é lido: depois da identificação, o wizard "Iniciar"
//! (Declarar/Retificar) monta um radio por ano-calendário, e cada radio
//! carrega `data-tipo-declaracao` — **`Original`** (nunca entregue) ou
//! **`Retificadora`** (já existe declaração) — além de
//! `data-situacao-especial-*`. Lemos todos de uma vez num único `eval`,
//! sem selecionar nada.
//!
//! CUIDADO (semântica de atraso): o portal lista uma janela fixa de ~5
//! anos, INDEPENDENTE de quando o CNPJ virou MEI. Então `Original` NÃO
//! quer dizer "em atraso" — pode ser ano anterior à vigência do MEI
//! (confirmado em campo: MEIs que optaram em 2024 mostram 2021–2023 como
//! `Original`). Quem cruza com a vigência e decide "em atraso" é o caller
//! (`mcp`), não este módulo.

use std::time::Duration;

use chromium_driver::PageSession;
use serde::{Deserialize, Serialize};

use crate::govbr::launch;
use crate::sanity;

const IDENTIFICACAO_URL: &str = "https://www8.receita.fazenda.gov.br/SimplesNacional/Aplicacoes/ATSPO/dasnsimei.app/identificacao";

const TIMEOUT: Duration = Duration::from_secs(30);
/// O hCaptcha invisível roda entre "Continuar" e o redirect pra Home.
const CAPTCHA_TIMEOUT: Duration = Duration::from_secs(90);

/// Status da DASN de um ano-calendário, lido do wizard "Iniciar". São
/// todos os dados que o portal público (acesso por CNPJ) expõe por ano —
/// valor declarado/recibo/data de transmissão exigiriam login gov.br.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DasnAno {
    pub ano: i32,
    /// `true` quando já existe declaração entregue (tipo `Retificadora`).
    pub entregue: bool,
    /// Texto cru do `data-tipo-declaracao` (`Original` | `Retificadora`).
    pub tipo: String,
    /// `data-situacao-especial-tipo` (baixa/extinção); `None` se `-`.
    pub situacao_especial: Option<String>,
    /// `data-situacao-especial-eventobaixa`; `None` se `-`.
    pub situacao_especial_evento: Option<String>,
}

/// Lê o status da DASN-SIMEI de um CNPJ — todos os anos que o portal
/// oferece, com a marca de entregue/não-entregue.
pub async fn consultar_dasn(cnpj: &str) -> anyhow::Result<Vec<DasnAno>> {
    let digits: String = cnpj.chars().filter(|c| c.is_ascii_digit()).collect();
    anyhow::ensure!(
        digits.len() == 14,
        "CNPJ deve ter 14 dígitos, recebido {}",
        digits.len()
    );

    let opts = launch::options_with_extensions().await?;
    let (mut process, browser) = chromium_driver::launch(opts).await?;

    let result = async {
        launch::configure_nopecha(&browser).await.ok();
        let page = browser.create_page("about:blank").await?.attach().await?;
        page.enable().await?;
        identificar(&page, &digits).await?;
        extrair_status(&page).await
    }
    .await;

    let _ = browser.close().await;
    let _ = process.wait().await;
    result
}

/// Identificação por CNPJ: digita (máscara jQuery — teclas reais + retry),
/// clica "Continuar" e espera sair da tela (hCaptcha invisível roda aqui).
/// Mesmo fluxo do PGMEI (`pgmei::identificar`); replicado porque os portais
/// são apps independentes — mantê-los desacoplados evita que um quebre o
/// outro quando o layout de um muda.
async fn identificar(page: &PageSession, cnpj_digits: &str) -> anyhow::Result<()> {
    page.navigate(IDENTIFICACAO_URL).await?;
    page.wait_for_load(TIMEOUT).await.ok();

    // A DASN-SIMEI (Angular + gov.br DS) usa `#identificacao-cnpj` e
    // `#identificacao-continuar` (com o hCaptcha amarrado no botão) — não os
    // ids `#cnpj`/`#continuar` do PGMEI clássico.
    const INPUT: &str = "#identificacao-cnpj";
    const BOTAO: &str = "#identificacao-continuar";

    let dom = page.dom().await?;
    sanity::wait_for(page, dom, "dasn: identificação", INPUT, TIMEOUT).await?;

    let mut ultimo_valor = String::new();
    let mut registrou = false;
    for tentativa in 1..=4 {
        tokio::time::sleep(Duration::from_millis(200 * tentativa)).await;
        page.eval_value(&format!(
            r#"(() => {{
                const el = document.querySelector('{INPUT}');
                el.focus();
                const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set;
                setter.call(el, '');
                el.dispatchEvent(new Event('input', {{ bubbles: true }}));
                return 'ok';
            }})()"#
        ))
        .await?;
        let el = dom.query_selector(INPUT).await?;
        el.type_text(cnpj_digits).await?;

        ultimo_valor = page
            .eval_value(&format!("document.querySelector('{INPUT}').value"))
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
        tracing::warn!(tentativa, campo = %ultimo_valor, "dasn: máscara do CNPJ não bateu, retentando");
    }
    if !registrou {
        return Err(sanity::fail(
            page,
            "dasn: identificação",
            &format!(
                "máscara não registrou o CNPJ após 4 tentativas (campo ficou com {ultimo_valor:?})"
            ),
        )
        .await);
    }

    page.eval_value(&format!(
        r#"(() => {{
            const b = document.querySelector('{BOTAO}');
            if (!b) return 'not_found';
            b.click();
            return 'ok';
        }})()"#
    ))
    .await?
    .as_str()
    .filter(|s| *s == "ok")
    .ok_or_else(|| anyhow::anyhow!("botão {BOTAO} não encontrado na identificação"))?;

    let deadline = tokio::time::Instant::now() + CAPTCHA_TIMEOUT;
    loop {
        tokio::time::sleep(Duration::from_secs(2)).await;
        let url = page
            .eval_value("location.href")
            .await?
            .as_str()
            .unwrap_or("")
            .to_string();
        if !url.to_lowercase().contains("identificacao") {
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
                "dasn: identificação",
                &format!("portal rejeitou: {t}"),
            )
            .await);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(sanity::fail(
                page,
                "dasn: identificação",
                "timeout aguardando sair da identificação (hCaptcha não resolvido?)",
            )
            .await);
        }
    }
}

/// Lê os radios de ano do wizard "Iniciar" com seus data-attributes.
async fn extrair_status(page: &PageSession) -> anyhow::Result<Vec<DasnAno>> {
    // O wizard pode levar um instante pra hidratar os radios.
    let dom = page.dom().await?;
    sanity::wait_for(
        page,
        dom,
        "dasn: wizard iniciar",
        "input[name=opcao]",
        TIMEOUT,
    )
    .await?;

    let v = page
        .eval_value(
            r#"(() => [...document.querySelectorAll('input[name=opcao]')].map(r => ({
                ano: r.value,
                tipo: r.getAttribute('data-tipo-declaracao') || '',
                situacao_especial: r.getAttribute('data-situacao-especial-tipo') || '',
                situacao_especial_evento: r.getAttribute('data-situacao-especial-eventobaixa') || '',
            })))()"#,
        )
        .await?;

    let Some(rows) = v.as_array() else {
        return Err(sanity::fail(
            page,
            "dasn: extrair status",
            "eval não devolveu array de anos",
        )
        .await);
    };

    let mut anos = Vec::with_capacity(rows.len());
    for row in rows {
        let ano: i32 = match row
            .get("ano")
            .and_then(|a| a.as_str())
            .and_then(|s| s.parse().ok())
        {
            Some(a) => a,
            None => continue,
        };
        let tipo = row
            .get("tipo")
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string();
        let attr = |k: &str| {
            row.get(k)
                .and_then(|s| s.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty() && *s != "-")
                .map(str::to_string)
        };
        anos.push(DasnAno {
            ano,
            entregue: tipo.eq_ignore_ascii_case("Retificadora"),
            tipo,
            situacao_especial: attr("situacao_especial"),
            situacao_especial_evento: attr("situacao_especial_evento"),
        });
    }
    if anos.is_empty() {
        return Err(sanity::fail(
            page,
            "dasn: extrair status",
            "nenhum ano-calendário no wizard",
        )
        .await);
    }
    anos.sort_by(|a, b| b.ano.cmp(&a.ano));
    Ok(anos)
}
