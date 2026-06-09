//! Persistência da sessão do navegador para pular 2FA em logins futuros.
//!
//! Estratégia: salvar só o necessário no banco (cookies de `*.gov.br` + UA).
//! No próximo acesso, restauramos antes de navegar — o cookie persistente
//! marcado como "navegador confiável" faz o SSO pular a verificação em duas
//! etapas. Deliberadamente *não* salvamos o `userDataDir` inteiro.

use chromium_driver::{Browser, Cookie, PageSession};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedSession {
    /// User-Agent usado no momento do login. Precisa ser consistente entre
    /// save/restore — o SSO pode amarrar o token de confiança ao UA.
    pub user_agent: String,
    /// Cookies de `*.gov.br`. O [`Cookie`] preserva todos os campos do CDP
    /// (via `flatten`), então persiste no banco com o mesmo JSON dos cookies
    /// crus e volta inteiro pro `set_cookies` sem perder atributos.
    pub cookies: Vec<Cookie>,
}

/// Dumpa todos os cookies de `*.gov.br` e o User-Agent atual do browser.
pub async fn capture(browser: &Browser) -> anyhow::Result<SavedSession> {
    let version = browser.get_version().await?;
    let user_agent = version.user_agent;

    let cookies: Vec<Cookie> = browser
        .get_cookies()
        .await?
        .into_iter()
        .filter(|c| c.domain.trim_start_matches('.').ends_with("gov.br"))
        .collect();

    tracing::info!(count = cookies.len(), "Cookies gov.br capturados");

    Ok(SavedSession {
        user_agent,
        cookies,
    })
}

/// Restaura a sessão salva: força o User-Agent na página e injeta os cookies
/// no contexto do browser. Deve ser chamada ANTES de navegar para gov.br.
pub async fn restore(
    browser: &Browser,
    page: &PageSession,
    saved: &SavedSession,
) -> anyhow::Result<()> {
    // UA — escopo de página (aplica-se a requisições desta target).
    page.set_user_agent(&saved.user_agent, None).await?;

    // Cookies — escopo de browser.
    if !saved.cookies.is_empty() {
        browser.set_cookies(saved.cookies.clone()).await?;
    }

    tracing::info!(
        cookies = saved.cookies.len(),
        "Sessão restaurada (UA + cookies)"
    );
    Ok(())
}
