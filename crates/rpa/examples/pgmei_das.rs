//! Roda as rotas de produção do `rpa::pgmei` contra o portal real.
//!
//! Uso:
//!   pgmei_das consultar <CNPJ> <ANO>
//!   pgmei_das emitir <CNPJ> <YYYYMM>     # salva o PDF em /tmp

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;

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

    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [cmd, cnpj, ano] if cmd == "consultar" => {
            let meses = rpa::pgmei::consultar_das(cnpj, ano.parse()?).await?;
            for m in &meses {
                println!(
                    "{} {:<14} apurado={} situacao={:<12} ({}) total={:?} venc={:?} emissivel={}",
                    m.periodo,
                    m.competencia,
                    m.apurado,
                    m.situacao.as_str(),
                    m.situacao_texto,
                    m.total_cents,
                    m.vencimento,
                    m.emissivel,
                );
            }
        }
        [cmd, cnpj, anos] if cmd == "consultar-anos" => {
            // anos separados por vírgula, ex: "2024,2025,2026". Valida a
            // varredura multi-ano numa única sessão.
            let lista: Vec<i32> = anos
                .split(',')
                .filter_map(|a| a.trim().parse().ok())
                .collect();
            let resultados = rpa::pgmei::consultar_das_anos(cnpj, &lista).await?;
            for (ano, meses) in &resultados {
                use rpa::pgmei::SituacaoDas;
                let abertos: Vec<String> = meses
                    .iter()
                    .filter(|m| matches!(m.situacao, SituacaoDas::Devedor | SituacaoDas::EmAberto))
                    .map(|m| format!("{}={}", m.competencia, m.situacao.as_str()))
                    .collect();
                println!(
                    "=== {ano}: {} meses | em aberto ({}): {}",
                    meses.len(),
                    abertos.len(),
                    abertos.join(", ")
                );
            }
        }
        [cmd, cnpj, periodo] if cmd == "emitir" => {
            let guia = rpa::pgmei::emitir_das(cnpj, periodo).await?;
            println!("competencia:     {}", guia.competencia);
            println!("numero_das:      {}", guia.numero_das);
            println!("vencimento:      {:?}", guia.vencimento);
            println!("pagar_ate:       {:?}", guia.pagar_ate);
            println!("total_cents:     {:?}", guia.total_cents);
            println!("linha_digitavel: {:?}", guia.linha_digitavel);
            println!("pdf:             {} bytes", guia.pdf.len());
            let path = format!("/tmp/das-{}.pdf", guia.periodo);
            std::fs::write(&path, &guia.pdf)?;
            println!("pdf salvo em:    {path}");
            // sanity: o base64 do PDF é o que a tool MCP devolve inline
            let _ = BASE64_STANDARD.encode(&guia.pdf);
        }
        _ => {
            eprintln!("uso: pgmei_das consultar <CNPJ> <ANO> | emitir <CNPJ> <YYYYMM>");
            std::process::exit(2);
        }
    }
    Ok(())
}
