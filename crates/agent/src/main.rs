use std::sync::Arc;
use std::time::Duration;

use deadpool_postgres::{Config, Runtime};
use tokio::sync::Semaphore;
use tokio_postgres::NoTls;

use agent::dispatch;
use ollama::OllamaClient;

const MAX_CONCURRENT: usize = 5;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv_override().ok();
    tracing_subscriber::fmt::init();

    let database_url = std::env::var("DATABASE_URL").unwrap();
    let ollama_url = std::env::var("OLLAMA_URL").unwrap();
    let ollama_model = std::env::var("OLLAMA_MODEL").unwrap();

    let mut pool_cfg = Config::new();
    pool_cfg.url = Some(database_url);
    let pool = pool_cfg.create_pool(Some(Runtime::Tokio1), NoTls)?;

    let ollama = Arc::new(OllamaClient::new(&ollama_url));

    // Recovery: marcar execuções órfãs como crashed e re-agendar clientes
    dispatch::recover_crashed(&pool).await?;
    tracing::info!("Recovery completo. Iniciando dispatch loop...");

    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT));

    loop {
        let permit = semaphore.clone().acquire_owned().await?;

        match dispatch::claim_next_client(&pool).await {
            Ok(Some(client)) => {
                let pool = pool.clone();
                let ollama = ollama.clone();
                let model = ollama_model.clone();
                tokio::spawn(async move {
                    let _permit = permit;
                    let client_id = client.id;
                    let chat_id = client.chat_id.clone();
                    tracing::info!(%client_id, %chat_id, "Processando cliente");
                    if let Err(e) = dispatch::process_client(&pool, &ollama, &model, client).await {
                        tracing::error!(%client_id, "Erro processando cliente: {e:#}");
                    }
                });
            }
            Ok(None) => {
                drop(permit);
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
            Err(e) => {
                drop(permit);
                tracing::error!("Erro no claim: {e:#}");
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}
