//! Verifica o caminho de injeção da key do NopeCHA: sobe o browser com a
//! extensão, roda `launch::configure_nopecha` (lê `NOPECHA_KEY`) e depois
//! abre o popup da extensão pra inspecionar o estado configurado.
//!
//! Uso: `NOPECHA_KEY=<key> nopecha_setup`

use std::time::Duration;

use rpa::govbr::launch;

const EXT_ID: &str = "dknlfmjaanfblgfdfebhijalfmhmjjjo";

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        if std::env::var("RUST_LOG").is_err() {
            std::env::set_var("RUST_LOG", "rpa=info,chromium_driver=warn");
        }
    }
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let key_set = std::env::var("NOPECHA_KEY")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .is_some();
    eprintln!("[setup] NOPECHA_KEY presente? {key_set}");

    let opts = launch::options_with_extensions().await?;
    let (mut process, browser) = chromium_driver::launch(opts).await?;
    eprintln!("[setup] browser ok; rodando configure_nopecha…");

    launch::configure_nopecha(&browser).await?;
    eprintln!("[setup] configure_nopecha retornou");

    // Lê de volta as settings persistidas pela extensão via a página de setup
    // sem hash (ela não reescreve nada) — na verdade inspecionamos o popup,
    // que renderiza a key/plano configurados.
    let page = browser
        .create_page(&format!("chrome-extension://{EXT_ID}/popup.html"))
        .await?
        .attach()
        .await?;
    page.enable().await?;
    page.wait_for_load(Duration::from_secs(15)).await.ok();
    tokio::time::sleep(Duration::from_secs(2)).await;

    let body = page
        .eval_value("document.body ? document.body.innerText : ''")
        .await
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default();
    eprintln!("[setup] popup innerText:\n----\n{}\n----", body.trim());

    let _ = page
        .debug_dump_in(std::path::Path::new("/tmp/govbr-dumps"), "nopecha-popup")
        .await;

    let _ = browser.close().await;
    let _ = process.wait().await;
    Ok(())
}
