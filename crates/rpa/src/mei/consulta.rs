use std::time::Duration;

use super::{CONSULTA_URL, ConsultaOptante, Periodo, Situacao};

/// Strips CNPJ formatting, keeping only digits.
fn normalize_cnpj(cnpj: &str) -> String {
    cnpj.chars().filter(|c| c.is_ascii_digit()).collect()
}

/// Converts "DD/MM/YYYY" to "YYYY-MM-DD".
/// Returns the original string if parsing fails.
fn parse_date_br(s: &str) -> String {
    let s = s.trim();
    let parts: Vec<&str> = s.split('/').collect();
    if parts.len() == 3 && parts[0].len() == 2 && parts[1].len() == 2 && parts[2].len() == 4 {
        format!("{}-{}-{}", parts[2], parts[1], parts[0])
    } else {
        s.to_string()
    }
}

/// Converts "DD/MM/YYYY HH:MM:SS" to "YYYY-MM-DDTHH:MM:SS".
/// Returns the original string if parsing fails.
fn parse_timestamp_br(s: &str) -> String {
    let s = s.trim();
    let mut parts = s.splitn(2, ' ');
    let date = parts.next().unwrap_or("");
    let time = parts.next().unwrap_or("");
    let date_iso = parse_date_br(date);
    if date_iso != date && !time.is_empty() {
        format!("{date_iso}T{time}")
    } else {
        s.to_string()
    }
}

/// Parses situacao text like "Optante pelo Simples Nacional desde DD/MM/YYYY"
/// into a Situacao { optante: true, desde: Some("YYYY-MM-DD") },
/// or "NÃO optante..." into Situacao { optante: false, desde: None }.
fn parse_situacao(s: &str) -> Situacao {
    let s = s.trim();
    if s.starts_with("NÃO") || s.starts_with("Não") {
        Situacao {
            optante: false,
            desde: None,
        }
    } else if let Some(pos) = s.find("desde ") {
        let date_part = &s[pos + 6..];
        Situacao {
            optante: true,
            desde: Some(parse_date_br(date_part)),
        }
    } else {
        Situacao {
            optante: !s.contains("NÃO") && !s.contains("Não"),
            desde: None,
        }
    }
}

pub async fn consultar_optante(cnpj: &str) -> anyhow::Result<ConsultaOptante> {
    let cnpj_digits = normalize_cnpj(cnpj);
    let timeout = Duration::from_secs(15);

    let (mut process, browser) =
        chromium_driver::launch(chromium_driver::LaunchOptions::default()).await?;

    let page = browser.create_page("about:blank").await?.attach().await?;
    page.enable().await?;

    let result = async {
        // 1. Navigate to the main page (contains iframe with the form)
        page.navigate(CONSULTA_URL).await?;
        page.wait_for_load(timeout).await?;

        // 2. Enter the iframe via FrameSession
        let frame_info = page.wait_for_frame("consultaoptantes", timeout).await?;
        let frame = page.frame(&frame_info.id).await?;
        let dom = frame.dom().await?;

        // 3. Fill CNPJ and click Consultar
        let input = dom.wait_for("#Cnpj", timeout).await?;
        input.click().await?;
        input.type_text(&cnpj_digits).await?;

        let btn = dom.query_selector("button.btn-verde").await?;
        btn.click().await?;

        // 4. Wait for the iframe to reload with results.
        // The iframe is cross-origin so wait_for_load on the parent page
        // won't fire. Poll the iframe DOM for the result content instead.
        let dom = loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let frame_info = page
                .wait_for_frame("consultaoptantes", Duration::from_secs(5))
                .await?;
            let frame = page.frame(&frame_info.id).await?;
            let dom = frame.dom().await?;
            // Check if the result page loaded (has #conteudo)
            if dom.try_query_selector("#conteudo").await?.is_some() {
                break dom;
            }
        };

        // 5. Check for errors (e.g. invalid CNPJ, captcha)
        if let Some(error_el) = dom.try_query_selector(".validation-summary-errors").await? {
            let error_text = error_el.text().await?;
            anyhow::bail!("Consulta error: {}", error_text.trim());
        }

        // 6. Extract basic fields
        let data_consulta_raw = dom
            .query_selector("#conteudo h5 span")
            .await?
            .text()
            .await?
            .trim()
            .to_string();
        let data_consulta = parse_timestamp_br(&data_consulta_raw);

        let spans = dom.query_selector_all(".spanValorVerde").await?;
        anyhow::ensure!(
            spans.len() >= 4,
            "Expected at least 4 .spanValorVerde elements, found {}",
            spans.len()
        );

        // Validate that the CNPJ on the result page matches what was queried
        let cnpj_on_page = spans[0].text().await?.trim().to_string();
        let cnpj_on_page_digits = normalize_cnpj(&cnpj_on_page);
        if cnpj_on_page_digits != cnpj_digits {
            anyhow::bail!("CNPJ mismatch: queried {cnpj_digits} but result shows {cnpj_on_page}");
        }

        let nome_empresarial = spans[1].text().await?.trim().to_string();
        let situacao_simples = parse_situacao(&spans[2].text().await?);
        let situacao_simei = parse_situacao(&spans[3].text().await?);

        // 7. Click "Mais informações" to expand the details section (AJAX load)
        let mais_info_btn = dom.query_selector("#btnMaisInfo").await?;
        mais_info_btn.click().await?;

        // Wait for AJAX content to load into #maisInfo
        tokio::time::sleep(Duration::from_secs(3)).await;
        dom.invalidate();

        // 8. Extract periods and events from the expanded panels inside #maisInfo
        let mais_info = dom.query_selector("#maisInfo").await?;
        let panels = mais_info.query_selector_all(".panel.panel-success").await?;

        let mut periodos_simples = vec![];
        let mut periodos_simei = vec![];
        let mut eventos_futuros_simples = None;
        let mut eventos_futuros_simei = None;
        let mut mei_transportador = None;

        for panel in &panels {
            let title = panel
                .query_selector(".panel-title")
                .await?
                .text()
                .await?
                .trim()
                .to_string();
            let body = panel.query_selector(".panel-body").await?;

            match title.as_str() {
                "Períodos Anteriores" => {
                    let tables = body.query_selector_all("table").await?;
                    let body_text = body.text().await?;

                    if body_text.contains("Opções pelo Simples Nacional em Períodos Anteriores")
                        && !tables.is_empty()
                    {
                        periodos_simples = extract_table_rows(&tables[0]).await?;
                    }

                    if body_text.contains("Enquadramentos no SIMEI em Períodos Anteriores") {
                        if tables.len() >= 2 {
                            periodos_simei = extract_table_rows(&tables[1]).await?;
                        } else if tables.len() == 1
                            && !body_text
                                .contains("Opções pelo Simples Nacional em Períodos Anteriores")
                        {
                            periodos_simei = extract_table_rows(&tables[0]).await?;
                        }
                    }
                }
                "Eventos Futuros (Simples Nacional)" => {
                    eventos_futuros_simples = non_empty_text(&body).await?;
                }
                "Eventos Futuros (SIMEI)" => {
                    eventos_futuros_simei = non_empty_text(&body).await?;
                }
                t if t.contains("MEI Transportador") => {
                    mei_transportador = non_empty_text(&body).await?;
                }
                _ => {}
            }
        }

        Ok(ConsultaOptante {
            data_consulta,
            nome_empresarial,
            situacao_simples,
            situacao_simei,
            periodos_simples,
            periodos_simei,
            eventos_futuros_simples,
            eventos_futuros_simei,
            mei_transportador,
        })
    }
    .await;

    let _ = browser.close().await;
    let _ = process.wait().await;

    result
}

/// Extracts rows from a period table (skips header row).
async fn extract_table_rows(table: &chromium_driver::dom::Element) -> anyhow::Result<Vec<Periodo>> {
    let rows = table.query_selector_all("tr").await?;
    let mut periodos = vec![];

    for row in &rows {
        let cells = row.query_selector_all("td").await?;
        if cells.len() >= 3 {
            periodos.push(Periodo {
                data_inicial: parse_date_br(&cells[0].text().await?),
                data_final: parse_date_br(&cells[1].text().await?),
                detalhamento: cells[2].text().await?.trim().to_string(),
            });
        }
    }

    Ok(periodos)
}

/// Returns the text of a panel body, or None if it's empty or "Não Existem".
async fn non_empty_text(el: &chromium_driver::dom::Element) -> anyhow::Result<Option<String>> {
    let text = el.text().await?.trim().to_string();
    if text.is_empty() || text == "Não Existem" {
        Ok(None)
    } else {
        Ok(Some(text))
    }
}
