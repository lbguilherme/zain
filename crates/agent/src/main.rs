use std::sync::Arc;
use std::time::Duration;

use deadpool_postgres::{Config, Runtime};
use tokio::signal::unix::{SignalKind, signal};
use tokio::sync::{Notify, Semaphore, watch};
use tokio_postgres::{AsyncMessage, NoTls};

use agent::dispatch;
use agent::dispatch::Models;

const MAX_CONCURRENT: usize = 5;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv_override().ok();
    tracing_subscriber::fmt::init();

    let database_url = std::env::var("DATABASE_URL").unwrap();
    let models = Arc::new(Models::from_env()?);

    let mut pool_cfg = Config::new();
    pool_cfg.url = Some(database_url.clone());
    let pool = pool_cfg.create_pool(Some(Runtime::Tokio1), NoTls)?;

    let ai_client = Arc::new(ai::Client::from_env());

    // Recovery: marcar execuções órfãs como crashed e re-agendar clientes
    dispatch::recover_crashed(&pool).await?;
    tracing::info!("Iniciando dispatch loop...");

    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT));

    // Notify compartilhado entre o listener e o loop principal.
    let notify = Arc::new(Notify::new());

    // Sinal de shutdown disparado por SIGTERM/SIGINT.
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    tokio::spawn(shutdown_signal(shutdown_tx));

    // Task dedicada de LISTEN em uma conexão fora do pool. Reconecta em loop
    // com backoff se a conexão cair.
    let listen_notify = notify.clone();
    let listen_url = database_url.clone();
    let mut listen_shutdown = shutdown_rx.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                biased;
                _ = listen_shutdown.wait_for(|v| *v) => return,
                r = listen_task(&listen_url, &listen_notify) => {
                    if let Err(e) = r {
                        tracing::error!("Listen task caiu: {e:#}. Reconectando em 5s...");
                    }
                }
            }
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(5)) => {}
                _ = listen_shutdown.wait_for(|v| *v) => return,
            }
        }
    });

    loop {
        let permit = {
            let mut sd = shutdown_rx.clone();
            tokio::select! {
                biased;
                _ = sd.wait_for(|v| *v) => break,
                r = semaphore.clone().acquire_owned() => r?,
            }
        };

        match dispatch::claim_next_client(&pool).await {
            Ok(Some(client)) => {
                let pool = pool.clone();
                let ai_client = ai_client.clone();
                let models = models.clone();
                tokio::spawn(async move {
                    let _permit = permit;
                    let client_id = client.id;
                    let chat_id = client.chat_id.clone();
                    tracing::info!(%client_id, %chat_id, "Processando cliente");
                    if let Err(e) =
                        dispatch::process_client(&pool, &ai_client, &models, client).await
                    {
                        tracing::error!(%client_id, "Erro processando cliente: {e:#}");
                    }
                });
            }
            Ok(None) => {
                drop(permit);
                let mut sd = shutdown_rx.clone();
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_secs(20)) => {}
                    _ = notify.notified() => {}
                    _ = sd.wait_for(|v| *v) => break,
                }
            }
            Err(e) => {
                drop(permit);
                tracing::error!("Erro no claim: {e:#}");
                let mut sd = shutdown_rx.clone();
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_secs(5)) => {}
                    _ = sd.wait_for(|v| *v) => break,
                }
            }
        }
    }

    tracing::info!("Shutdown solicitado: aguardando execuções em andamento...");
    let _all = semaphore
        .acquire_many(MAX_CONCURRENT as u32)
        .await
        .expect("semaphore fechado inesperadamente");
    tracing::info!("Graceful shutdown concluído.");
    Ok(())
}

async fn shutdown_signal(tx: watch::Sender<bool>) {
    let mut sigterm = match signal(SignalKind::terminate()) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Falha ao instalar handler SIGTERM: {e}");
            return;
        }
    };
    let mut sigint = match signal(SignalKind::interrupt()) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Falha ao instalar handler SIGINT: {e}");
            return;
        }
    };
    tokio::select! {
        _ = sigterm.recv() => tracing::info!("SIGTERM recebido, iniciando graceful shutdown..."),
        _ = sigint.recv() => tracing::info!("SIGINT recebido, iniciando graceful shutdown..."),
    }
    let _ = tx.send(true);
}

async fn listen_task(database_url: &str, notify: &Arc<Notify>) -> anyhow::Result<()> {
    let (client, connection) = tokio_postgres::connect(database_url, NoTls).await?;

    // Canal para repassar notifications do driver da conexão para o loop.
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<()>();

    // Sub-task que dirige a Connection dedicada e extrai AsyncMessages.
    let driver = tokio::spawn(async move {
        tokio::pin!(connection);
        loop {
            let msg = std::future::poll_fn(|cx| connection.as_mut().poll_message(cx)).await;
            match msg {
                Some(Ok(AsyncMessage::Notification(_))) => {
                    if tx.send(()).is_err() {
                        break;
                    }
                }
                Some(Ok(_)) => {}
                Some(Err(e)) => {
                    tracing::error!("Erro na conexão LISTEN: {e}");
                    break;
                }
                None => break,
            }
        }
    });

    client
        .batch_execute("LISTEN zain_clients_needs_processing")
        .await?;

    while rx.recv().await.is_some() {
        notify.notify_one();
    }

    driver.abort();
    anyhow::bail!("conexão LISTEN encerrou");
}
