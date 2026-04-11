//! Fluxo de login + verificação de perfil no gov.br.
//!
//! ## Estratégia
//!
//! 1. Se existe `session` no banco, tenta reutilizar: abre browser novo,
//!    injeta cookies + UA e navega para `contas.acesso.gov.br/contas/{cpf}`.
//!    Se ainda está logado → extrai perfil, refresca sessão e pronto.
//! 2. Caso contrário (sem sessão, ou sessão morta), limpa a coluna e faz
//!    login do zero com `cpf + password`, lidando com 2FA se aparecer.
//! 3. No 2FA marca "não solicitar verificação em duas etapas novamente
//!    neste navegador" — é o cookie persistente que fica salvo.
//! 4. Ao final do caminho feliz, re-captura a sessão e grava no banco.
//!
//! ## Seletores
//!
//! Os seletores do SSO estão marcados com `TODO` — foram chutados a partir de
//! convenções comuns e devem ser confirmados/ajustados durante o teste manual.
//! Em erro, um `debug_dump` é salvo em `dumps/` pra facilitar a inspeção.

use std::time::Duration;

use chromium_driver::PageSession;
use deadpool_postgres::Pool;
use serde::Serialize;
use thiserror::Error;

use super::Nivel;
use super::db;
use super::launch;
use super::session::{self, SavedSession};
use crate::captcha;

const SSO_HOST: &str = "sso.acesso.gov.br";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
const POST_ACTION_SETTLE: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Serialize)]
pub struct Profile {
    pub cpf: String,
    pub nome: String,
    pub email: Option<String>,
    pub telefone: Option<String>,
    pub nivel: Option<Nivel>,
}

#[derive(Debug)]
pub enum CheckOutcome {
    /// Sessão salva ainda era válida.
    Reused(Profile),
    /// Login feito do zero.
    LoggedIn(Profile),
}

impl CheckOutcome {
    pub fn profile(&self) -> &Profile {
        match self {
            CheckOutcome::Reused(p) | CheckOutcome::LoggedIn(p) => p,
        }
    }
}

#[derive(Debug, Error)]
pub enum GovbrError {
    #[error("CPF não cadastrado em zain.govbr")]
    NotRegistered,
    #[error("credenciais inválidas")]
    InvalidCredentials,
    #[error("código OTP ausente ou inválido")]
    MissingOtp,
    #[error("falha ao extrair dados do perfil: {0}")]
    ProfileParse(String),
    #[error(transparent)]
    Cdp(#[from] chromium_driver::CdpError),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Ponto de entrada principal.
///
/// Abre o browser em `https://contas.acesso.gov.br/contas/{cpf}`. Se estiver
/// logado, extrai perfil. Caso contrário faz login com os dados do banco.
///
/// Se o SSO do gov.br exibir um hCaptcha durante o fluxo, o login depende
/// da extensão NopeCHA estar carregada no browser (via
/// `NOPECHA_EXTENSION_PATH`) — o Rust apenas espera o iframe sumir.
pub async fn check_govbr_profile(pool: &Pool, cpf: &str) -> Result<CheckOutcome, GovbrError> {
    let row = db::load(pool, cpf).await?;
    let Some(row) = row else {
        return Err(GovbrError::NotRegistered);
    };

    // 1. Tentar reusar sessão salva.
    if let Some(saved) = &row.session {
        tracing::info!(cpf, "Tentando reusar sessão salva");
        match try_reuse_session(pool, cpf, saved).await {
            Ok(Some(profile)) => return Ok(CheckOutcome::Reused(profile)),
            Ok(None) => {
                tracing::info!(cpf, "Sessão salva inválida, vai logar do zero");
                db::clear_session(pool, cpf).await?;
            }
            Err(e) => {
                tracing::warn!(cpf, "Erro reusando sessão: {e:#}");
                db::clear_session(pool, cpf).await?;
            }
        }
    }

    // 2. Login do zero.
    let profile = do_fresh_login(pool, cpf, &row.password, row.otp.as_deref()).await?;
    Ok(CheckOutcome::LoggedIn(profile))
}

// ── Reuso de sessão ───────────────────────────────────────────────────────

/// `Ok(Some)` = logado via sessão salva.
/// `Ok(None)` = sessão rejeitada (caiu no login).
/// `Err`     = falha de browser/transporte.
async fn try_reuse_session(
    pool: &Pool,
    cpf: &str,
    saved: &SavedSession,
) -> anyhow::Result<Option<Profile>> {
    let opts = launch::options_with_extensions().await?;
    let (mut process, browser) = chromium_driver::launch(opts).await?;

    let result = async {
        let page = browser.create_page("about:blank").await?.attach().await?;
        page.enable().await?;

        session::restore(&browser, &page, saved).await?;

        page.navigate(&profile_url(cpf)).await?;
        page.wait_for_load(DEFAULT_TIMEOUT).await.ok();
        tokio::time::sleep(POST_ACTION_SETTLE).await;

        let current = current_url(&page).await?;
        tracing::info!(cpf, url = %current, "Após restore + navigate");

        if current.contains(SSO_HOST) || current.contains("/login") {
            return Ok::<Option<Profile>, anyhow::Error>(None);
        }

        let profile = match extract_profile(&page, cpf).await {
            Ok(p) => p,
            Err(e) => {
                let _ = page.debug_dump("govbr-reuse-extract-fail").await;
                return Err(anyhow::anyhow!("extract_profile: {e}"));
            }
        };

        // Refresca a sessão (cookies podem ter girado) e persiste perfil.
        let fresh = session::capture(&browser).await?;
        db::save_session(pool, cpf, &fresh).await?;
        persist_profile(pool, &profile).await?;

        Ok(Some(profile))
    }
    .await;

    let _ = browser.close().await;
    let _ = process.wait().await;
    result
}

// ── Login do zero ─────────────────────────────────────────────────────────

async fn do_fresh_login(
    pool: &Pool,
    cpf: &str,
    password: &str,
    otp: Option<&str>,
) -> Result<Profile, GovbrError> {
    let opts = launch::options_with_extensions().await?;
    let (mut process, browser) = chromium_driver::launch(opts)
        .await
        .map_err(anyhow::Error::from)?;

    let result = async {
        let page = browser.create_page("about:blank").await?.attach().await?;
        page.enable().await?;

        // Vai direto pro perfil — o SSO redireciona pro login automaticamente.
        page.navigate(&profile_url(cpf)).await?;
        page.wait_for_load(DEFAULT_TIMEOUT).await.ok();
        tokio::time::sleep(POST_ACTION_SETTLE).await;

        // ── Passo 1: CPF ─────────────────────────────────────────────────
        let dom = page.dom().await?;
        let cpf_input = dom.wait_for("input#accountId", DEFAULT_TIMEOUT).await?;
        cpf_input.click().await?;
        cpf_input.type_text(cpf).await?;

        let cpf_btn = dom.query_selector("button#enter-account-id").await?;
        cpf_btn.click().await?;
        page.wait_for_load(DEFAULT_TIMEOUT).await.ok();
        dom.invalidate();

        // Pode aparecer hCaptcha entre o CPF e a senha. A resolução é
        // delegada à extensão NopeCHA carregada no browser — aqui só
        // aguardamos o iframe sumir.
        wait_for_hcaptcha(&page, "pós-cpf").await?;

        // ── Passo 2: senha ───────────────────────────────────────────────
        let pwd_input = dom.wait_for("input#password", DEFAULT_TIMEOUT).await?;
        pwd_input.click().await?;
        pwd_input.type_text(password).await?;

        let enter_btn = dom.query_selector("button#submit-button").await?;
        enter_btn.click().await?;
        page.wait_for_load(DEFAULT_TIMEOUT).await.ok();
        dom.invalidate();

        // E pode aparecer de novo depois da senha, antes do 2FA.
        wait_for_hcaptcha(&page, "pós-senha").await?;

        // Detectar erro de credencial: se continuamos na mesma página
        // de senha com uma mensagem de erro visível.
        if dom
            .try_query_selector(".alert-error, .error-message, .feedback-error")
            .await?
            .is_some()
            && dom.try_query_selector("input#password").await?.is_some()
        {
            let _ = page.debug_dump("govbr-login-invalid-credentials").await;
            return Err(GovbrError::InvalidCredentials);
        }

        // ── Passo 3: 2FA (opcional) ──────────────────────────────────────
        let needs_2fa = dom.try_query_selector("input#otpInput").await?.is_some();

        if needs_2fa {
            tracing::info!(cpf, "2FA solicitado");
            let Some(otp_code) = otp else {
                let _ = page.debug_dump("govbr-login-needs-otp").await;
                return Err(GovbrError::MissingOtp);
            };

            // Fecha o hintbox explicativo do checkbox — ele vem com um
            // overlay (`#gdd-overlay`) que intercepta cliques. Tenta o
            // botão de fechar e depois esconde o overlay via JS, caso
            // ainda persista.
            let _ = page
                .eval_value(
                    r#"(() => {
                        const close = document.querySelector('#gdd-close-hint');
                        if (close) close.click();
                        const overlay = document.querySelector('#gdd-overlay');
                        if (overlay) overlay.style.display = 'none';
                        const hint = document.querySelector('#gdd-hintbox');
                        if (hint) hint.style.display = 'none';
                        return true;
                    })()"#,
                )
                .await;
            tokio::time::sleep(Duration::from_millis(300)).await;

            // Marcar "Não solicitar verificação em duas etapas novamente
            // neste navegador" antes de digitar o código.
            let _ = page
                .eval_value(
                    r#"(() => {
                        const cb = document.querySelector('input#device');
                        if (cb && !cb.checked) cb.click();
                        return cb ? cb.checked : false;
                    })()"#,
                )
                .await;

            let otp_input = dom.wait_for("input#otpInput", DEFAULT_TIMEOUT).await?;
            otp_input.click().await?;
            otp_input.type_text(otp_code).await?;

            let ok_btn = dom.query_selector("button#enter-offline-2fa-code").await?;
            ok_btn.click().await?;
            page.wait_for_load(DEFAULT_TIMEOUT).await.ok();
            dom.invalidate();

            // O gov.br mantém a mesma URL e apenas injeta um
            // `.alert-danger` com a mensagem de código incorreto. Se
            // ainda existe o input de OTP ou o alerta de erro, o código
            // foi recusado.
            let otp_rejected = dom.try_query_selector(".alert-danger").await?.is_some()
                || dom.try_query_selector("input#otpInput").await?.is_some();
            if otp_rejected {
                let _ = page.debug_dump("govbr-login-otp-rejected").await;
                return Err(GovbrError::MissingOtp);
            }
        }

        // ── Passo 4: página de perfil ────────────────────────────────────
        let url_now = current_url(&page).await?;
        if !url_now.contains("contas.acesso.gov.br") {
            page.navigate(&profile_url(cpf)).await?;
            page.wait_for_load(DEFAULT_TIMEOUT).await.ok();
            tokio::time::sleep(POST_ACTION_SETTLE).await;
        }

        let profile = match extract_profile(&page, cpf).await {
            Ok(p) => p,
            Err(e) => {
                let _ = page.debug_dump("govbr-login-extract-fail").await;
                return Err(GovbrError::ProfileParse(e.to_string()));
            }
        };

        // Persistência final: cookies atualizados + perfil.
        let fresh = session::capture(&browser).await?;
        db::save_session(pool, cpf, &fresh).await?;
        persist_profile(pool, &profile).await?;

        Ok::<Profile, GovbrError>(profile)
    }
    .await;

    let _ = browser.close().await;
    let _ = process.wait().await;
    result
}

// ── Helpers ───────────────────────────────────────────────────────────────

async fn current_url(page: &PageSession) -> anyhow::Result<String> {
    let v = page.eval_value("location.href").await?;
    Ok(v.as_str().unwrap_or("").to_string())
}

fn profile_url(cpf: &str) -> String {
    format!("https://contas.acesso.gov.br/contas/{cpf}")
}

/// Aguarda o iframe de hCaptcha sumir, presumindo que a extensão NopeCHA
/// (carregada no browser via [`launch::options_with_extensions`]) vai
/// resolver o desafio. Se não havia desafio, volta em ~2s sem custo.
///
/// Falha o login se um desafio aparecer e não for resolvido dentro do
/// `SOLVE_TIMEOUT` — tipicamente porque a extensão não está instalada ou
/// ficou sem créditos.
async fn wait_for_hcaptcha(page: &PageSession, stage: &str) -> Result<(), GovbrError> {
    const DETECT_TIMEOUT: Duration = Duration::from_secs(2);
    const SOLVE_TIMEOUT: Duration = Duration::from_secs(120);

    match captcha::wait_until_gone(page, DETECT_TIMEOUT, SOLVE_TIMEOUT).await {
        Ok(false) => {}
        Ok(true) => {
            tracing::info!(stage, "hcaptcha resolvido pela extensão");
            tokio::time::sleep(POST_ACTION_SETTLE).await;
        }
        Err(e) => {
            let _ = page
                .debug_dump(&format!("govbr-hcaptcha-failed-{stage}"))
                .await;
            return Err(GovbrError::Other(anyhow::anyhow!(
                "hcaptcha ({stage}): {e:#}"
            )));
        }
    }
    Ok(())
}

/// Extrai nome, email, telefone e nível do perfil via JS.
///
/// Usa heurística de regex sobre `body.innerText` como fallback porque os
/// seletores específicos da página ainda não foram confirmados. TODO:
/// substituir por seletores diretos após inspecionar a página real.
async fn extract_profile(page: &PageSession, cpf: &str) -> anyhow::Result<Profile> {
    let js = r#"
        (() => {
            const body = document.body ? (document.body.innerText || "") : "";
            const pick = (re) => {
                const m = body.match(re);
                return m ? m[1].trim() : null;
            };
            return {
                nome: pick(/Nome(?:\s*completo)?[:\s]+([^\n]+)/i),
                email: pick(/E-?mail[:\s]+([^\n]+)/i),
                telefone: pick(/Telefone(?:\s*celular)?[:\s]+([^\n]+)/i),
                nivel: pick(/Selo[:\s]+([^\n]+)/i)
                    || pick(/Nível(?:\s*da conta)?[:\s]+([^\n]+)/i),
            };
        })()
    "#;

    let value = page.eval_value(js).await?;

    let nome = value
        .get("nome")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow::anyhow!("nome não encontrado"))?
        .to_string();

    let email = value
        .get("email")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string());
    let telefone = value
        .get("telefone")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string());
    let nivel = value
        .get("nivel")
        .and_then(|v| v.as_str())
        .and_then(|s| s.trim().parse::<Nivel>().ok());

    Ok(Profile {
        cpf: cpf.to_string(),
        nome,
        email,
        telefone,
        nivel,
    })
}

async fn persist_profile(pool: &Pool, profile: &Profile) -> anyhow::Result<()> {
    db::save_profile(
        pool,
        &profile.cpf,
        &profile.nome,
        profile.email.as_deref(),
        profile.telefone.as_deref(),
        profile.nivel,
    )
    .await
}
