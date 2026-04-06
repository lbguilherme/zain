use std::sync::Arc;

use deadpool_postgres::{Config, Runtime};
use tokio_postgres::NoTls;

use whatsapp::client::WhapiClient;
use whatsapp::outbox;
use whatsapp::webhook;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv_override().ok();
    tracing_subscriber::fmt::init();

    let database_url = std::env::var("DATABASE_URL").unwrap();
    let whapi_token = std::env::var("WHAPI_TOKEN").expect("WHAPI_TOKEN não definido");
    let whapi_base_url = std::env::var("WHAPI_BASE_URL").unwrap();
    let webhook_port: u16 = std::env::var("WEBHOOK_PORT")
        .unwrap_or_else(|_| "3100".into())
        .parse()?;

    let mut pool_cfg = Config::new();
    pool_cfg.url = Some(database_url);
    let pool = Arc::new(pool_cfg.create_pool(Some(Runtime::Tokio1), NoTls)?);

    let api = WhapiClient::new(&whapi_base_url, &whapi_token);
    let addr = ([0, 0, 0, 0], webhook_port).into();

    tracing::info!("Iniciando webhook server + outbox loop...");

    tokio::select! {
        r = webhook::webhook_server(pool.clone(), addr) => {
            tracing::error!("webhook_server terminou: {r:?}");
            r
        }
        r = outbox::outbox_loop(&pool, &api) => {
            tracing::error!("outbox_loop terminou: {r:?}");
            r
        }
    }
}
