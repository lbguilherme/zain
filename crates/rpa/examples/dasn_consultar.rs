//! Roda a rota de produção `rpa::dasn::consultar_dasn` contra o portal.
//! Uso: `dasn_consultar <CNPJ>`

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

    let cnpj = std::env::args().nth(1).expect("uso: dasn_consultar <CNPJ>");
    let anos = rpa::dasn::consultar_dasn(&cnpj).await?;
    for a in &anos {
        println!(
            "{} | entregue={} | tipo={} | situacao_especial={:?}",
            a.ano, a.entregue, a.tipo, a.situacao_especial
        );
    }
    Ok(())
}
