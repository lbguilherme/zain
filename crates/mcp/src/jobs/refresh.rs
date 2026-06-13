//! Worker de background ÚNICO que mantém o estado dos clientes fresco.
//!
//! Em vez de uma cron por tarefa (MEI, DAS, DASN), há um só worker que
//! acorda a cada poucos minutos, acha **qualquer cliente com qualquer
//! refresh pendente** (a fila é as próprias colunas `*_proxima_tentativa_em`
//! — cada `refresh_*_status` reagenda a sua, ver "Cadência das crons" no
//! FLUXOS.md) e despacha pro refresh certo. Sequencial: um browser por vez
//! nesta box.
//!
//! Knobs (env vars):
//! - `REFRESH_ENABLED`        — `false`/`0` desliga o worker (default: ligado).
//! - `REFRESH_INTERVAL_SECS`  — período entre ciclos (default: 300 = 5 min).
//! - `REFRESH_BATCH`          — máx. de tarefas por ciclo (default: 5).
//! - `REFRESH_SKIP_KINDS`     — tipos a pular, separados por vírgula (ex.:
//!   `mei` pra desligar só o gov.br numa instabilidade). Default: nenhum.

use std::sync::Arc;
use std::time::Duration;

use pgsafe::sql;

use crate::state::AppState;
use crate::tools::{das, dasn, govbr};

const DEFAULT_INTERVAL_SECS: u64 = 300;
const DEFAULT_BATCH: i64 = 5;

fn env_num<T: std::str::FromStr>(key: &str, default: T) -> T {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// `true` a menos que `REFRESH_ENABLED` seja explicitamente `false`/`0`.
pub fn enabled() -> bool {
    match std::env::var("REFRESH_ENABLED") {
        Ok(v) => !matches!(v.trim().to_ascii_lowercase().as_str(), "false" | "0" | "no"),
        Err(_) => true,
    }
}

/// `(incluir_mei, incluir_das, incluir_dasn)` a partir de `REFRESH_SKIP_KINDS`.
fn kinds_habilitados() -> (bool, bool, bool) {
    let skip = std::env::var("REFRESH_SKIP_KINDS").unwrap_or_default();
    let skip = |k: &str| skip.split(',').any(|s| s.trim().eq_ignore_ascii_case(k));
    (!skip("mei"), !skip("das"), !skip("dasn"))
}

pub async fn run_forever(state: Arc<AppState>) {
    let interval_secs = env_num::<u64>("REFRESH_INTERVAL_SECS", DEFAULT_INTERVAL_SECS);
    let mut tick = tokio::time::interval(Duration::from_secs(interval_secs));
    // Consome o 1º tick (imediato) pra não subir browser no boot.
    tick.tick().await;
    tracing::info!(interval_secs, "refresh: worker único iniciado");
    loop {
        tick.tick().await;
        if let Err(e) = run_once(&state).await {
            tracing::warn!(error = %crate::errlog::anyhow_chain(&e), "refresh: ciclo falhou");
        }
    }
}

/// Um ciclo: seleciona as tarefas mais vencidas (de qualquer tipo) e executa
/// em sequência. Cada `refresh_*_status` reagenda o próprio
/// `*_proxima_tentativa_em` (cadência no sucesso, backoff na falha).
async fn run_once(state: &AppState) -> anyhow::Result<()> {
    let batch = env_num::<i64>("REFRESH_BATCH", DEFAULT_BATCH);
    let (incluir_mei, incluir_das, incluir_dasn) = kinds_habilitados();

    // "Qualquer cliente com qualquer refresh pendente", do mais vencido pro
    // menos. Filtros por tipo: mei exige credencial gov.br utilizável (sem
    // otp pendente); das/dasn exigem CNPJ. NULL em proxima_tentativa = nunca
    // consultado (entra já). Cada `$incluir_*` desliga um tipo via env.
    let pendentes = sql!(
        &state.pool,
        "SELECT id, kind, cpf, cnpj, due FROM (
            SELECT id, 'mei' AS kind, cpf, cnpj, mei_proxima_tentativa_em AS due
            FROM zain.clients
            WHERE $incluir_mei
              AND cpf IS NOT NULL AND recusado_em IS NULL
              AND govbr_cpf IS NOT NULL AND govbr_password IS NOT NULL
              AND govbr_otp_pendente = false
              AND (mei_proxima_tentativa_em IS NULL OR mei_proxima_tentativa_em <= now())
            UNION ALL
            SELECT id, 'das' AS kind, cpf, cnpj, das_proxima_tentativa_em AS due
            FROM zain.clients
            WHERE $incluir_das
              AND cnpj IS NOT NULL AND recusado_em IS NULL
              AND (das_proxima_tentativa_em IS NULL OR das_proxima_tentativa_em <= now())
            UNION ALL
            SELECT id, 'dasn' AS kind, cpf, cnpj, dasn_proxima_tentativa_em AS due
            FROM zain.clients
            WHERE $incluir_dasn
              AND cnpj IS NOT NULL AND recusado_em IS NULL
              AND (dasn_proxima_tentativa_em IS NULL OR dasn_proxima_tentativa_em <= now())
         ) AS fila
         ORDER BY due ASC NULLS FIRST
         LIMIT $batch"
    )
    .fetch_all()
    .await?;

    if pendentes.is_empty() {
        return Ok(());
    }
    tracing::info!(
        n = pendentes.len(),
        "refresh: processando tarefas pendentes"
    );

    for row in &pendentes {
        let client_id = row.id;
        match row.kind.as_str() {
            "mei" => {
                let Some(cpf) = row.cpf.as_deref() else {
                    continue;
                };
                tracing::info!(%client_id, "refresh: MEI (gov.br)");
                // refresh_mei_status engole/loga o próprio erro e reagenda.
                let _ = govbr::refresh_mei_status(state, client_id, cpf, None).await;
            }
            "das" => {
                let Some(cnpj) = row.cnpj.as_deref() else {
                    continue;
                };
                tracing::info!(%client_id, "refresh: DAS");
                if let Err(e) = das::refresh_das_status(state, client_id, cnpj).await {
                    tracing::warn!(%client_id, error = %crate::errlog::anyhow_chain(&e), "refresh: falha no DAS");
                }
            }
            "dasn" => {
                let Some(cnpj) = row.cnpj.as_deref() else {
                    continue;
                };
                tracing::info!(%client_id, "refresh: DASN");
                if let Err(e) = dasn::refresh_dasn_status(state, client_id, cnpj).await {
                    tracing::warn!(%client_id, error = %crate::errlog::anyhow_chain(&e), "refresh: falha na DASN");
                }
            }
            other => {
                tracing::warn!(kind = other, "refresh: tipo desconhecido na fila");
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod manual_tests {
    //! Harness manual contra o banco real (`DATABASE_URL`). Roda UM ciclo da
    //! cron única (RPA real) e mostra o `get_client_state` de um cliente.
    //! Ignorados por default; rode com:
    //!
    //!   cargo test -p mcp jobs::refresh::manual_tests::run_once_real -- --ignored --nocapture
    //!   CLIENT_ID=<uuid> cargo test -p mcp jobs::refresh::manual_tests::render_state -- --ignored --nocapture
    use std::sync::Arc;

    use super::*;
    use crate::state::Models;
    use crate::tools::get_client_state;

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
    }

    #[ignore]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn render_state() {
        let state = build_state();
        let client_id: uuid::Uuid = std::env::var("CLIENT_ID")
            .expect("set CLIENT_ID=<uuid>")
            .parse()
            .expect("CLIENT_ID uuid inválido");
        let r = get_client_state::run(&state, client_id, get_client_state::Args {}).await;
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
