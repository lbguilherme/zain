//! Sonda exploratória do PGMEI (Programa Gerador do DAS do MEI), pra
//! mapear as telas de consulta de débitos/pagamentos e emissão de guia
//! (boleto/PIX) antes de escrever as rotas definitivas do RPA.
//!
//! Uso: `pgmei_probe <CNPJ> [SESSION_JSON] [--ate N]`
//!
//! Dumpa HTML+screenshot de cada tela em `/tmp/pgmei-dumps` e imprime
//! um inventário (inputs/botões/links/iframes) no stderr a cada passo.

use std::path::Path;
use std::time::Duration;

use chromium_driver::PageSession;
use rpa::govbr::{launch, session::SavedSession};

const DUMP_DIR: &str = "/tmp/pgmei-dumps";
const PGMEI_URL: &str =
    "https://www8.receita.fazenda.gov.br/SimplesNacional/Aplicacoes/ATSPO/pgmei.app/Identificacao";

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
    let cnpj = args.next().expect("uso: pgmei_probe <CNPJ> [SESSION_JSON]");
    let session_path = args.next();
    let dump_dir = Path::new(DUMP_DIR);

    let opts = launch::options_with_extensions().await?;
    let (mut process, browser) = chromium_driver::launch(opts).await?;
    let result: anyhow::Result<()> = async {
        launch::configure_nopecha(&browser).await.ok();

        let page = browser.create_page("about:blank").await?.attach().await?;
        page.enable().await?;
        page.set_lifecycle_events_enabled(true).await.ok();

        if let Some(path) = &session_path {
            let json = std::fs::read_to_string(path)?;
            let saved: SavedSession = serde_json::from_str(&json)?;
            rpa::govbr::session::restore(&browser, &page, &saved).await?;
            eprintln!("[probe] sessão restaurada ({} cookies)", saved.cookies.len());
        }

        // ── Passo 1: identificação ─────────────────────────────────
        page.navigate(PGMEI_URL).await?;
        page.wait_for_load(Duration::from_secs(20)).await.ok();
        page.wait_for_network_idle(Duration::from_secs(3)).await.ok();
        dump(&page, dump_dir, "01-identificacao").await;

        // CNPJ + Continuar. O PGMEI clássico é ASP.NET: input #cnpj com
        // máscara jQuery — precisa de foco + teclas reais.
        page.eval_value(
            r#"(() => { const el = document.querySelector('#cnpj'); el.focus(); el.value=''; el.dispatchEvent(new Event('input',{bubbles:true})); })()"#,
        )
        .await?;
        let dom = page.dom().await?;
        let el = dom.wait_for("#cnpj", Duration::from_secs(10)).await?;
        el.type_text(&cnpj).await?;
        let valor = page
            .eval_value(r#"document.querySelector('#cnpj').value"#)
            .await?;
        eprintln!("[probe] valor do campo cnpj: {valor:?}");

        let clicked = click_submit_like(&page, &["continuar", "ok", "entrar"]).await?;
        eprintln!("[probe] clicou: {clicked}");
        // hCaptcha invisível roda no submit; NopeCHA resolve em background.
        // Espera a URL sair da Identificacao (até 90s), logando o estado.
        let deadline = tokio::time::Instant::now() + Duration::from_secs(90);
        loop {
            tokio::time::sleep(Duration::from_secs(3)).await;
            let url = page
                .eval_value("location.href")
                .await?
                .as_str()
                .unwrap_or("")
                .to_string();
            let toast = page
                .eval_value(
                    r#"(() => { const t = document.querySelector('.toast-message'); return t ? t.textContent.trim() : null; })()"#,
                )
                .await
                .ok()
                .and_then(|v| v.as_str().map(str::to_string));
            eprintln!("[probe] aguardando pós-submit url={url} toast={toast:?}");
            if !url.contains("Identificacao") {
                break;
            }
            if tokio::time::Instant::now() >= deadline {
                eprintln!("[probe] timeout aguardando sair da Identificacao");
                break;
            }
        }
        page.wait_for_network_idle(Duration::from_secs(3)).await.ok();
        dump(&page, dump_dir, "02-home").await;

        // ── Passo 2: menu "Emitir Guia de Pagamento (DAS)" ────────
        let nav = page
            .eval_value(
                r#"(() => {
                    const links = [...document.querySelectorAll('a')];
                    const alvo = links.find(a => /emitir guia/i.test(a.textContent || ''));
                    if (!alvo) return null;
                    alvo.click();
                    return (alvo.textContent || '').trim();
                })()"#,
            )
            .await?;
        eprintln!("[probe] menu emitir: {nav:?}");
        page.wait_for_load(Duration::from_secs(20)).await.ok();
        page.wait_for_network_idle(Duration::from_secs(3)).await.ok();
        tokio::time::sleep(Duration::from_secs(1)).await;
        dump(&page, dump_dir, "03-emitir-guia").await;

        // ── Passo 3: escolhe o ano corrente, se houver select ─────
        let ano = page
            .eval_value(
                r#"(() => {
                    const sel = document.querySelector('select');
                    if (!sel) return null;
                    const opts = [...sel.options].map(o => o.value);
                    // pega o maior ano disponível
                    const alvo = opts.filter(v => /^\d{4}$/.test(v)).sort().pop() || opts.pop();
                    sel.value = alvo;
                    sel.dispatchEvent(new Event('change', { bubbles: true }));
                    return alvo;
                })()"#,
            )
            .await?;
        eprintln!("[probe] ano selecionado: {ano:?}");
        tokio::time::sleep(Duration::from_millis(500)).await;
        let ok = click_submit_like(&page, &["ok", "continuar", "apurar"]).await.ok();
        eprintln!("[probe] submit do ano: {ok:?}");
        page.wait_for_load(Duration::from_secs(30)).await.ok();
        page.wait_for_network_idle(Duration::from_secs(3)).await.ok();
        tokio::time::sleep(Duration::from_secs(2)).await;
        dump(&page, dump_dir, "04-periodos").await;

        // ── Passo 4: seleciona o mês devedor e gera o DAS ──────────
        let mes = page
            .eval_value(
                r#"(() => {
                    const cb = [...document.querySelectorAll('input.paSelecionado:not([disabled])')][0];
                    if (!cb) return null;
                    cb.click();
                    return cb.value;
                })()"#,
            )
            .await?;
        eprintln!("[probe] mês selecionado: {mes:?}");
        tokio::time::sleep(Duration::from_millis(500)).await;
        page.eval_value(r#"document.querySelector('#btnEmitirDas').click()"#)
            .await?;
        page.wait_for_load(Duration::from_secs(30)).await.ok();
        page.wait_for_network_idle(Duration::from_secs(5)).await.ok();
        tokio::time::sleep(Duration::from_secs(2)).await;
        dump(&page, dump_dir, "05-das-gerado").await;

        // ── Passo 5: baixa o PDF do DAS via fetch in-page ──────────
        let pdf_b64 = page
            .eval_value_async(
                r#"(async () => {
                    const r = await fetch('/SimplesNacional/Aplicacoes/ATSPO/pgmei.app/emissao/imprimir', { credentials: 'include' });
                    if (!r.ok) return 'HTTP ' + r.status;
                    const buf = await r.arrayBuffer();
                    const bytes = new Uint8Array(buf);
                    let bin = '';
                    const chunk = 0x8000;
                    for (let i = 0; i < bytes.length; i += chunk) {
                        bin += String.fromCharCode.apply(null, bytes.subarray(i, i + chunk));
                    }
                    return btoa(bin);
                })()"#,
            )
            .await?;
        let pdf_b64 = pdf_b64.as_str().unwrap_or("");
        if pdf_b64.starts_with("HTTP ") {
            eprintln!("[probe] download do PDF falhou: {pdf_b64}");
        } else {
            use base64::Engine as _;
            let bytes = base64::engine::general_purpose::STANDARD.decode(pdf_b64)?;
            let path = dump_dir.join("das-202604.pdf");
            std::fs::write(&path, &bytes)?;
            eprintln!("[probe] PDF salvo: {} ({} bytes)", path.display(), bytes.len());
        }

        eprintln!("[probe] fim — dumps em {DUMP_DIR}");
        Ok(())
    }
    .await;

    if let Err(e) = &result {
        eprintln!("[probe] ERRO: {e:?}");
        // dump final de socorro
        if let Ok(p) = browser.create_page("about:blank").await {
            drop(p);
        }
    }
    let _ = browser.close().await;
    let _ = process.wait().await;
    result.map_err(Into::into)
}

async fn dump(page: &PageSession, dir: &Path, name: &str) {
    if let Err(e) = page.debug_dump_in(dir, name).await {
        eprintln!("[probe] dump {name} falhou: {e}");
    }
    inventory(page, name).await;
}

/// Imprime inputs, botões, selects, links e iframes visíveis — o mapa
/// pra evoluir a sonda sem adivinhar seletor.
async fn inventory(page: &PageSession, label: &str) {
    let v = page
        .eval_value(
            r#"(() => {
                const vis = el => !!(el.offsetWidth || el.offsetHeight || el.getClientRects().length);
                const desc = el => ({
                    tag: el.tagName.toLowerCase(),
                    id: el.id || undefined,
                    name: el.getAttribute('name') || undefined,
                    type: el.getAttribute('type') || undefined,
                    text: (el.textContent || el.value || '').replace(/\s+/g, ' ').trim().slice(0, 80) || undefined,
                    href: el.getAttribute('href') || undefined,
                });
                return {
                    url: location.href,
                    title: document.title,
                    inputs: [...document.querySelectorAll('input, select, textarea')].filter(vis).map(desc),
                    buttons: [...document.querySelectorAll('button, input[type=submit], input[type=button], a.btn, a.button')].filter(vis).map(desc),
                    links: [...document.querySelectorAll('a')].filter(vis).map(desc).slice(0, 40),
                    iframes: [...document.querySelectorAll('iframe')].map(f => f.src),
                };
            })()"#,
        )
        .await;
    match v {
        Ok(v) => eprintln!(
            "[inventário {label}] {}",
            serde_json::to_string_pretty(&v).unwrap_or_default()
        ),
        Err(e) => eprintln!("[inventário {label}] falhou: {e}"),
    }
}

/// Clica no primeiro botão/submit cujo texto case com um dos rótulos.
async fn click_submit_like(page: &PageSession, labels: &[&str]) -> anyhow::Result<String> {
    let labels_js = serde_json::to_string(labels)?;
    let v = page
        .eval_value(&format!(
            r#"(() => {{
                const labels = {labels_js};
                const vis = el => !!(el.offsetWidth || el.offsetHeight || el.getClientRects().length);
                const cands = [...document.querySelectorAll('button, input[type=submit], input[type=button], a.btn, a.button')].filter(vis);
                for (const lbl of labels) {{
                    const b = cands.find(x => ((x.textContent || x.value || '')).replace(/\s+/g,' ').trim().toLowerCase().includes(lbl));
                    if (b) {{ b.click(); return (b.textContent || b.value || '').trim(); }}
                }}
                return null;
            }})()"#
        ))
        .await?;
    v.as_str()
        .map(str::to_string)
        .ok_or_else(|| anyhow::anyhow!("nenhum botão {labels:?} visível"))
}
