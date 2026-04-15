//! RPA para consulta do Certificado da Condição de MEI (CCMEI) no
//! portal da Receita.
//!
//! A página é pública (não exige sessão gov.br): aceita CPF ou CNPJ como
//! entrada, redireciona para a visualização do certificado se o
//! documento tem MEI ativo, ou mostra um banner `br-message.danger` se
//! não tem. O formulário é Angular + ngx-mask, então seguimos o mesmo
//! padrão de [`crate::mei::inscricao`]: digitação real via CDP para
//! acionar a máscara e clicks via JS.
//!
//! Quando o certificado existe, além de extrair os dados da tela, o RPA
//! clica em "Fazer Download do Certificado em PDF" e captura os bytes
//! do arquivo baixado via `Browser.setDownloadBehavior` + polling no
//! diretório temporário — chromium_driver ainda não expõe um evento
//! tipado pra `Browser.downloadProgress`, então observamos o filesystem.

use std::path::{Path, PathBuf};
use std::time::Duration;

use chromium_driver::PageSession;
use chromium_driver::cdp::browser::DownloadBehavior;
use serde::{Deserialize, Serialize};

const CONSULTA_CERTIFICADO_URL: &str = "https://mei.receita.economia.gov.br/certificado/consulta";

const TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificadoMei {
    pub nome_empresario: String,
    pub cpf: String,
    pub cnpj: String,
    /// Data de abertura em ISO (YYYY-MM-DD). Mantém o formato BR se o
    /// parse falhar.
    pub data_abertura: String,
    pub nome_empresarial: String,
    pub capital_social: String,
    pub situacao_cadastral: String,
    /// Data da situação cadastral em ISO (YYYY-MM-DD).
    pub data_situacao_cadastral: String,
    pub endereco_comercial: EnderecoCertificado,
    pub situacao_atual: String,
    pub periodos_mei: Vec<PeriodoMei>,
    pub forma_atuacao: String,
    pub ocupacao_principal: String,
    pub atividade_principal: String,
    /// Bytes do PDF do certificado, capturados via download CDP após o
    /// click em "Fazer Download do Certificado em PDF". Fica fora do
    /// JSON serializado (`#[serde(skip)]`) porque o tamanho explodiria
    /// logs e respostas LLM — use o campo diretamente em Rust.
    #[serde(skip)]
    pub pdf: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnderecoCertificado {
    pub cep: String,
    pub logradouro: String,
    pub numero: String,
    pub complemento: Option<String>,
    pub bairro: String,
    pub municipio: String,
    pub uf: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodoMei {
    /// Texto do período (ex.: "1° Período").
    pub periodo: String,
    /// Data inicial em ISO (YYYY-MM-DD).
    pub inicio: String,
    /// Data final em ISO, ou `None` se ainda estiver em aberto
    /// (portal mostra "-").
    pub fim: Option<String>,
}

/// Consulta o certificado de MEI por CPF ou CNPJ. `None` indica que o
/// documento não tem empresas MEI (tela com banner `.br-message.danger`);
/// `Some(certificado)` devolve todos os dados disponíveis na tela de
/// visualização.
pub async fn consultar_certificado(cpf_or_cnpj: &str) -> anyhow::Result<Option<CertificadoMei>> {
    let digits: String = cpf_or_cnpj.chars().filter(|c| c.is_ascii_digit()).collect();
    let tipo = match digits.len() {
        11 => TipoConsulta::Cpf,
        14 => TipoConsulta::Cnpj,
        n => anyhow::bail!("documento deve ter 11 (CPF) ou 14 (CNPJ) dígitos, recebido {n}"),
    };

    // Diretório temporário exclusivo para o download do PDF. O nome
    // carrega PID + nanos pra evitar colisões entre execuções paralelas
    // ou reutilização acidental. Limpamos no final, sucesso ou falha.
    let download_dir = std::env::temp_dir().join(format!(
        "zain-mei-certificado-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0),
    ));
    std::fs::create_dir_all(&download_dir)?;
    let download_dir_str = download_dir
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("download_dir tem bytes não-UTF8"))?
        .to_string();

    let (mut process, browser) =
        chromium_driver::launch(chromium_driver::LaunchOptions::default()).await?;

    let result: anyhow::Result<Option<CertificadoMei>> = async {
        // Liga o download antes de qualquer navegação pra garantir que
        // o handler já está armado quando clicarmos no botão de PDF.
        browser
            .set_download_behavior(DownloadBehavior::Allow, Some(&download_dir_str))
            .await?;

        let page = browser.create_page("about:blank").await?.attach().await?;
        page.enable().await?;

        page.navigate(CONSULTA_CERTIFICADO_URL).await?;
        page.wait_for_load(TIMEOUT).await.ok();

        let dom = page.dom().await?;
        // Aguarda o form estar totalmente hidratado: o botão "Continuar"
        // só aparece depois que o Angular termina o bootstrap.
        dom.wait_for("app-consulta button.br-button.primary", TIMEOUT)
            .await?;

        // Default é CNPJ (`#tipo-0` checked). Se for CPF, clica no label
        // do radio alternativo — o `<input type=radio>` tem opacity:0 do
        // design system, então o click no input direto não registra.
        if matches!(tipo, TipoConsulta::Cpf) {
            click_js(&page, "app-consulta label[for=tipo-1]").await?;
            tokio::time::sleep(Duration::from_millis(200)).await;
        }

        // ngx-mask não reconhece value setado via JS — precisa de teclas
        // reais pra aplicar a máscara e sincronizar o FormControl.
        let input_selector = match tipo {
            TipoConsulta::Cpf => "app-consulta input[name=cpf]",
            TipoConsulta::Cnpj => "app-consulta input[name=cnpj]",
        };
        type_text_human(&page, input_selector, &digits).await?;

        click_js(&page, "app-consulta button.br-button.primary").await?;

        match wait_for_outcome(&page).await? {
            Outcome::SemMei => Ok(None),
            Outcome::Certificado => {
                let mut cert = extrair_certificado(&page).await?;
                // Clica "Fazer Download do Certificado em PDF" (único
                // primary na `.button-row` — o "Voltar" é `secondary`)
                // e aguarda o arquivo aparecer no download_dir.
                click_js(
                    &page,
                    "app-visualizacao-certificado .button-row button.br-button.primary",
                )
                .await?;
                let pdf_path = wait_for_download(&download_dir).await?;
                cert.pdf = std::fs::read(&pdf_path)?;
                Ok(Some(cert))
            }
        }
    }
    .await;

    let _ = browser.close().await;
    let _ = process.wait().await;
    let _ = std::fs::remove_dir_all(&download_dir);
    result
}

/// Poll o diretório de download até aparecer um arquivo finalizado
/// (i.e., não `.crdownload`) com tamanho não-zero. O Chromium escreve
/// primeiro um `<nome>.crdownload` e renomeia atomicamente pro nome
/// final quando termina.
async fn wait_for_download(dir: &Path) -> anyhow::Result<PathBuf> {
    const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(60);
    const POLL_INTERVAL: Duration = Duration::from_millis(300);

    let deadline = tokio::time::Instant::now() + DOWNLOAD_TIMEOUT;
    loop {
        if let Some(path) = find_completed_file(dir)? {
            return Ok(path);
        }
        if tokio::time::Instant::now() >= deadline {
            anyhow::bail!(
                "timeout aguardando download do certificado MEI (dir: {})",
                dir.display()
            );
        }
        tokio::time::sleep(POLL_INTERVAL).await;
    }
}

fn find_completed_file(dir: &Path) -> anyhow::Result<Option<PathBuf>> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("crdownload") {
            continue;
        }
        let meta = entry.metadata()?;
        if meta.is_file() && meta.len() > 0 {
            return Ok(Some(path));
        }
    }
    Ok(None)
}

enum TipoConsulta {
    Cpf,
    Cnpj,
}

enum Outcome {
    SemMei,
    Certificado,
}

/// Poll até a próxima tela aparecer. O Angular troca o componente sem
/// navegar, então olhamos o DOM direto: `app-visualizacao-certificado`
/// significa sucesso; `.br-message.danger` dentro do `<br-message>` do
/// layout significa "sem MEI".
async fn wait_for_outcome(page: &PageSession) -> anyhow::Result<Outcome> {
    let deadline = tokio::time::Instant::now() + TIMEOUT;
    loop {
        let v = page
            .eval_value(
                r#"(() => {
                    if (document.querySelector('app-visualizacao-certificado')) return 'certificado';
                    if (document.querySelector('br-message .br-message.danger')) return 'sem_mei';
                    return 'pending';
                })()"#,
            )
            .await?;
        match v.as_str().unwrap_or("") {
            "certificado" => return Ok(Outcome::Certificado),
            "sem_mei" => return Ok(Outcome::SemMei),
            _ => {}
        }
        if tokio::time::Instant::now() >= deadline {
            anyhow::bail!("timeout aguardando resposta da consulta de certificado MEI");
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
}

/// Extrai todos os campos da tela `app-visualizacao-certificado` num
/// único `eval` pra evitar N roundtrips CDP. Os ids são estáveis (vêm do
/// próprio template do portal).
async fn extrair_certificado(page: &PageSession) -> anyhow::Result<CertificadoMei> {
    let v = page
        .eval_value(
            r#"(() => {
                const root = document.querySelector('app-visualizacao-certificado');
                if (!root) return null;
                const txt = (sel) => {
                    const el = root.querySelector(sel);
                    return el ? (el.textContent || '').replace(/\s+/g, ' ').trim() : '';
                };
                const periodos = [];
                for (const tr of root.querySelectorAll('#periodosComoMei tbody tr')) {
                    const tds = tr.querySelectorAll('td');
                    if (tds.length < 3) continue;
                    periodos.push({
                        periodo: (tds[0].textContent || '').replace(/\s+/g, ' ').trim(),
                        inicio: (tds[1].textContent || '').trim(),
                        fim: (tds[2].textContent || '').trim(),
                    });
                }
                return {
                    nome_empresario: txt('#nomeEmpresario'),
                    cpf: txt('#cpf'),
                    cnpj: txt('#cnpj'),
                    data_abertura: txt('#dataAbertura'),
                    nome_empresarial: txt('#nomeEmpresarial'),
                    capital_social: txt('#capitalSocial'),
                    situacao_cadastral: txt('#situacaoCadastral'),
                    data_situacao_cadastral: txt('#dataInicioSituacao'),
                    endereco_comercial: {
                        cep: txt('#cepEnderecoComercial'),
                        logradouro: txt('#logradouroEnderecoComercial'),
                        numero: txt('#numeroEnderecoComercial'),
                        complemento: txt('#complementoEnderecoComercial'),
                        bairro: txt('#bairroEnderecoComercial'),
                        municipio: txt('#municipioEnderecoComercial'),
                        uf: txt('#ufEnderecoComercial'),
                    },
                    situacao_atual: txt('#situcaoAtual'),
                    periodos_mei: periodos,
                    forma_atuacao: txt('#formaAtuacao'),
                    ocupacao_principal: txt('#ocupacaoPrincipal'),
                    atividade_principal: txt('#atividadePrincipal'),
                };
            })()"#,
        )
        .await?;

    let obj = v
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("extração do certificado retornou null"))?;
    let get = |k: &str| -> String {
        obj.get(k)
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .trim()
            .to_string()
    };

    let end_obj = obj
        .get("endereco_comercial")
        .and_then(|v| v.as_object())
        .ok_or_else(|| anyhow::anyhow!("endereco_comercial ausente na extração"))?;
    let end = |k: &str| -> String {
        end_obj
            .get(k)
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .trim()
            .to_string()
    };

    let complemento_raw = end("complemento");
    let complemento = if complemento_raw == "-" || complemento_raw.is_empty() {
        None
    } else {
        Some(complemento_raw)
    };

    let periodos_raw = obj
        .get("periodos_mei")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let periodos_mei: Vec<PeriodoMei> = periodos_raw
        .into_iter()
        .filter_map(|p| {
            let po = p.as_object()?;
            let periodo_str = po.get("periodo")?.as_str()?.to_string();
            let inicio_br = po.get("inicio")?.as_str()?.trim().to_string();
            let fim_br = po.get("fim")?.as_str()?.trim().to_string();
            let fim = if fim_br == "-" || fim_br.is_empty() {
                None
            } else {
                Some(parse_date_br(&fim_br))
            };
            Some(PeriodoMei {
                periodo: periodo_str,
                inicio: parse_date_br(&inicio_br),
                fim,
            })
        })
        .collect();

    Ok(CertificadoMei {
        nome_empresario: get("nome_empresario"),
        cpf: get("cpf"),
        cnpj: get("cnpj"),
        data_abertura: parse_date_br(&get("data_abertura")),
        nome_empresarial: get("nome_empresarial"),
        capital_social: get("capital_social"),
        situacao_cadastral: get("situacao_cadastral"),
        data_situacao_cadastral: parse_date_br(&get("data_situacao_cadastral")),
        endereco_comercial: EnderecoCertificado {
            cep: end("cep"),
            logradouro: end("logradouro"),
            numero: end("numero"),
            complemento,
            bairro: end("bairro"),
            municipio: end("municipio"),
            uf: end("uf"),
        },
        situacao_atual: get("situacao_atual"),
        periodos_mei,
        forma_atuacao: get("forma_atuacao"),
        ocupacao_principal: get("ocupacao_principal"),
        atividade_principal: get("atividade_principal"),
        // Preenchido depois, fora desta função — ver `consultar_certificado`.
        pdf: Vec::new(),
    })
}

/// Converte "DD/MM/YYYY" → "YYYY-MM-DD". Retorna a string original se
/// não casar o formato — mantém o dado visível pro caller mesmo quando
/// o portal muda o layout.
fn parse_date_br(s: &str) -> String {
    let s = s.trim();
    let parts: Vec<&str> = s.split('/').collect();
    if parts.len() == 3 && parts[0].len() == 2 && parts[1].len() == 2 && parts[2].len() == 4 {
        format!("{}-{}-{}", parts[2], parts[1], parts[0])
    } else {
        s.to_string()
    }
}

async fn click_js(page: &PageSession, selector: &str) -> anyhow::Result<()> {
    let selector_js = serde_json::to_string(selector)?;
    let js = format!(
        r#"(() => {{
            const el = document.querySelector({selector_js});
            if (!el) return 'not_found';
            el.click();
            return 'ok';
        }})()"#
    );
    let v = page.eval_value(&js).await?;
    match v.as_str().unwrap_or("") {
        "ok" => Ok(()),
        "not_found" => anyhow::bail!("elemento não encontrado: {selector}"),
        other => anyhow::bail!("click_js({selector}): {other}"),
    }
}

/// Digita texto com teclas reais via CDP — necessário para campos com
/// ngx-mask/máscaras custom, onde setar `value` via JS não aciona o
/// handler da máscara e o FormControl enxerga o campo como vazio.
async fn type_text_human(page: &PageSession, selector: &str, value: &str) -> anyhow::Result<()> {
    let selector_js = serde_json::to_string(selector)?;
    let focus_js = format!(
        r#"(() => {{
            const el = document.querySelector({selector_js});
            if (!el) return 'not_found';
            el.focus();
            const setter = Object.getOwnPropertyDescriptor(
                window.HTMLInputElement.prototype, 'value'
            ).set;
            setter.call(el, '');
            el.dispatchEvent(new Event('input', {{ bubbles: true }}));
            return 'ok';
        }})()"#
    );
    let v = page.eval_value(&focus_js).await?;
    match v.as_str().unwrap_or("") {
        "ok" => {}
        "not_found" => anyhow::bail!("elemento não encontrado: {selector}"),
        other => anyhow::bail!("type_text_human focus({selector}): {other}"),
    }

    let dom = page.dom().await?;
    let element = dom.query_selector(selector).await?;
    element.type_text(value).await?;

    let blur_js = format!(
        r#"(() => {{
            const el = document.querySelector({selector_js});
            if (!el) return 'not_found';
            el.blur();
            el.dispatchEvent(new Event('blur', {{ bubbles: true }}));
            return 'ok';
        }})()"#
    );
    page.eval_value(&blur_js).await?;
    Ok(())
}
