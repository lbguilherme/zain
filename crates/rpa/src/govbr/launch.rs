//! Construção de [`LaunchOptions`] com extensões do browser.
//!
//! Para driblar o hCaptcha do gov.br delegamos a resolução à extensão
//! **NopeCHA** instalada no Chromium. O ID da extensão é lido de
//! `NOPECHA_EXTENSION_ID` (ou usa o oficial publicado na Chrome Web Store
//! como default). O CRX é baixado on-demand e cacheado em
//! `$TMPDIR/zain-chrome-extensions/{id}/`.
//!
//! Para desabilitar a extensão (login sem captcha solver) basta exportar
//! `NOPECHA_EXTENSION_ID=` (vazio).
//!
//! ## API key (plano pago)
//!
//! O tier grátis do NopeCHA é limitado por IP (100 solves/dia, exclui IPs
//! não-residenciais, detecção de abuso → "Banned IP"). Com um plano pago,
//! a key autentica por conta própria e contorna esse gating. Como o browser
//! sobe com `user-data-dir` descartável a cada launch, o `chrome.storage` da
//! extensão é zerado toda vez — então [`configure_nopecha`] re-injeta a key
//! a cada launch, navegando à página oficial de setup
//! (`https://nopecha.com/setup#<key>`), cujo content-script persiste a config
//! na extensão. Set via env `NOPECHA_KEY` (vazio/ausente = tier grátis).

use std::time::Duration;

use chromium_driver::{Browser, LaunchOptions};

use super::extension;

/// ID oficial da extensão NopeCHA Captcha Solver na Chrome Web Store.
/// <https://chromewebstore.google.com/detail/nopecha-captcha-solver/dknlfmjaanfblgfdfebhijalfmhmjjjo>
const DEFAULT_NOPECHA_ID: &str = "dknlfmjaanfblgfdfebhijalfmhmjjjo";

/// Constrói [`LaunchOptions`] já com flags de extensão. Baixa e descomprime
/// o CRX se ainda não estiver em cache.
pub async fn options_with_extensions() -> anyhow::Result<LaunchOptions> {
    let mut opts = LaunchOptions::default();

    let id = std::env::var("NOPECHA_EXTENSION_ID")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_NOPECHA_ID.to_string());

    let dir = extension::ensure_unpacked(&id).await?;
    let dir_str = dir
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("path da extensão tem bytes não-UTF8: {}", dir.display()))?;

    tracing::info!(extension = %id, dir = dir_str, "carregando extensão");

    // --load-extension aceita lista separada por vírgula. Aqui só temos uma;
    // se no futuro quisermos mais, basta acumular antes de juntar.
    opts.extra_args.push(format!("--load-extension={dir_str}"));
    // Bloqueia qualquer outra extensão — garante ambiente previsível.
    opts.extra_args
        .push(format!("--disable-extensions-except={dir_str}"));

    Ok(opts)
}

/// Injeta a API key do NopeCHA na extensão, se `NOPECHA_KEY` estiver setada.
///
/// No-op silencioso quando a env não existe (usa o tier grátis). Deve ser
/// chamada logo após o launch e ANTES de navegar pro gov.br, pra que a key
/// já esteja no `chrome.storage` quando o primeiro captcha aparecer.
///
/// Best-effort: se a página de setup não confirmar, loga um aviso e segue —
/// um captcha não resolvido depois falha o login com erro claro de qualquer
/// forma. Abre numa aba própria que fica em background (o fluxo gov.br cria
/// a sua própria página).
pub async fn configure_nopecha(browser: &Browser) -> anyhow::Result<()> {
    let Some(key) = std::env::var("NOPECHA_KEY")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
    else {
        return Ok(());
    };

    // A página de setup lê o que vem depois do `#`: um valor "pelado" é
    // interpretado como `key=<valor>`. Daí o content-script chama
    // `settings::update`, que persiste no storage da extensão.
    let url = format!("https://nopecha.com/setup#{key}");
    let page = browser.create_page("about:blank").await?.attach().await?;
    page.enable().await?;
    page.navigate(&url).await?;
    page.wait_for_load(Duration::from_secs(20)).await.ok();

    // O setup.js renderiza uma <table> "Imported settings" quando aplica a
    // config. Esperamos por ela e damos um respiro pro service worker gravar.
    let dom = page.dom().await?;
    match dom.wait_for("table", Duration::from_secs(10)).await {
        Ok(_) => {
            tokio::time::sleep(Duration::from_millis(800)).await;
            tracing::info!("NopeCHA: API key configurada na extensão");
        }
        Err(e) => {
            tracing::warn!(error = %e, "NopeCHA: página de setup não confirmou import da key");
        }
    }
    Ok(())
}
