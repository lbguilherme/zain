use std::net::SocketAddr;
use std::sync::Arc;

use deadpool_postgres::{Config, Runtime};
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use tokio_postgres::NoTls;

mod client_state;
mod jobs;
mod meta;
mod resources;
mod server;
mod state;
mod tools;
mod validators;

use crate::server::ZainMcpServer;
use crate::state::{AppState, Models};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv_override().ok();
    tracing_subscriber::fmt::init();

    let database_url =
        std::env::var("DATABASE_URL").map_err(|_| anyhow::anyhow!("DATABASE_URL não definido"))?;

    let mut pool_cfg = Config::new();
    pool_cfg.url = Some(database_url);
    let pool = pool_cfg.create_pool(Some(Runtime::Tokio1), NoTls)?;

    let ai = Arc::new(ai::Client::from_env());
    let models = Arc::new(Models::from_env()?);
    let state = Arc::new(AppState { pool, ai, models });

    // Worker de background: mantém a situação MEI dos clientes fresca pra
    // o `get_client_state` ser leitura SQL pura. Desligável via
    // `MEI_REFRESH_ENABLED=false`.
    if jobs::mei_refresh::enabled() {
        let worker_state = state.clone();
        tokio::spawn(async move { jobs::mei_refresh::run_forever(worker_state).await });
    } else {
        tracing::info!("mei_refresh: worker desligado (MEI_REFRESH_ENABLED=false)");
    }

    let service_state = state.clone();
    // Stateless + json_response: sem `Mcp-Session-Id` obrigatório e
    // resposta `application/json` pura (sem SSE). Caller faz cada
    // `tools/call` independente, carregando `_meta.client_id` —
    // sessão MCP não acrescenta nada porque a identidade vem por
    // request, não por conexão.
    let service = StreamableHttpService::new(
        move || Ok(ZainMcpServer::new(service_state.clone())),
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default()
            .with_stateful_mode(false)
            .with_json_response(true)
            .disable_allowed_hosts(),
    );

    let addr: SocketAddr = std::env::var("MCP_LISTEN_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:8088".to_string())
        .parse()?;

    let app = axum::Router::new().fallback_service(service);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr, "servidor MCP escutando na raiz");

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.ok();
        })
        .await?;
    Ok(())
}
