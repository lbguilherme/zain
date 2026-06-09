//! Sanity checks reusáveis pros fluxos RPA.
//!
//! Os portais do governo (gov.br, Receita) são SPAs Angular que mudam sem
//! aviso: um elemento some, a rota redireciona pro SSO, um banner de erro
//! aparece. Quando isso acontece no meio de um fluxo, o erro cru do
//! chromium-driver é inútil — `dom.wait_for(...)` num timeout devolve só
//! "request timed out after 30s", sem dizer QUE etapa, em QUE URL, esperando
//! QUE seletor. Foi assim que a mudança do portal do CCMEI (que passou a
//! exigir login) ficou escondida atrás de um timeout genérico.
//!
//! Este módulo dá um padrão único: a cada etapa, garanta que a página está
//! no estado esperado e, se fugir, registre logs ÚTEIS (etapa, URL, título,
//! seletor) e um `debug_dump` (HTML/screenshot em `dumps/`), e devolva um
//! erro descritivo. Use:
//!
//! - [`wait_for`] no lugar de `dom.wait_for(sel, t).await?`.
//! - [`tick`] dentro de loops de polling (garante teto de tempo + log).
//! - [`expect_url_contains`] pra detectar navegação/redirect inesperado.
//! - [`checkpoint`] pra um sanity leve no começo de cada etapa.
//! - [`fail`] pra abortar com contexto num ponto arbitrário (extração null…).
//!
//! Todos devolvem/produzem `anyhow::Error`, que os erros de domínio dos
//! fluxos absorvem via `#[from]` (ex: `InscricaoMeiError::Other`).

use std::time::Duration;

use chromium_driver::PageSession;
use chromium_driver::dom::{Dom, Element};

/// URL + título atuais da página, pra contexto de log. Best-effort: se o
/// eval falhar (página caindo, browser instável), devolve strings vazias em
/// vez de propagar — o objetivo é enriquecer o erro, não criar outro.
pub async fn page_where(page: &PageSession) -> (String, String) {
    let url = page
        .eval_value("location.href")
        .await
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default();
    let title = page
        .eval_value("document.title")
        .await
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default();
    (url, title)
}

/// Espera `selector` aparecer. Em falha (timeout ou erro do CDP), loga
/// contexto (etapa, URL, título, seletor) + `debug_dump` e devolve um erro
/// descritivo com a etapa e a URL atual — em vez do "timeout after 30s" cru.
pub async fn wait_for(
    page: &PageSession,
    dom: &Dom,
    etapa: &str,
    selector: &str,
    timeout: Duration,
) -> anyhow::Result<Element> {
    match dom.wait_for(selector, timeout).await {
        Ok(el) => Ok(el),
        Err(e) => Err(report(
            page,
            etapa,
            &format!("elemento esperado não apareceu ({selector})"),
            Some(&e.to_string()),
        )
        .await),
    }
}

/// Avança um loop de polling: dorme `interval` e, se o `deadline` já passou,
/// loga contexto + `debug_dump` e devolve `Err` (pra abortar o loop via `?`).
/// Garante que loops de polling tenham SEMPRE um teto de tempo e um log útil
/// no estouro — em vez de rodar pra sempre ou estourar sem contexto.
pub async fn tick(
    page: &PageSession,
    etapa: &str,
    deadline: tokio::time::Instant,
    interval: Duration,
) -> anyhow::Result<()> {
    if tokio::time::Instant::now() >= deadline {
        return Err(report(
            page,
            etapa,
            "timeout aguardando a página chegar no estado esperado",
            None,
        )
        .await);
    }
    tokio::time::sleep(interval).await;
    Ok(())
}

/// Valida que a URL atual contém `esperado`. Se não, trata como navegação
/// inesperada: loga + `debug_dump` + erro. Use depois de cliques/navegações
/// que deviam levar a uma rota conhecida.
pub async fn expect_url_contains(
    page: &PageSession,
    etapa: &str,
    esperado: &str,
) -> anyhow::Result<()> {
    let (url, title) = page_where(page).await;
    if url.contains(esperado) {
        tracing::debug!(etapa, %url, "rpa sanity: url ok");
        Ok(())
    } else {
        tracing::error!(etapa, %url, title, esperado, "rpa sanity: navegação inesperada (URL fora do esperado)");
        let _ = page.debug_dump(&dump_name(etapa)).await;
        anyhow::bail!(
            "[{etapa}] navegação inesperada: esperava URL contendo {esperado:?}, mas está em {url}"
        )
    }
}

/// Sanity leve no início de uma etapa: registra onde a página está (URL +
/// título) em nível debug, criando um rastro do caminho percorrido.
pub async fn checkpoint(page: &PageSession, etapa: &str) {
    let (url, title) = page_where(page).await;
    tracing::debug!(etapa, %url, title, "rpa sanity: checkpoint");
}

/// Aborta a etapa atual com contexto: loga (etapa + URL + título + msg) em
/// nível error + `debug_dump` e devolve um `anyhow::Error` pronto pra `?`.
/// Use quando um `eval`/extração devolve algo fora do esperado.
pub async fn fail(page: &PageSession, etapa: &str, msg: &str) -> anyhow::Error {
    report(page, etapa, msg, None).await
}

/// Núcleo: loga contexto + dump e constrói o erro com etapa + URL.
async fn report(page: &PageSession, etapa: &str, msg: &str, causa: Option<&str>) -> anyhow::Error {
    let (url, title) = page_where(page).await;
    tracing::error!(etapa, %url, title, causa = causa.unwrap_or(""), "rpa sanity: {msg}");
    let _ = page.debug_dump(&dump_name(etapa)).await;
    match causa {
        Some(c) => anyhow::anyhow!("[{etapa}] {msg}; url atual: {url}; causa: {c}"),
        None => anyhow::anyhow!("[{etapa}] {msg}; url atual: {url}"),
    }
}

/// Sanitiza a etapa pra um nome de arquivo de dump estável (sem timestamp:
/// a última falha de cada etapa fica em `dumps/rpa-<etapa>.{html,png}`).
fn dump_name(etapa: &str) -> String {
    let slug: String = etapa
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    format!("rpa-{}", slug.trim_matches('-'))
}
