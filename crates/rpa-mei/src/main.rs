use clap::Parser;

#[derive(Parser)]
#[command(
    name = "rpa-mei",
    about = "Consulta optantes do Simples Nacional / SIMEI"
)]
struct Cli {
    /// CNPJ to query (digits only or formatted)
    cnpj: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    let result = rpa_mei::consulta::consultar_optante(&cli.cnpj).await?;

    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
