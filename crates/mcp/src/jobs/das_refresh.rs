//! Worker de background que mantém a situação DAS (mensalidade do MEI)
//! dos clientes fresca, pra que o `get_client_state` reporte atraso e
//! próximo vencimento como leitura SQL pura (sem RPA no caminho).
//!
//! Mesmo desenho do `mei_refresh`: de hora em hora o worker acorda e pega
//! um lote de clientes com CNPJ que estão "vencidos" pra reconsulta, e
//! roda [`das::refresh_das_status`] em cada um. O PGMEI é público por CNPJ
//! (não precisa de credenciais gov.br) — a seleção filtra CNPJ presente,
//! lead não recusado, e o agendamento por cliente em `das_proxima_tentativa_em`.
//!
//! O **quando reconsultar cada cliente** não é um TTL fixo: no sucesso, o
//! `refresh_das_status` agenda a próxima consulta pro menor vencimento
//! futuro entre os meses `a_vencer` (nada muda antes disso), e em falha
//! aplica backoff exponencial. Os dois usam a mesma coluna
//! `das_proxima_tentativa_em`; o worker só seleciona quem já passou dela.
//!
//! Knobs (env vars):
//! - `DAS_REFRESH_ENABLED`        — `false`/`0` desliga (default: ligado).
//! - `DAS_REFRESH_INTERVAL_SECS`  — período entre ciclos (default: 3600).
//! - `DAS_REFRESH_BATCH`          — máx. de clientes por ciclo (default: 20).

use std::sync::Arc;
use std::time::Duration;

use pgsafe::sql;

use crate::state::AppState;
use crate::tools::das;

const DEFAULT_INTERVAL_SECS: u64 = 3600;
const DEFAULT_BATCH: i64 = 20;

fn env_num<T: std::str::FromStr>(key: &str, default: T) -> T {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// `true` a menos que `DAS_REFRESH_ENABLED` seja explicitamente `false`/`0`.
pub fn enabled() -> bool {
    match std::env::var("DAS_REFRESH_ENABLED") {
        Ok(v) => !matches!(v.trim().to_ascii_lowercase().as_str(), "false" | "0" | "no"),
        Err(_) => true,
    }
}

/// Loop infinito do worker — mesma disciplina do `mei_refresh`: um ciclo
/// por vez, primeiro tick consumido pra não subir browser no boot.
pub async fn run_forever(state: Arc<AppState>) {
    let interval_secs = env_num::<u64>("DAS_REFRESH_INTERVAL_SECS", DEFAULT_INTERVAL_SECS);
    let mut tick = tokio::time::interval(Duration::from_secs(interval_secs));
    tick.tick().await;
    tracing::info!(interval_secs, "das_refresh: worker iniciado");
    loop {
        tick.tick().await;
        if let Err(e) = run_once(&state).await {
            tracing::warn!(error = %crate::errlog::anyhow_chain(&e), "das_refresh: ciclo falhou");
        }
    }
}

/// Um ciclo: seleciona o lote pendente e atualiza em sequência (um
/// browser por vez nesta box).
async fn run_once(state: &AppState) -> anyhow::Result<()> {
    let batch = env_num::<i64>("DAS_REFRESH_BATCH", DEFAULT_BATCH);

    // Seleção governada só por `das_proxima_tentativa_em`: NULL = cliente
    // novo nunca consultado (entra já); senão, o sucesso agendou pro
    // próximo vencimento e a falha pro backoff. Sem TTL fixo — quando está
    // tudo em dia nada muda até a fatura seguinte.
    let pendentes = sql!(
        &state.pool,
        "SELECT id, cnpj
         FROM zain.clients
         WHERE cnpj IS NOT NULL
           AND recusado_em IS NULL
           AND (das_proxima_tentativa_em IS NULL OR das_proxima_tentativa_em <= now())
         ORDER BY das_proxima_tentativa_em ASC NULLS FIRST
         LIMIT $batch"
    )
    .fetch_all()
    .await?;

    if pendentes.is_empty() {
        return Ok(());
    }
    tracing::info!(
        n = pendentes.len(),
        "das_refresh: processando clientes pendentes"
    );

    for row in &pendentes {
        let Some(cnpj) = row.cnpj.as_deref() else {
            continue;
        };
        let client_id = row.id;
        tracing::info!(%client_id, "das_refresh: atualizando situação DAS");
        if let Err(e) = das::refresh_das_status(state, client_id, cnpj).await {
            // Backoff já aplicado dentro do refresh; aqui só registra e
            // segue pro próximo cliente do lote.
            tracing::warn!(%client_id, error = %crate::errlog::anyhow_chain(&e), "das_refresh: falha ao atualizar cliente");
        }
    }
    Ok(())
}

#[cfg(test)]
mod manual_tests {
    //! Harness manual contra o banco real (`DATABASE_URL`). Roda UM ciclo da
    //! cron — seleciona os pendentes e faz RPA real no PGMEI — pra validar a
    //! população de `zain.das_mensal` sem esperar o intervalo do worker.
    //! Ignorado por default; rode com:
    //!
    //!   cargo test -p mcp jobs::das_refresh::manual_tests::run_once_real -- --ignored --nocapture
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

        // Mostra o que ficou consolidado.
        let conn = state.pool.get().await.expect("conn");
        let rows = conn
            .query(
                "SELECT client_id, periodo, competencia, situacao, total_cents, vencimento
                 FROM zain.das_mensal ORDER BY client_id, periodo",
                &[],
            )
            .await
            .expect("select das_mensal");
        println!("\n===== zain.das_mensal ({} linhas) =====", rows.len());
        for r in &rows {
            let competencia: String = r.get("competencia");
            let situacao: String = r.get("situacao");
            let total: Option<i64> = r.get("total_cents");
            let venc: Option<chrono::NaiveDate> = r.get("vencimento");
            println!("  {competencia:<14} {situacao:<12} total={total:?} venc={venc:?}");
        }
    }
}
