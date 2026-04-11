use clap::{Parser, Subcommand};
use cubos_sql::sql;
use deadpool_postgres::{Config, Pool, Runtime};
use rpa::govbr::Nivel;
use rpa::govbr::session::SavedSession;
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
    /// Lê credenciais em `zain.clients` pelo `govbr_cpf`. Retorna nome,
    /// email, telefone e nível de segurança. Atualiza a sessão salva ao
    /// final. Este subcomando só existe para testes locais — o agent não
    /// passa pelo banco para invocar o fluxo.
    Govbr {
        /// CPF do gov.br cadastrado em `zain.clients.govbr_cpf`.
        cpf: String,
        /// Código OTP de 6 dígitos, se necessário para 2FA.
        #[arg(long)]
        otp: Option<String>,
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
        Command::Govbr { cpf, otp } => {
            let pool = connect_pool()?;
            let row = load_govbr(&pool, &cpf)
                .await?
                .ok_or_else(|| anyhow::anyhow!("nenhum cliente com govbr_cpf = {cpf}"))?;

            let outcome = rpa::govbr::check_govbr_profile(
                &row.cpf,
                &row.password,
                otp.as_deref(),
                row.session.as_ref(),
            )
            .await?;

            save_govbr(&pool, &row.cpf, &outcome).await?;

            let tag = if outcome.fresh_login {
                "fresh-login"
            } else {
                "reused-session"
            };
            eprintln!("→ {tag}");
            println!("{}", serde_json::to_string_pretty(&outcome.profile)?);
        }
        Command::Pgfn { documento } => {
            let result = rpa::pgfn::consultar_divida(&documento).await?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
    }

    Ok(())
}

fn connect_pool() -> anyhow::Result<Pool> {
    let database_url = std::env::var("DATABASE_URL")?;
    let mut cfg = Config::new();
    cfg.url = Some(database_url);
    Ok(cfg.create_pool(Some(Runtime::Tokio1), NoTls)?)
}

struct GovbrRow {
    cpf: String,
    password: String,
    session: Option<SavedSession>,
}

async fn load_govbr(pool: &Pool, cpf: &str) -> anyhow::Result<Option<GovbrRow>> {
    let row = sql!(
        pool,
        "SELECT govbr_cpf, govbr_password, govbr_session
         FROM zain.clients
         WHERE govbr_cpf = $cpf
         LIMIT 1"
    )
    .fetch_optional()
    .await?;

    let Some(r) = row else {
        return Ok(None);
    };

    Ok(Some(GovbrRow {
        cpf: r
            .govbr_cpf
            .ok_or_else(|| anyhow::anyhow!("govbr_cpf nulo"))?,
        password: r
            .govbr_password
            .ok_or_else(|| anyhow::anyhow!("govbr_password nulo"))?,
        session: r.govbr_session,
    }))
}

async fn save_govbr(
    pool: &Pool,
    cpf: &str,
    outcome: &rpa::govbr::CheckOutcome,
) -> anyhow::Result<()> {
    let session = outcome.session.clone();
    let nome: Option<&str> = Some(&outcome.profile.nome);
    let email = outcome.profile.email.as_deref();
    let telefone = outcome.profile.telefone.as_deref();
    let nivel: Option<Nivel> = outcome.profile.nivel;

    sql!(
        pool,
        "UPDATE zain.clients
         SET govbr_session          = $session!,
             govbr_session_valid_at = now(),
             govbr_nome              = $nome,
             govbr_email             = $email,
             govbr_telefone          = $telefone,
             govbr_nivel             = $nivel,
             updated_at              = now()
         WHERE govbr_cpf = $cpf"
    )
    .execute()
    .await?;
    Ok(())
}
