mod db;
mod download;
mod import;
mod schema;
mod source;
mod sources;

use anyhow::Result;
use clap::Parser;

use source::DataSource;
use sources::cnae::CnaeSource;
use sources::cnpj::CnpjSource;
use sources::mei_cnaes::MeiCnaesSource;
use sources::pgfn::PgfnSource;

#[derive(Parser)]
#[command(
    name = "dados-abertos",
    about = "Sincroniza dados abertos do governo para PostgreSQL"
)]
struct Cli {
    /// Pular etapa de download
    #[arg(long)]
    skip_download: bool,

    /// Sincronizar apenas este schema (ex: cnpj, pgfn)
    #[arg(long)]
    only: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv_override().ok();
    let cli = Cli::parse();

    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://localhost/pjtei".into());

    println!("=== Conectando ao banco de dados ===");
    let mut client = db::connect(&database_url).await?;
    println!("  Conectado em {database_url}");

    let sources: Vec<Box<dyn DataSource>> = vec![
        Box::new(CnpjSource),
        Box::new(PgfnSource),
        Box::new(CnaeSource),
        Box::new(MeiCnaesSource),
    ];

    for source in &sources {
        let name = source.schema_name();

        if let Some(ref only) = cli.only
            && only != name
        {
            continue;
        }

        let installed = db::read_schema_version(&client, name).await?;

        if !source.needs_update(installed.as_ref()) {
            println!(
                "{name}: up to date (dados={}, extrator=v{})",
                source.data_version(),
                source.extractor_version()
            );
            continue;
        }

        match &installed {
            Some(v) => println!(
                "=== Atualizando {name} (dados: {} → {}, extrator: v{} → v{}) ===",
                v.data_version,
                source.data_version(),
                v.extractor_version,
                source.extractor_version()
            ),
            None => println!(
                "=== Instalando {name} (dados={}, extrator=v{}) ===",
                source.data_version(),
                source.extractor_version()
            ),
        }

        // Download
        if !cli.skip_download {
            let downloads = source.downloads();
            if !downloads.is_empty() {
                let data_dir = source.data_dir();
                println!("  Download → {}", data_dir.display());
                download::download_all(&downloads, &data_dir).await?;
            }
        }

        // Import numa transaction
        let tables = source.tables();

        let tx = client.transaction().await?;

        println!("  Criando schema temporário...");
        let temp = db::create_temp_schema(&tx, tables, source.setup_ddl()).await?;

        println!("  Importando dados...");
        source.import_data(&tx, &temp).await?;

        println!("  Criando índices...");
        db::create_indexes(&tx, &temp, tables).await?;

        println!("  Substituindo schema {name}...");
        db::swap_schemas(&tx, &temp, name, tables, &source.current_version()).await?;

        tx.commit().await?;
        println!("  {name}: OK");
    }

    println!("=== Concluído! ===");
    Ok(())
}
