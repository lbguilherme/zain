//! Após enviar o CPF, monitora a árvore de frames a cada 2s por ~100s,
//! logando iframes de hcaptcha e se `input#password` apareceu — pra confirmar
//! se o challenge é resolvido pela extensão ou fica preso.

use std::time::{Duration, Instant};

use rpa::govbr::launch;

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

    let mut args = std::env::args().skip(1);
    let cpf = args.next().expect("uso: govbr_watch <CPF>");

    let t0 = Instant::now();
    let opts = launch::options_with_extensions().await?;
    let (mut process, browser) = chromium_driver::launch(opts).await?;
    let page = browser.create_page("about:blank").await?.attach().await?;
    page.enable().await?;
    page.set_lifecycle_events_enabled(true).await.ok();

    page.navigate(&format!("https://contas.acesso.gov.br/contas/{cpf}"))
        .await?;
    page.wait_for_load(Duration::from_secs(15)).await.ok();
    page.wait_for_network_idle(Duration::from_secs(2))
        .await
        .ok();

    let dom = page.dom().await?;
    let cpf_input = dom
        .wait_for("input#accountId", Duration::from_secs(20))
        .await?;
    cpf_input.click().await?;
    cpf_input.type_text(&cpf).await?;
    dom.query_selector("button#enter-account-id")
        .await?
        .click()
        .await?;
    eprintln!(
        "[{:.1}s] CPF enviado; monitorando frames…",
        t0.elapsed().as_secs_f32()
    );
    dom.invalidate();

    let deadline = Instant::now() + Duration::from_secs(100);
    let mut seen_challenge = false;
    while Instant::now() < deadline {
        let el = t0.elapsed().as_secs_f32();
        let frames = page.get_frames().await.unwrap_or_default();
        let hcap: Vec<String> = frames
            .iter()
            .filter(|f| f.url.contains("hcaptcha.com"))
            .map(|f| {
                let kind = if f.url.contains("frame=challenge") {
                    "CHALLENGE"
                } else if f.url.contains("frame=checkbox") {
                    "checkbox"
                } else {
                    "outro"
                };
                format!("{kind}")
            })
            .collect();
        let has_pwd = dom
            .try_query_selector("input#password")
            .await
            .ok()
            .flatten()
            .is_some();
        let has_otp = dom
            .try_query_selector("input#otpInput")
            .await
            .ok()
            .flatten()
            .is_some();
        if hcap.iter().any(|k| k == "CHALLENGE") {
            seen_challenge = true;
        }
        eprintln!(
            "[{el:.1}s] hcaptcha=[{}] password={has_pwd} otp={has_otp}",
            hcap.join(", ")
        );
        if has_pwd || has_otp {
            eprintln!("[{el:.1}s] AVANÇOU (password/otp presente) — captcha resolvido");
            break;
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    eprintln!(
        "[{:.1}s] fim. viu_challenge={seen_challenge}",
        t0.elapsed().as_secs_f32()
    );
    let _ = page
        .debug_dump_in(std::path::Path::new("/tmp/govbr-dumps"), "watch-final")
        .await;
    let _ = browser.close().await;
    let _ = process.wait().await;
    Ok(())
}
