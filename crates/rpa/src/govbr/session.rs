//! Persistência da sessão do navegador para pular 2FA em logins futuros.
//!
//! Estratégia: salvar só o necessário no banco (cookies de `*.gov.br` + UA).
//! No próximo acesso, restauramos antes de navegar — o cookie persistente
//! marcado como "navegador confiável" faz o SSO pular a verificação em duas
//! etapas. Deliberadamente *não* salvamos o `userDataDir` inteiro.

use chromium_driver::{Browser, PageSession};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedSession {
    /// User-Agent usado no momento do login. Precisa ser consistente entre
    /// save/restore — o SSO pode amarrar o token de confiança ao UA.
    pub user_agent: String,
    /// Cookies crus do CDP `Storage.getCookies`, filtrados para `*.gov.br`.
    /// Repassamos o objeto inteiro pro `Storage.setCookies` sem converter.
    pub cookies: Vec<Value>,
}

/// Dumpa todos os cookies de `*.gov.br` e o User-Agent atual do browser.
pub async fn capture(browser: &Browser) -> anyhow::Result<SavedSession> {
    let version = browser.get_version().await?;
    let user_agent = version.user_agent;

    let resp: Value = browser.cdp().call("Storage.getCookies", &json!({})).await?;

    let all = resp
        .get("cookies")
        .and_then(|c| c.as_array())
        .cloned()
        .unwrap_or_default();

    let cookies: Vec<Value> = all
        .into_iter()
        .filter(|c| {
            c.get("domain")
                .and_then(|d| d.as_str())
                .map(|d| d.trim_start_matches('.').ends_with("gov.br"))
                .unwrap_or(false)
        })
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
    // UA — scopo de página (aplica-se a requisições desta target).
    page.cdp()
        .call_no_response(
            "Emulation.setUserAgentOverride",
            &json!({ "userAgent": saved.user_agent }),
        )
        .await?;

    // Cookies — escopo de browser.
    if !saved.cookies.is_empty() {
        browser
            .cdp()
            .call_no_response("Storage.setCookies", &json!({ "cookies": saved.cookies }))
            .await?;
    }

    tracing::info!(
        cookies = saved.cookies.len(),
        "Sessão restaurada (UA + cookies)"
    );
    Ok(())
}
