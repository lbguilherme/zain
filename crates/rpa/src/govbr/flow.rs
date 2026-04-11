//! Fluxo de login + verificação de perfil no gov.br.
//!
//! ## Estratégia
//!
//! 1. Se o caller passou uma `session` salva, tenta reutilizar: abre
//!    browser novo, injeta cookies + UA e navega para
//!    `contas.acesso.gov.br/contas/{cpf}`. Se ainda está logado → extrai
//!    perfil, re-captura a sessão e pronto.
//! 2. Caso contrário (sem sessão, ou sessão morta), faz login do zero com
//!    `cpf + password`, lidando com 2FA se aparecer.
//! 3. No 2FA marca "não solicitar verificação em duas etapas novamente
//!    neste navegador" — é o cookie persistente que fica salvo.
//! 4. Ao final do caminho feliz, re-captura a sessão.
//!
//! Este módulo é *puro*: não toca banco de dados. O caller é responsável
//! por persistir a sessão/perfil retornados onde quiser.
//!
//! ## Seletores
//!
//! Os seletores do SSO estão marcados com `TODO` — foram chutados a partir de
//! convenções comuns e devem ser confirmados/ajustados durante o teste manual.
//! Em erro, um `debug_dump` é salvo em `dumps/` pra facilitar a inspeção.

use std::time::Duration;

use chromium_driver::PageSession;
use serde::Serialize;
use thiserror::Error;

use super::Nivel;
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
pub struct CheckOutcome {
    pub profile: Profile,
    pub session: SavedSession,
    /// `true` se foi preciso logar do zero; `false` se a sessão salva foi
    /// reaproveitada.
    pub fresh_login: bool,
}

#[derive(Debug, Error)]
pub enum GovbrError {
    /// O gov.br recusou o par cpf+senha. A `String` carrega o texto
    /// exato do aviso amarelo exibido na página, que pode variar além
    /// do "usuário e/ou senha inválidos" — conta bloqueada, senha
    /// expirada, código interno ERLxxxxx, etc. Repassar a mensagem pro
    /// LLM permite reagir sem adivinhar.
    #[error("{0}")]
    InvalidCredentials(String),
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
/// Recebe as credenciais do gov.br e, opcionalmente, uma sessão previamente
/// salva. Tenta reaproveitar a sessão; se não der, loga do zero usando
/// `password` + `otp`. Devolve sempre uma sessão fresca e o perfil extraído.
///
/// Se o SSO do gov.br exibir um hCaptcha durante o fluxo, o login depende
/// da extensão NopeCHA estar carregada no browser (via
/// `NOPECHA_EXTENSION_PATH`) — o Rust apenas espera o iframe sumir.
pub async fn check_govbr_profile(
    cpf: &str,
    password: &str,
    otp: Option<&str>,
    saved: Option<&SavedSession>,
) -> Result<CheckOutcome, GovbrError> {
    // 1. Tentar reusar sessão salva.
    if let Some(saved) = saved {
        tracing::info!(cpf, "Tentando reusar sessão salva");
        match try_reuse_session(cpf, saved).await {
            Ok(Some((profile, session))) => {
                return Ok(CheckOutcome {
                    profile,
                    session,
                    fresh_login: false,
                });
            }
            Ok(None) => {
                tracing::info!(cpf, "Sessão salva inválida, vai logar do zero");
            }
            Err(e) => {
                tracing::warn!(cpf, "Erro reusando sessão: {e:#}");
            }
        }
    }

    // 2. Login do zero.
    let (profile, session) = do_fresh_login(cpf, password, otp).await?;
    Ok(CheckOutcome {
        profile,
        session,
        fresh_login: true,
    })
}

// ── Reuso de sessão ───────────────────────────────────────────────────────

/// `Ok(Some)` = logado via sessão salva.
/// `Ok(None)` = sessão rejeitada (caiu no login).
/// `Err`     = falha de browser/transporte.
async fn try_reuse_session(
    cpf: &str,
    saved: &SavedSession,
) -> anyhow::Result<Option<(Profile, SavedSession)>> {
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
            return Ok::<Option<(Profile, SavedSession)>, anyhow::Error>(None);
        }

        let profile = match extract_profile(&page, cpf).await {
            Ok(p) => p,
            Err(e) => {
                let _ = page.debug_dump("govbr-reuse-extract-fail").await;
                return Err(anyhow::anyhow!("extract_profile: {e}"));
            }
        };

        // Refresca a sessão (cookies podem ter girado).
        let fresh = session::capture(&browser).await?;
        Ok(Some((profile, fresh)))
    }
    .await;

    let _ = browser.close().await;
    let _ = process.wait().await;
    result
}

// ── Login do zero ─────────────────────────────────────────────────────────

async fn do_fresh_login(
    cpf: &str,
    password: &str,
    otp: Option<&str>,
) -> Result<(Profile, SavedSession), GovbrError> {
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
        // Senha errada no gov.br: a página volta pra tela de CPF e injeta
        // um `.br-message.warning` com o texto "Usuário e/ou senha
        // inválidos. (ERL0003900)". Detectamos isso e retornamos
        // `InvalidCredentials` — o caller (tool `auth_govbr`) sabe
        // traduzir pra mensagem ao cliente.
        let needs_2fa = loop {
            if dom.try_query_selector("input#otpInput").await?.is_some() {
                break true;
            }
            if dom.try_query_selector("input#accountId").await?.is_some()
                && dom
                    .try_query_selector(".br-message.warning")
                    .await?
                    .is_some()
            {
                let _ = page.debug_dump("govbr-login-invalid-credentials").await;
                let message = read_warning_message(&page).await;
                return Err(GovbrError::InvalidCredentials(message));
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

        // Captura a sessão final para o caller persistir.
        let fresh = session::capture(&browser).await?;

        Ok::<(Profile, SavedSession), GovbrError>((profile, fresh))
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

/// Lê o texto do banner de aviso amarelo que o SSO exibe quando o login
/// falha. O texto costuma ser algo como "Usuário e/ou senha inválidos.
/// (ERL0003900)" mas pode variar (conta bloqueada, senha expirada, ...).
/// Se a extração falhar, devolve um fallback genérico.
async fn read_warning_message(page: &PageSession) -> String {
    const FALLBACK: &str = "gov.br rejeitou o login (mensagem não capturada)";
    let js = r#"
        (() => {
            const el = document.querySelector('.br-message.warning');
            if (!el) return null;
            return (el.innerText || el.textContent || "").replace(/\s+/g, " ").trim();
        })()
    "#;
    match page.eval_value(js).await {
        Ok(v) => v
            .as_str()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| FALLBACK.to_string()),
        Err(_) => FALLBACK.to_string(),
    }
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
