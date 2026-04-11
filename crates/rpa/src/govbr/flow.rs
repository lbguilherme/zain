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
        dom.invalidate();

        // E pode aparecer de novo depois da senha, antes do 2FA.
        wait_for_hcaptcha(&page, "pós-senha").await?;

        // SSO é SPA — `wait_for_load` não é confiável. Pollamos o DOM
        // esperando a próxima tela aparecer (2FA, erro de credencial, ou
        // redirect direto pro perfil quando o 2FA está dispensado).
        let post_password_deadline = tokio::time::Instant::now() + Duration::from_secs(30);
        let needs_2fa = loop {
            if dom.try_query_selector("input#otpInput").await?.is_some() {
                break true;
            }
            if dom
                .try_query_selector(".alert-error, .error-message, .feedback-error")
                .await?
                .is_some()
                && dom.try_query_selector("input#password").await?.is_some()
            {
                let _ = page.debug_dump("govbr-login-invalid-credentials").await;
                return Err(GovbrError::InvalidCredentials);
            }
            if current_url(&page).await?.contains("contas.acesso.gov.br") {
                break false;
            }
            if tokio::time::Instant::now() >= post_password_deadline {
                let _ = page.debug_dump("govbr-login-post-password-timeout").await;
                return Err(GovbrError::Other(anyhow::anyhow!(
                    "timeout aguardando próxima etapa após senha"
                )));
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        };

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
            dom.invalidate();

            // O gov.br mantém a mesma URL e apenas injeta um
            // `.alert-danger` com a mensagem de código incorreto. Pollamos
            // até um dos estados finais aparecer: erro de OTP, ou URL do
            // perfil.
            let post_otp_deadline = tokio::time::Instant::now() + Duration::from_secs(30);
            loop {
                if dom.try_query_selector(".alert-danger").await?.is_some() {
                    let _ = page.debug_dump("govbr-login-otp-rejected").await;
                    return Err(GovbrError::MissingOtp);
                }
                if current_url(&page).await?.contains("contas.acesso.gov.br") {
                    break;
                }
                if tokio::time::Instant::now() >= post_otp_deadline {
                    let _ = page.debug_dump("govbr-login-post-otp-timeout").await;
                    return Err(GovbrError::Other(anyhow::anyhow!(
                        "timeout aguardando próxima etapa após OTP"
                    )));
                }
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        }

        // ── Passo 4: página de perfil ────────────────────────────────────
        let url_now = current_url(&page).await?;
        if !url_now.contains("contas.acesso.gov.br") {
            page.navigate(&profile_url(cpf)).await?;
            page.wait_for_load(DEFAULT_TIMEOUT).await.ok();
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
/// A página renderiza cada dado num `StaticField` (label + `<p>`), então
/// procuramos pelo texto do `<label>` e pegamos o `<p>` irmão. O nível é
/// um caso especial: o texto "bronze/prata/ouro" é injetado via
/// `::before`/`::after` em cima de um `data-level` numérico, então o
/// `innerText` fica vazio — lemos o atributo direto.
///
/// A página é renderizada dinamicamente — após o submit do OTP o
/// `wait_for_load` volta antes do conteúdo ser populado. Por isso aqui
/// pollamos a extração até o `nome` aparecer (ou dar timeout).
async fn extract_profile(page: &PageSession, cpf: &str) -> anyhow::Result<Profile> {
    const EXTRACT_TIMEOUT: Duration = Duration::from_secs(30);
    const POLL_INTERVAL: Duration = Duration::from_millis(500);

    let js = r#"
        (() => {
            const norm = (s) => (s || "").replace(/\s+/g, " ").trim();

            // Pega o <p> de um StaticField cujo <label> bate (case-insensitive,
            // sem acentos) com um dos candidatos.
            const fieldByLabel = (...labels) => {
                const wanted = labels.map((l) =>
                    l.normalize("NFD").replace(/[\u0300-\u036f]/g, "").toLowerCase()
                );
                const fields = document.querySelectorAll(
                    ".govbr-preact-tools-components-StaticField"
                );
                for (const f of fields) {
                    const label = f.querySelector("label");
                    if (!label) continue;
                    const text = norm(label.textContent)
                        .normalize("NFD")
                        .replace(/[\u0300-\u036f]/g, "")
                        .toLowerCase();
                    if (wanted.includes(text)) {
                        const p = f.querySelector("p");
                        return p ? norm(p.textContent) : null;
                    }
                }
                return null;
            };

            const reliability = document.querySelector(
                ".govbr-preact-tools-components-reliability"
            );
            const levelMap = { "1": "bronze", "2": "prata", "3": "ouro" };
            const nivel = reliability
                ? (levelMap[reliability.getAttribute("data-level")] || null)
                : null;

            return {
                nome: fieldByLabel("nome", "nome completo"),
                email: fieldByLabel("e-mail", "email"),
                telefone: fieldByLabel("telefone", "telefone celular", "celular"),
                nivel,
            };
        })()
    "#;

    let pick_string = |value: &serde_json::Value, key: &str| -> Option<String> {
        value
            .get(key)
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };

    let deadline = tokio::time::Instant::now() + EXTRACT_TIMEOUT;
    loop {
        let value = page.eval_value(js).await?;

        if let Some(nome) = pick_string(&value, "nome") {
            let email = pick_string(&value, "email");
            let telefone = pick_string(&value, "telefone");
            let nivel = pick_string(&value, "nivel").and_then(|s| s.parse::<Nivel>().ok());

            return Ok(Profile {
                cpf: cpf.to_string(),
                nome,
                email,
                telefone,
                nivel,
            });
        }

        if tokio::time::Instant::now() >= deadline {
            anyhow::bail!("nome não encontrado");
        }

        tokio::time::sleep(POLL_INTERVAL).await;
    }
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
