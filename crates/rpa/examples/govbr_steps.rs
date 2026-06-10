//! Replica os passos de `do_fresh_login` inline, logando URL/título a cada
//! etapa e salvando dump (HTML + screenshot) em `/tmp/govbr-dumps` quando um
//! passo falha — pra localizar exatamente onde o login trava.
//!
//! Uso: `govbr_steps <CPF> <SENHA> [OTP]`

use std::path::Path;
use std::time::{Duration, Instant};

use rpa::govbr::launch;

const T: Duration = Duration::from_secs(30);
const DUMP_DIR: &str = "/tmp/govbr-dumps";

fn el(t0: Instant) -> String {
    format!("{:.1}s", t0.elapsed().as_secs_f32())
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        if std::env::var("RUST_LOG").is_err() {
            std::env::set_var("RUST_LOG", "rpa=debug,chromium_driver=info");
        }
    }
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let mut args = std::env::args().skip(1);
    let cpf = args.next().expect("uso: govbr_steps <CPF> <SENHA> [OTP]");
    let password = args.next().expect("uso: govbr_steps <CPF> <SENHA> [OTP]");
    let _otp = args.next();

    let t0 = Instant::now();
    let dump = Path::new(DUMP_DIR);

    let opts = launch::options_with_extensions().await?;
    let (mut process, browser) = chromium_driver::launch(opts).await?;
    eprintln!("[{}] launch ok", el(t0));

    let page = browser.create_page("about:blank").await?.attach().await?;
    page.enable().await?;
    page.set_lifecycle_events_enabled(true).await.ok();

    let url = format!("https://contas.acesso.gov.br/contas/{cpf}");
    page.navigate(&url).await?;
    page.wait_for_load(T).await.ok();
    page.wait_for_network_idle(Duration::from_secs(2))
        .await
        .ok();
    where_now(&page, t0, "após navigate").await;

    let dom = page.dom().await?;

    // Passo 1: CPF
    eprintln!("[{}] esperando input#accountId…", el(t0));
    match dom.wait_for("input#accountId", T).await {
        Ok(cpf_input) => {
            cpf_input.click().await?;
            cpf_input.type_text(&cpf).await?;
            eprintln!("[{}] CPF digitado", el(t0));
            let btn = dom.query_selector("button#enter-account-id").await?;
            btn.click().await?;
            page.wait_for_load(T).await.ok();
            dom.invalidate();
        }
        Err(e) => {
            eprintln!("[{}] FALHA input#accountId: {e}", el(t0));
            let _ = page.debug_dump_in(dump, "step1-cpf").await;
            return finish(&browser, &mut process).await;
        }
    }
    where_now(&page, t0, "após enviar CPF").await;
    detect_captcha(&page, t0, "pós-cpf").await;

    // Passo 2: senha
    eprintln!("[{}] esperando input#password…", el(t0));
    match dom.wait_for("input#password", T).await {
        Ok(pwd) => {
            pwd.click().await?;
            pwd.type_text(&password).await?;
            eprintln!("[{}] senha digitada", el(t0));
            let btn = dom.query_selector("button#submit-button").await?;
            btn.click().await?;
            dom.invalidate();
        }
        Err(e) => {
            eprintln!("[{}] FALHA input#password: {e}", el(t0));
            let _ = page.debug_dump_in(dump, "step2-senha").await;
            where_now(&page, t0, "no timeout da senha").await;
            return finish(&browser, &mut process).await;
        }
    }
    where_now(&page, t0, "após enviar senha").await;
    detect_captcha(&page, t0, "pós-senha").await;

    // Aguardar tela pós-senha (2FA / perfil / erro)
    eprintln!("[{}] aguardando tela pós-senha…", el(t0));
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if dom.try_query_selector("input#otpInput").await?.is_some() {
            eprintln!("[{}] -> 2FA (input#otpInput presente)", el(t0));
            break;
        }
        if dom.try_query_selector("input#accountId").await?.is_some()
            && dom
                .try_query_selector(".br-message.warning")
                .await?
                .is_some()
        {
            eprintln!("[{}] -> ERRO de credencial (.br-message.warning)", el(t0));
            let _ = page.debug_dump_in(dump, "step3-credencial-invalida").await;
            break;
        }
        if cur(&page).await.contains("contas.acesso.gov.br") {
            eprintln!("[{}] -> PERFIL (logado, 2FA dispensado)", el(t0));
            break;
        }
        if Instant::now() >= deadline {
            eprintln!("[{}] -> TIMEOUT: nenhuma tela esperada apareceu", el(t0));
            let _ = page.debug_dump_in(dump, "step3-timeout").await;
            where_now(&page, t0, "timeout pós-senha").await;
            break;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    where_now(&page, t0, "estado final").await;
    let _ = page.debug_dump_in(dump, "final").await;
    finish(&browser, &mut process).await
}

async fn cur(page: &chromium_driver::PageSession) -> String {
    page.eval_value("location.href")
        .await
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default()
}

async fn where_now(page: &chromium_driver::PageSession, t0: Instant, label: &str) {
    let url = cur(page).await;
    let title = page
        .eval_value("document.title")
        .await
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default();
    eprintln!("[{}] {label}: url={url} | title={title:?}", el(t0));
}

async fn detect_captcha(page: &chromium_driver::PageSession, t0: Instant, stage: &str) {
    let frames = page.get_frames().await.unwrap_or_default();
    let hcap: Vec<&str> = frames
        .iter()
        .map(|f| f.url.as_str())
        .filter(|u| u.contains("hcaptcha.com"))
        .collect();
    if hcap.is_empty() {
        eprintln!("[{}] captcha {stage}: nenhum iframe hcaptcha", el(t0));
    } else {
        eprintln!(
            "[{}] captcha {stage}: {} iframes hcaptcha:",
            el(t0),
            hcap.len()
        );
        for u in hcap {
            eprintln!("        {u}");
        }
    }
}

async fn finish(
    browser: &chromium_driver::Browser,
    process: &mut chromium_driver::ChromiumProcess,
) -> Result<(), Box<dyn std::error::Error>> {
    let _ = browser.close().await;
    let _ = process.wait().await;
    Ok(())
}
