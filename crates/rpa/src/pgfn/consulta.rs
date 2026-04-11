use std::time::Duration;

use super::{ConsultaDivida, PGFN_URL};

/// Normaliza CPF/CNPJ, mantendo apenas dígitos.
fn normalize_documento(doc: &str) -> String {
    doc.chars().filter(|c| c.is_ascii_digit()).collect()
}

/// Converte valor no formato brasileiro ("1.753.673.631,00") para f64.
fn parse_valor_br(s: &str) -> f64 {
    s.trim()
        .replace('.', "")
        .replace(',', ".")
        .parse::<f64>()
        .unwrap_or(0.0)
}

/// Consulta se um CPF ou CNPJ possui dívida ativa na lista de devedores da PGFN.
pub async fn consultar_divida(documento: &str) -> anyhow::Result<ConsultaDivida> {
    let doc_digits = normalize_documento(documento);

    anyhow::ensure!(
        doc_digits.len() == 11 || doc_digits.len() == 14,
        "Documento deve ter 11 dígitos (CPF) ou 14 dígitos (CNPJ), recebeu {}",
        doc_digits.len()
    );

    let timeout = Duration::from_secs(30);

    let (mut process, browser) =
        chromium_driver::launch(chromium_driver::LaunchOptions::default()).await?;

    let page = browser.create_page("about:blank").await?.attach().await?;
    page.enable().await?;

    let result = async {
        // 1. Navegar para a página da PGFN
        page.navigate(PGFN_URL).await?;
        page.wait_for_load(timeout).await?;

        let dom = page.dom().await?;

        // 2. Aguardar o formulário carregar (Angular bootstrap)
        let input = dom.wait_for("#identificacaoInput", timeout).await?;

        // 3. Preencher o CPF/CNPJ
        input.click().await?;
        input.type_text(&doc_digits).await?;

        // 4. Aguardar o botão CONSULTAR ficar habilitado
        //    O Angular habilita o botão após validar o input.
        //    Polling até o botão não ter o atributo "disabled".
        let btn = loop {
            tokio::time::sleep(Duration::from_millis(500)).await;
            let btn = dom.query_selector("button.btn-warning").await?;
            let disabled = btn.attribute("disabled").await?;
            if disabled.is_none() {
                break btn;
            }
        };

        // 5. Scrollar o botão para dentro da viewport e clicar CONSULTAR
        let btn_js = btn.resolve().await?;
        btn_js
            .eval("function() { this.scrollIntoView({block: 'center', behavior: 'instant'}); }")
            .await?;
        tokio::time::sleep(Duration::from_millis(300)).await;
        btn.click().await?;

        // 6. Aguardar resultado — polling por .total-mensagens.info-panel
        //    Aparece tanto no caso de "nenhum registro" quanto no caso de resultado.
        let result_text = loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            dom.invalidate();
            if let Some(msg_el) = dom.try_query_selector("p.total-mensagens").await? {
                let text = msg_el.text().await?;
                let text = text.trim().to_string();
                if !text.is_empty() {
                    break text;
                }
            }
        };

        // 7. Interpretar resultado
        if result_text.contains("Nenhum registro") {
            return Ok(ConsultaDivida {
                documento: doc_digits.clone(),
                tem_divida: false,
                total_divida: 0.0,
                nome: None,
            });
        }

        // 8. Tem resultado — extrair dados da tabela dentro do virtual scroll
        //    Aguardar um pouco para o virtual scroll renderizar as rows.
        tokio::time::sleep(Duration::from_secs(1)).await;
        dom.invalidate();

        let rows = dom
            .query_selector_all("cdk-virtual-scroll-viewport tbody tr")
            .await?;

        if rows.is_empty() {
            return Ok(ConsultaDivida {
                documento: doc_digits.clone(),
                tem_divida: false,
                total_divida: 0.0,
                nome: None,
            });
        }

        let mut total_divida = 0.0;
        let mut primeiro_nome: Option<String> = None;

        for row in &rows {
            // Nome: td com classe tamanho-maximo-nome, atributo title
            if let Some(nome_td) = row.try_query_selector("td.tamanho-maximo-nome").await?
                && primeiro_nome.is_none()
                && let Some(title) = nome_td.attribute("title").await?
            {
                let title = title.trim().to_string();
                if !title.is_empty() {
                    primeiro_nome = Some(title);
                }
            }

            // Valor: td com classe text-end
            if let Some(valor_td) = row.try_query_selector("td.text-end").await? {
                let valor_text = valor_td.text().await?;
                total_divida += parse_valor_br(&valor_text);
            }
        }

        Ok(ConsultaDivida {
            documento: doc_digits.clone(),
            tem_divida: true,
            total_divida,
            nome: primeiro_nome,
        })
    }
    .await;

    let _ = browser.close().await;
    let _ = process.wait().await;

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valor_br() {
        assert_eq!(parse_valor_br(" 1.753.673.631,00 "), 1_753_673_631.0);
        assert_eq!(parse_valor_br("48.764.425.173,71"), 48_764_425_173.71);
        assert_eq!(parse_valor_br("15.000,00"), 15_000.0);
        assert_eq!(parse_valor_br("0,00"), 0.0);
        assert_eq!(parse_valor_br(""), 0.0);
    }

    #[test]
    fn test_normalize_documento() {
        assert_eq!(normalize_documento("123.456.789-00"), "12345678900");
        assert_eq!(normalize_documento("33.412.081/0001-96"), "33412081000196");
        assert_eq!(normalize_documento("12345678900"), "12345678900");
    }
}
