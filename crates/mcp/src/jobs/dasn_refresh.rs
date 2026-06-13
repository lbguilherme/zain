//! Worker de background que mantém o status da DASN-SIMEI (declaração
//! anual do MEI) dos clientes fresco, pra o `get_client_state` reportar
//! anos em atraso/pendentes como leitura SQL pura.
//!
//! Cadência **bem infrequente**: a DASN muda ~1x por ano (quando o cliente
//! declara, ou quando o portal adiciona o ano novo em janeiro). O
//! `refresh_dasn_status` agenda a próxima consulta de cada cliente pra 30
//! dias à frente no sucesso; o worker só acorda de tempos em tempos e pega
//! quem já passou de `dasn_proxima_tentativa_em`. O acesso é público por
//! CNPJ (sem gov.br) — a seleção filtra só CNPJ presente e lead não recusado.
//!
//! Knobs (env vars):
//! - `DASN_REFRESH_ENABLED`       — `false`/`0` desliga (default: ligado).
//! - `DASN_REFRESH_INTERVAL_SECS` — período entre ciclos (default: 21600 = 6h).
//! - `DASN_REFRESH_BATCH`         — máx. de clientes por ciclo (default: 10).

use std::sync::Arc;
use std::time::Duration;

use pgsafe::sql;

use crate::state::AppState;
use crate::tools::dasn;

const DEFAULT_INTERVAL_SECS: u64 = 21_600;
const DEFAULT_BATCH: i64 = 10;

fn env_num<T: std::str::FromStr>(key: &str, default: T) -> T {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// `true` a menos que `DASN_REFRESH_ENABLED` seja explicitamente `false`/`0`.
pub fn enabled() -> bool {
    match std::env::var("DASN_REFRESH_ENABLED") {
        Ok(v) => !matches!(v.trim().to_ascii_lowercase().as_str(), "false" | "0" | "no"),
        Err(_) => true,
    }
}

pub async fn run_forever(state: Arc<AppState>) {
    let interval_secs = env_num::<u64>("DASN_REFRESH_INTERVAL_SECS", DEFAULT_INTERVAL_SECS);
    let mut tick = tokio::time::interval(Duration::from_secs(interval_secs));
    tick.tick().await;
    tracing::info!(interval_secs, "dasn_refresh: worker iniciado");
    loop {
        tick.tick().await;
        if let Err(e) = run_once(&state).await {
            tracing::warn!(error = %crate::errlog::anyhow_chain(&e), "dasn_refresh: ciclo falhou");
        }
    }
}

async fn run_once(state: &AppState) -> anyhow::Result<()> {
    let batch = env_num::<i64>("DASN_REFRESH_BATCH", DEFAULT_BATCH);

    // Seleção governada por `dasn_proxima_tentativa_em`: NULL = nunca
    // consultado (entra já); senão, o sucesso agendou +30 dias e a falha
    // o backoff.
    let pendentes = sql!(
        &state.pool,
        "SELECT id, cnpj
         FROM zain.clients
         WHERE cnpj IS NOT NULL
           AND recusado_em IS NULL
           AND (dasn_proxima_tentativa_em IS NULL OR dasn_proxima_tentativa_em <= now())
         ORDER BY dasn_proxima_tentativa_em ASC NULLS FIRST
         LIMIT $batch"
    )
    .fetch_all()
    .await?;

    if pendentes.is_empty() {
        return Ok(());
    }
    tracing::info!(
        n = pendentes.len(),
        "dasn_refresh: processando clientes pendentes"
    );

    for row in &pendentes {
        let Some(cnpj) = row.cnpj.as_deref() else {
            continue;
        };
        let client_id = row.id;
        tracing::info!(%client_id, "dasn_refresh: atualizando status DASN");
        if let Err(e) = dasn::refresh_dasn_status(state, client_id, cnpj).await {
            tracing::warn!(%client_id, error = %crate::errlog::anyhow_chain(&e), "dasn_refresh: falha ao atualizar cliente");
        }
    }
    Ok(())
}

#[cfg(test)]
mod manual_tests {
    //! Harness manual contra o banco real + RPA no portal. Roda UM ciclo da
    //! cron e mostra o que ficou em `zain.dasn_anual`. Ignorado por default:
    //!
    //!   cargo test -p mcp jobs::dasn_refresh::manual_tests::run_once_real -- --ignored --nocapture
    use std::sync::Arc;

    use super::*;
    use crate::state::Models;

    fn build_state() -> AppState {
        dotenvy::dotenv_override().ok();
        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL não definido");
        let mut cfg = deadpool_postgres::Config::new();
        cfg.url = Some(database_url);
        let pool = cfg
            .create_pool(
                Some(deadpool_postgres::Runtime::Tokio1),
                tokio_postgres::NoTls,
            )
            .expect("criar pool");
        let ai = Arc::new(ai::Client::from_env());
        let models = Arc::new(Models::from_env().expect("Models::from_env"));
        AppState { pool, ai, models }
    }

    #[ignore]
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn run_once_real() {
        let _ = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .with_test_writer()
            .try_init();
        let state = build_state();
        run_once(&state).await.expect("run_once");

        let conn = state.pool.get().await.expect("conn");
        let rows = conn
            .query(
                "SELECT client_id, ano, entregue FROM zain.dasn_anual ORDER BY client_id, ano DESC",
                &[],
            )
            .await
            .expect("select dasn_anual");
        println!("\n===== zain.dasn_anual ({} linhas) =====", rows.len());
        for r in &rows {
            let ano: i32 = r.get("ano");
            let entregue: bool = r.get("entregue");
            println!("  {ano}  entregue={entregue}");
        }
    }

    /// Renderiza o `get_client_state` de um cliente (leitura pura, sem RPA)
    /// pra conferir o bloco DASN. Rode com:
    ///   CLIENT_ID=<uuid> cargo test -p mcp jobs::dasn_refresh::manual_tests::render_state -- --ignored --nocapture
    #[ignore]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn render_state() {
        let state = build_state();
        let client_id: uuid::Uuid = std::env::var("CLIENT_ID")
            .expect("set CLIENT_ID=<uuid>")
            .parse()
            .expect("CLIENT_ID uuid inválido");
        let r = crate::tools::get_client_state::run(
            &state,
            client_id,
            crate::tools::get_client_state::Args {},
        )
        .await;
        let v = serde_json::to_value(&r).unwrap_or_default();
        let text = v
            .get("content")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or("(sem texto)");
        println!("\n===== get_client_state =====\n{text}");
    }
}
