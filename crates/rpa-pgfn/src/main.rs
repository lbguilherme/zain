use clap::Parser;

#[derive(Parser)]
#[command(
    name = "rpa-pgfn",
    about = "Consulta dívida ativa na lista de devedores da PGFN"
)]
struct Cli {
    /// CPF (11 dígitos) ou CNPJ (14 dígitos), com ou sem formatação
    documento: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();
    let result = rpa_pgfn::consulta::consultar_divida(&cli.documento).await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
