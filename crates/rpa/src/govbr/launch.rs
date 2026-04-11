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

use chromium_driver::LaunchOptions;

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
