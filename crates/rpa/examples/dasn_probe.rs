//! Sonda exploratória da DASN-SIMEI (Declaração Anual do MEI), pra mapear
//! as telas de STATUS — anos declarados, pendentes, em atraso, recibo —
//! antes de escrever as rotas do RPA. Somente leitura.
//!
//! Uso: `dasn_probe <CNPJ> [SESSION_JSON]`
//!
//! Dumpa HTML+screenshot de cada tela em `/tmp/dasn-dumps` e imprime um
//! inventário (inputs/botões/links/selects/iframes) no stderr a cada passo.

use std::path::Path;
use std::time::Duration;

use chromium_driver::PageSession;
use rpa::govbr::{launch, session::SavedSession};

const DUMP_DIR: &str = "/tmp/dasn-dumps";
// Candidatas de URL da DASN-SIMEI — testamos a primeira que carregar com
// um #cnpj. O app clássico do Simples fica sob ATSPO/.../dasnsimei.app.
const URLS: &[&str] = &[
    "https://www8.receita.fazenda.gov.br/SimplesNacional/Aplicacoes/ATSPO/dasnsimei.app/identificacao",
    "https://www8.receita.fazenda.gov.br/SimplesNacional/Aplicacoes/ATSPO/dasnsimei.app/",
];

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
    let cnpj = args.next().expect("uso: dasn_probe <CNPJ> [SESSION_JSON]");
    let cnpj_digits: String = cnpj.chars().filter(|c| c.is_ascii_digit()).collect();
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

        // ── Passo 1: achar uma tela de identificação com #cnpj ─────
        let mut achou = false;
        for (i, url) in URLS.iter().enumerate() {
            eprintln!("[probe] tentando URL {i}: {url}");
            page.navigate(url).await?;
            page.wait_for_load(Duration::from_secs(20)).await.ok();
            page.wait_for_network_idle(Duration::from_secs(3)).await.ok();
            let tem_cnpj = page
                .eval_value("!!document.querySelector('#cnpj, input[name=cnpj]')")
                .await?
                .as_bool()
                .unwrap_or(false);
            dump(&page, dump_dir, &format!("01-url{i}")).await;
            if tem_cnpj {
                eprintln!("[probe] #cnpj encontrado em {url}");
                achou = true;
                break;
            }
        }
        if !achou {
            eprintln!("[probe] nenhuma URL trouxe #cnpj — veja os dumps 01-url* (pode exigir login gov.br ou caminho diferente)");
            return Ok(());
        }

        // ── Passo 2: digita CNPJ (máscara jQuery — retry) e continua ──
        preencher_cnpj(&page, &cnpj_digits).await?;
        clicar_continuar(&page).await?;

        // Espera sair da identificação (hCaptcha invisível roda aqui).
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
            eprintln!("[probe] pós-submit url={url} toast={toast:?}");
            if !url.to_lowercase().contains("identificacao") {
                break;
            }
            if tokio::time::Instant::now() >= deadline {
                eprintln!("[probe] timeout aguardando sair da identificação");
                break;
            }
        }
        page.wait_for_network_idle(Duration::from_secs(3)).await.ok();
        dump(&page, dump_dir, "02-pos-identificacao").await;

        // ── Passo 3: lê o status de TODOS os anos de uma vez ──────────
        // O wizard "Iniciar" embute, em cada radio de ano, o tipo de
        // declaração (Original = nunca entregue / Retificadora = já
        // entregue) e a situação especial — sem precisar selecionar nada.
        let status = page
            .eval_value(
                r#"(() => [...document.querySelectorAll('input[name=opcao]')].map(r => ({
                    ano: r.value,
                    tipo: r.getAttribute('data-tipo-declaracao'),
                    situacao_especial_tipo: r.getAttribute('data-situacao-especial-tipo'),
                    situacao_especial_eventobaixa: r.getAttribute('data-situacao-especial-eventobaixa'),
                })))()"#,
            )
            .await?;
        eprintln!(
            "[probe] STATUS POR ANO: {}",
            serde_json::to_string_pretty(&status).unwrap_or_default()
        );

        // ── Passo 4 (opt-in via DASN_PEEK=1): espia a tela "Preencher"
        //    (passo 2/4 — NÃO transmite; a transmissão só ocorre na
        //    "Conclusão") pra mapear os campos de receita. Desligado por
        //    padrão pra não entrar no wizard de declaração de terceiros. ──
        // Ano a espiar (default 2025). Pra mapear DADOS DE DECLARAÇÃO
        // PASSADA, aponte pra um ano `Retificadora` (já entregue) — o
        // wizard pré-carrega os valores declarados.
        if std::env::var("DASN_PEEK").is_err() {
            eprintln!("[probe] fim (status-only) — dumps em {DUMP_DIR}");
            return Ok(());
        }
        let ano = std::env::var("DASN_PEEK_ANO").unwrap_or_else(|_| "2025".into());
        let sel = page
            .eval_value(&format!(
                r#"(() => {{
                    const r = document.querySelector('input[name=opcao][value="{ano}"]');
                    if (!r) return 'sem_radio';
                    r.click();
                    r.checked = true;
                    r.dispatchEvent(new Event('change', {{ bubbles: true }}));
                    return 'ok';
                }})()"#
            ))
            .await?;
        eprintln!("[probe] selecionou {ano}: {sel:?}");
        tokio::time::sleep(Duration::from_millis(800)).await;
        page.eval_value(
            r#"(() => { const b = document.querySelector('#iniciar-continuar'); if (b) b.click(); })()"#,
        )
        .await?;
        page.wait_for_load(Duration::from_secs(15)).await.ok();
        page.wait_for_network_idle(Duration::from_secs(3)).await.ok();
        tokio::time::sleep(Duration::from_secs(1)).await;
        dump(&page, dump_dir, "03-preencher").await;

        // Lê os valores pré-carregados do Preencher (o que dá pra puxar de
        // uma declaração passada): receitas, total, empregado.
        let preenchido = page
            .eval_value(
                r#"(() => {
                    const val = id => { const el = document.getElementById(id); return el ? el.value : null; };
                    const empregado = document.querySelector('input[name=informacao-empregado]:checked');
                    return {
                        receita_comercio_industria: val('input-rbt-icms'),
                        receita_servicos: val('input-rbt-iss'),
                        receita_bruta_total: (document.getElementById('input-rbt-total')||{}).value
                            || (document.querySelector('[id*=total] input, [class*=total]')||{}).textContent || null,
                        possui_empregado: empregado ? empregado.value : null,
                    };
                })()"#,
            )
            .await?;
        eprintln!(
            "[probe] PREENCHER (dados da declaração de {ano}): {}",
            serde_json::to_string_pretty(&preenchido).unwrap_or_default()
        );

        // Espia o Resumo (passo 3/4 — ainda NÃO transmite; só a Conclusão
        // transmite) pra ver se há recibo/data de transmissão.
        page.eval_value(
            r#"(() => { const b = document.querySelector('#preencher-continuar'); if (b) b.click(); })()"#,
        )
        .await?;
        page.wait_for_load(Duration::from_secs(15)).await.ok();
        page.wait_for_network_idle(Duration::from_secs(3)).await.ok();
        tokio::time::sleep(Duration::from_secs(1)).await;
        dump(&page, dump_dir, "04-resumo").await;

        eprintln!("[probe] fim — dumps em {DUMP_DIR}");
        Ok(())
    }
    .await;

    if let Err(e) = &result {
        eprintln!("[probe] ERRO: {e:?}");
    }
    let _ = browser.close().await;
    let _ = process.wait().await;
    result.map_err(Into::into)
}

async fn preencher_cnpj(page: &PageSession, cnpj_digits: &str) -> anyhow::Result<()> {
    let dom = page.dom().await?;
    dom.wait_for("#cnpj, input[name=cnpj]", Duration::from_secs(15))
        .await?;
    let sel = "#cnpj, input[name=cnpj]";
    for tentativa in 1..=4u64 {
        tokio::time::sleep(Duration::from_millis(200 * tentativa)).await;
        page.eval_value(&format!(
            r#"(() => {{
                const el = document.querySelector('{sel}');
                el.focus();
                const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set;
                setter.call(el, '');
                el.dispatchEvent(new Event('input', {{ bubbles: true }}));
            }})()"#
        ))
        .await?;
        let el = dom.query_selector(sel).await?;
        el.type_text(cnpj_digits).await?;
        let registrado: String = page
            .eval_value(&format!("document.querySelector('{sel}').value"))
            .await?
            .as_str()
            .unwrap_or("")
            .chars()
            .filter(|c| c.is_ascii_digit())
            .collect();
        eprintln!("[probe] cnpj tentativa {tentativa}: campo={registrado:?}");
        if registrado == cnpj_digits {
            return Ok(());
        }
    }
    anyhow::bail!("máscara do CNPJ não bateu após 4 tentativas")
}

async fn clicar_continuar(page: &PageSession) -> anyhow::Result<()> {
    let v = page
        .eval_value(
            r#"(() => {
                const vis = el => !!(el.offsetWidth || el.offsetHeight || el.getClientRects().length);
                const cands = [...document.querySelectorAll('button, input[type=submit]')].filter(vis);
                const b = cands.find(x => /continuar|ok|entrar|avan/i.test((x.textContent||x.value||'')))
                       || document.querySelector('#continuar') || cands[0];
                if (!b) return 'sem_botao';
                b.click();
                return (b.textContent || b.value || '').trim() || 'clicado';
            })()"#,
        )
        .await?;
    eprintln!("[probe] continuar: {v:?}");
    Ok(())
}

async fn dump(page: &PageSession, dir: &Path, name: &str) {
    if let Err(e) = page.debug_dump_in(dir, name).await {
        eprintln!("[probe] dump {name} falhou: {e}");
    }
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
                });
                return {
                    url: location.href,
                    title: document.title,
                    inputs: [...document.querySelectorAll('input, select, textarea')].filter(vis).map(desc),
                    buttons: [...document.querySelectorAll('button, input[type=submit], input[type=button], a.btn, a.button')].filter(vis).map(desc),
                    tabelas: [...document.querySelectorAll('table')].map(t => (t.querySelector('caption, thead')||t).textContent.replace(/\s+/g,' ').trim().slice(0,200)),
                    iframes: [...document.querySelectorAll('iframe')].map(f => f.src),
                };
            })()"#,
        )
        .await;
    match v {
        Ok(v) => eprintln!(
            "[inv {name}] {}",
            serde_json::to_string_pretty(&v).unwrap_or_default()
        ),
        Err(e) => eprintln!("[inv {name}] falhou: {e}"),
    }
}
