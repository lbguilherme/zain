use clap::{Parser, Subcommand};
use deadpool_postgres::{Config, Runtime};
use tokio_postgres::NoTls;
use uuid::Uuid;

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
    /// Lê credenciais de `zain.govbr` pelo `client_id`. Retorna nome,
    /// email, telefone e nível de segurança. Atualiza a sessão salva ao
    /// final.
    Govbr {
        /// ID do cliente em `zain.clients` (UUID).
        client_id: Uuid,
    },
    /// Consulta dívida ativa na lista de devedores da PGFN.
    Pgfn {
        /// CPF (11 dígitos) ou CNPJ (14 dígitos), com ou sem formatação.
        documento: String,
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
        Command::Govbr { client_id } => {
            let database_url = std::env::var("DATABASE_URL")?;
            let mut cfg = Config::new();
            cfg.url = Some(database_url);
            let pool = cfg.create_pool(Some(Runtime::Tokio1), NoTls)?;

            let outcome = rpa::govbr::check_govbr_profile(&pool, client_id).await?;
            let tag = match &outcome {
                rpa::govbr::CheckOutcome::Reused(_) => "reused-session",
                rpa::govbr::CheckOutcome::LoggedIn(_) => "fresh-login",
            };
            eprintln!("→ {tag}");
            println!("{}", serde_json::to_string_pretty(outcome.profile())?);
        }
        Command::Pgfn { documento } => {
            let result = rpa::pgfn::consultar_divida(&documento).await?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
    }

    Ok(())
}
