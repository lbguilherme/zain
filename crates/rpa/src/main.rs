use clap::{Parser, Subcommand};
use deadpool_postgres::{Config, Runtime};
use tokio_postgres::NoTls;

#[derive(Parser)]
#[command(name = "rpa", about = "Automações de navegador (SIMEI, gov.br, ...)")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Consulta optantes do Simples Nacional / SIMEI pelo CNPJ.
    Mei {
        /// CNPJ (somente dígitos ou formatado).
        cnpj: String,
    },
    /// Verifica / loga no gov.br e extrai dados do perfil.
    ///
    /// Lê credenciais de `zain.govbr`. Retorna nome, email, telefone e
    /// nível de segurança. Atualiza a sessão salva ao final.
    Govbr {
        /// CPF (somente dígitos).
        cpf: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv_override().ok();
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Command::Mei { cnpj } => {
            let result = rpa::mei::consultar_optante(&cnpj).await?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::Govbr { cpf } => {
            let database_url = std::env::var("DATABASE_URL")?;
            let mut cfg = Config::new();
            cfg.url = Some(database_url);
            let pool = cfg.create_pool(Some(Runtime::Tokio1), NoTls)?;

            let outcome = rpa::govbr::check_govbr_profile(&pool, &cpf).await?;
            let tag = match &outcome {
                rpa::govbr::CheckOutcome::Reused(_) => "reused-session",
                rpa::govbr::CheckOutcome::LoggedIn(_) => "fresh-login",
            };
            eprintln!("→ {tag}");
            println!("{}", serde_json::to_string_pretty(outcome.profile())?);
        }
    }

    Ok(())
}
