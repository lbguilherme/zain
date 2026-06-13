use std::net::SocketAddr;
use std::sync::Arc;

use deadpool_postgres::{Config, Runtime};
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use tokio_postgres::NoTls;

mod client_state;
mod errlog;
mod jobs;
mod meta;
mod server;
mod state;
mod tools;
mod validators;

use crate::server::ZainMcpServer;
use crate::state::{AppState, Models};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv_override().ok();
    // Default explícito de INFO quando `RUST_LOG` não está setado (e honra
    // a env quando está). NÃO use `fmt::init()`: a feature `env-filter` é
    // unificada no binário via `rpa`/`chromium-driver`, e nesse caminho o
    // `fmt::init()` cai num `EnvFilter::from_default_env()` cuja diretiva
    // default é ERROR — o que silenciou todo INFO/WARN em produção quando os
    // exemplos de debug puxaram `env-filter` pra árvore de dependências.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let database_url =
        std::env::var("DATABASE_URL").map_err(|_| anyhow::anyhow!("DATABASE_URL não definido"))?;

    let mut pool_cfg = Config::new();
    pool_cfg.url = Some(database_url);
    let pool = pool_cfg.create_pool(Some(Runtime::Tokio1), NoTls)?;

    let ai = Arc::new(ai::Client::from_env());
    let models = Arc::new(Models::from_env()?);
    let state = Arc::new(AppState { pool, ai, models });

    // Worker de background ÚNICO: mantém MEI/DAS/DASN frescos pra o
    // `get_client_state` ser leitura SQL pura. Acorda a cada poucos minutos
    // e pega qualquer cliente com qualquer refresh pendente. Desligável via
    // `REFRESH_ENABLED=false`.
    if jobs::refresh::enabled() {
        let worker_state = state.clone();
        tokio::spawn(async move { jobs::refresh::run_forever(worker_state).await });
    } else {
        tracing::info!("refresh: worker desligado (REFRESH_ENABLED=false)");
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
