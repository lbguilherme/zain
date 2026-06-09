use std::time::Duration;

use super::{ConsultaDivida, PGFN_URL};
use crate::sanity;

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
        page.wait_for_load(timeout).await.ok();
        sanity::checkpoint(&page, "pgfn: página carregada").await;

        let dom = page.dom().await?;

        // 2. Aguardar o formulário carregar (Angular bootstrap)
        let input = sanity::wait_for(
            &page,
            dom,
            "pgfn: aguardar form de consulta",
            "#identificacaoInput",
            timeout,
        )
        .await?;

        // 3. Preencher o CPF/CNPJ
        input.click().await?;
        input.type_text(&doc_digits).await?;

        // 4. Aguardar o botão CONSULTAR ficar habilitado (o Angular habilita
        //    após validar o input). Polling COM teto de tempo — antes era um
        //    `loop` infinito que travaria se o botão nunca habilitasse.
        let deadline = tokio::time::Instant::now() + timeout;
        let btn = loop {
            let btn = dom.query_selector("button.btn-warning").await?;
            if btn.attribute("disabled").await?.is_none() {
                break btn;
            }
            sanity::tick(
                &page,
                "pgfn: aguardar botão CONSULTAR habilitar",
                deadline,
                Duration::from_millis(500),
            )
            .await?;
        };

        // 5. Scrollar o botão para dentro da viewport e clicar CONSULTAR
        let btn_js = btn.resolve().await?;
        btn_js
            .eval("function() { this.scrollIntoView({block: 'center', behavior: 'instant'}); }")
            .await?;
        tokio::time::sleep(Duration::from_millis(300)).await;
        btn.click().await?;

        // 6. Aguardar resultado — polling por p.total-mensagens (aparece tanto
        //    no "Nenhum registro" quanto no resultado). Com teto de tempo.
        let deadline = tokio::time::Instant::now() + timeout;
        let result_text = loop {
            dom.invalidate();
            if let Some(msg_el) = dom.try_query_selector("p.total-mensagens").await? {
                let text = msg_el.text().await?.trim().to_string();
                if !text.is_empty() {
                    break text;
                }
            }
            sanity::tick(
                &page,
                "pgfn: aguardar resultado da consulta",
                deadline,
                Duration::from_secs(1),
            )
            .await?;
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

        // 8. Tem resultado — extrair dados da tabela dentro do virtual scroll.
        //    Espera o JS do virtual scroll renderizar ao menos uma row (sinal
        //    real) em vez de um `sleep(1s)` cego. Bounded pelo timeout; se
        //    estourar, o check de `rows.is_empty()` abaixo aborta com contexto.
        page.wait_for_function(
            "document.querySelectorAll('cdk-virtual-scroll-viewport tbody tr').length > 0",
            timeout,
        )
        .await
        .ok();
        dom.invalidate();

        let rows = dom
            .query_selector_all("cdk-virtual-scroll-viewport tbody tr")
            .await?;

        // A página indicou resultado (não foi "Nenhum registro"), mas a tabela
        // veio vazia — estado inesperado (não renderizou? layout mudou?). NÃO
        // tratamos como "sem dívida" pra não gerar falso negativo perigoso;
        // abortamos com contexto (e o cache PGFN não grava o resultado errado).
        if rows.is_empty() {
            return Err(sanity::fail(
                &page,
                "pgfn: extrair tabela de devedores",
                "página indicou resultado mas a tabela de devedores veio vazia",
            )
            .await);
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

        // Havia linhas, mas nenhuma casou os seletores de nome/valor —
        // provável mudança de layout do portal. Loga pra investigar (sem
        // abortar: `tem_divida=true` segue correto, a tabela tinha registros).
        if total_divida == 0.0 && primeiro_nome.is_none() {
            let (url, _) = sanity::page_where(&page).await;
            tracing::warn!(
                rows = rows.len(),
                %url,
                "pgfn: tabela tinha linhas mas nenhum td.tamanho-maximo-nome/td.text-end casou — layout pode ter mudado"
            );
            let _ = page.debug_dump("rpa-pgfn-extracao-sem-campos").await;
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
