use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;

use cubos_sql::sql;
use deadpool_postgres::Pool;
use http_body_util::BodyExt;
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

pub async fn webhook_server(pool: Arc<Pool>, addr: SocketAddr) -> anyhow::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    tracing::info!(%addr, "Webhook server listening");

    loop {
        let (stream, remote) = listener.accept().await?;
        let pool = pool.clone();

        tokio::spawn(async move {
            let io = TokioIo::new(stream);
            let pool = pool.clone();

            let service = service_fn(move |req| {
                let pool = pool.clone();
                async move { handle_request(req, &pool).await }
            });

            if let Err(e) = http1::Builder::new().serve_connection(io, service).await {
                tracing::error!(%remote, "HTTP error: {e}");
            }
        });
    }
}

async fn handle_request(
    req: Request<Incoming>,
    pool: &Pool,
) -> Result<Response<String>, Infallible> {
    if req.method() != Method::POST {
        return Ok(Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .body("POST only".into())
            .unwrap());
    }

    let body_bytes = match req.into_body().collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            tracing::error!("Erro lendo body: {e}");
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body("bad request".into())
                .unwrap());
        }
    };

    let body: serde_json::Value = match serde_json::from_slice(&body_bytes) {
        Ok(v) => v,
        Err(_) => {
            // Se não for JSON válido, salva como string raw
            serde_json::Value::String(String::from_utf8_lossy(&body_bytes).into_owned())
        }
    };

    match save_webhook(pool, &body).await {
        Ok(id) => {
            tracing::info!(id, "Webhook recebido");
            Ok(Response::new(format!("{{\"ok\":true,\"id\":{id}}}")))
        }
        Err(e) => {
            tracing::error!("Erro salvando webhook: {e:#}");
            Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body("internal error".into())
                .unwrap())
        }
    }
}

async fn save_webhook(pool: &Pool, body: &serde_json::Value) -> anyhow::Result<i64> {
    let body = body.clone();

    let row = sql!(
        pool,
        "INSERT INTO whatsapp.webhooks (body)
         VALUES ($body)
         RETURNING id"
    )
    .fetch_one()
    .await?;

    Ok(row.id)
}
