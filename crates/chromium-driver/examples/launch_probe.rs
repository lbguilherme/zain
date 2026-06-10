//! Probe de diagnóstico: replica os primeiros passos do fluxo gov.br
//! (launch com extensão → create_page → attach → enable → UA → navigate)
//! logando cada etapa com tempo, pra apontar onde o timeout acontece.
//!
//! Uso: `launch_probe [dir-da-extensão]`

use std::time::{Duration, Instant};

use chromium_driver::LaunchOptions;

fn elapsed(t0: Instant) -> String {
    format!("{:.1}s", t0.elapsed().as_secs_f32())
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let level = std::env::var("RUST_LOG").unwrap_or_else(|_| "debug".into());
    unsafe { std::env::set_var("RUST_LOG", &level) };
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let t0 = Instant::now();
    let mut opts = LaunchOptions::default();
    if let Some(ext_dir) = std::env::args().nth(1) {
        eprintln!("[probe] usando extensão em {ext_dir}");
        opts.extra_args.push(format!("--load-extension={ext_dir}"));
        opts.extra_args
            .push(format!("--disable-extensions-except={ext_dir}"));
    }

    eprintln!("[probe {}] launch…", elapsed(t0));
    let (mut process, browser) = chromium_driver::launch(opts).await?;
    eprintln!("[probe {}] launch ok, ws={}", elapsed(t0), process.ws_url());

    let version = browser.get_version().await?;
    eprintln!(
        "[probe {}] get_version ok: {} / {}",
        elapsed(t0),
        version.product,
        version.user_agent
    );

    eprintln!("[probe {}] create_page…", elapsed(t0));
    let target = browser.create_page("about:blank").await?;
    eprintln!("[probe {}] create_page ok; attach…", elapsed(t0));
    let page = target.attach().await?;
    eprintln!("[probe {}] attach ok; enable…", elapsed(t0));
    page.enable().await?;
    eprintln!(
        "[probe {}] enable ok; set_lifecycle_events_enabled…",
        elapsed(t0)
    );
    match page.set_lifecycle_events_enabled(true).await {
        Ok(()) => eprintln!("[probe {}] lifecycle ok", elapsed(t0)),
        Err(e) => eprintln!("[probe {}] lifecycle ERRO: {e}", elapsed(t0)),
    }

    eprintln!("[probe {}] set_user_agent…", elapsed(t0));
    page.set_user_agent(&version.user_agent, None).await?;
    eprintln!("[probe {}] set_user_agent ok; navigate…", elapsed(t0));

    page.navigate("https://sso.acesso.gov.br/login").await?;
    eprintln!("[probe {}] navigate ok; wait_for_load…", elapsed(t0));
    match page.wait_for_load(Duration::from_secs(30)).await {
        Ok(()) => eprintln!("[probe {}] load ok", elapsed(t0)),
        Err(e) => eprintln!("[probe {}] load ERRO: {e}", elapsed(t0)),
    }

    let url = page.eval_value("location.href").await?;
    eprintln!("[probe {}] url atual: {url}", elapsed(t0));

    let dom = page.dom().await?;
    eprintln!("[probe {}] dom ok; wait_for input#accountId…", elapsed(t0));
    match dom
        .wait_for("input#accountId", Duration::from_secs(30))
        .await
    {
        Ok(_) => eprintln!("[probe {}] input#accountId OK", elapsed(t0)),
        Err(e) => eprintln!("[probe {}] input#accountId ERRO: {e}", elapsed(t0)),
    }

    let _ = browser.close().await;
    let _ = process.wait().await;
    eprintln!("[probe {}] fim", elapsed(t0));
    Ok(())
}
