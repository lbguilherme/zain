//! Worker de background que mantém a situação MEI dos clientes fresca,
//! pra que o `get_client_state` seja uma leitura SQL pura (sem RPA no
//! caminho de leitura).
//!
//! De hora em hora pega um lote de clientes cuja situação está velha (TTL
//! de 24h) ou nunca foi checada, e roda [`govbr::refresh_mei_status`] em
//! cada um — que consulta o CCMEI pelo CPF e, se não tem MEI, checa a
//! elegibilidade pra abrir um (usando/renovando a sessão gov.br salva).
//! Toda a persistência e o tratamento de erro/2FA ficam lá; aqui só
//! decidimos QUEM checar e QUANDO.
//!
//! Knobs (env vars):
//! - `MEI_REFRESH_ENABLED`   — `false`/`0` desliga o worker (default: ligado).
//! - `MEI_REFRESH_INTERVAL_SECS` — período entre ciclos (default: 3600).
//! - `MEI_REFRESH_BATCH`     — máx. de clientes por ciclo (default: 20).

use std::sync::Arc;
use std::time::Duration;

use pgsafe::sql;

use crate::state::AppState;
use crate::tools::govbr;

const DEFAULT_INTERVAL_SECS: u64 = 3600;
const DEFAULT_BATCH: i64 = 20;

/// Lê uma env var numérica, caindo no default se ausente/inválida.
fn env_num<T: std::str::FromStr>(key: &str, default: T) -> T {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// `true` a menos que `MEI_REFRESH_ENABLED` seja explicitamente `false`/`0`.
pub fn enabled() -> bool {
    match std::env::var("MEI_REFRESH_ENABLED") {
        Ok(v) => !matches!(v.trim().to_ascii_lowercase().as_str(), "false" | "0" | "no"),
        Err(_) => true,
    }
}

/// Loop infinito do worker. Cada ciclo é aguardado até o fim antes do
/// próximo tick — `interval` coalesce ticks perdidos, então não há
/// sobreposição mesmo se um ciclo passar do período.
pub async fn run_forever(state: Arc<AppState>) {
    let interval_secs = env_num::<u64>("MEI_REFRESH_INTERVAL_SECS", DEFAULT_INTERVAL_SECS);
    let mut tick = tokio::time::interval(Duration::from_secs(interval_secs));
    // Consome o primeiro tick (que dispara imediatamente) pra NÃO subir
    // browser no instante do boot — útil em dev. O primeiro ciclo real
    // acontece após um período completo.
    tick.tick().await;
    tracing::info!(interval_secs, "mei_refresh: worker iniciado");
    loop {
        tick.tick().await;
        if let Err(e) = run_once(&state).await {
            tracing::warn!(error = %e, "mei_refresh: ciclo falhou");
        }
    }
}

/// Um ciclo: seleciona o lote de pendentes e atualiza cada um em
/// sequência (um browser por vez é mais seguro nesta box).
async fn run_once(state: &AppState) -> anyhow::Result<()> {
    let batch = env_num::<i64>("MEI_REFRESH_BATCH", DEFAULT_BATCH);

    // Pendentes: tem CPF, não foi recusado, e a situação MEI está velha
    // (>24h) OU nunca checada OU não tem MEI/elegibilidade desconhecida
    // mas agora há sessão gov.br pra checar (3ª cláusula: re-dispara a
    // elegibilidade só DEPOIS que o cliente re-autentica).
    let pendentes = sql!(
        &state.pool,
        "SELECT id, cpf
         FROM zain.clients
         WHERE cpf IS NOT NULL
           AND recusado_em IS NULL
           AND ( mei_consultado_em IS NULL
              OR mei_consultado_em < now() - interval '24 hours'
              OR (mei_ccmei IS NULL AND mei_pode_abrir IS NULL AND govbr_session IS NOT NULL) )
         ORDER BY mei_consultado_em ASC NULLS FIRST
         LIMIT $batch"
    )
    .fetch_all()
    .await?;

    if pendentes.is_empty() {
        return Ok(());
    }
    tracing::info!(
        n = pendentes.len(),
        "mei_refresh: processando clientes pendentes"
    );

    for row in &pendentes {
        let Some(cpf) = row.cpf.as_deref() else {
            continue;
        };
        let client_id = row.id;
        tracing::info!(%client_id, "mei_refresh: atualizando situação MEI");
        // `refresh_mei_status` nunca propaga erro (engole e loga
        // internamente) e persiste tudo; ignoramos o retorno (ele é só
        // pro caller interativo).
        let _ = govbr::refresh_mei_status(state, client_id, cpf, None).await;
    }
    Ok(())
}

#[cfg(test)]
mod manual_tests {
    //! Harness manual contra o banco real (host do `DATABASE_URL`). Roda a
    //! cron uma vez e mostra o `get_client_state` de cada cliente
    //! antes/depois. Ignorado por default (faz RPA real); rode com:
    //!
    //!   cargo test -p mcp manual_tests::cron_e_getstate -- --ignored --nocapture
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

    fn text_of(r: &rmcp::model::CallToolResult) -> String {
        let v = serde_json::to_value(r).unwrap_or_default();
        v.get("content")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or("(sem texto)")
            .to_string()
    }

    async fn dump_states(state: &AppState, label: &str) {
        let client = state.pool.get().await.expect("get conn");
        let rows = client
            .query("SELECT id, name FROM zain.clients ORDER BY created_at", &[])
            .await
            .expect("listar clientes");
        println!("\n================= get_client_state — {label} =================");
        for row in &rows {
            let id: uuid::Uuid = row.get("id");
            let name: Option<String> = row.get("name");
            let r = get_client_state::run(state, id, get_client_state::Args {}).await;
            println!(
                "\n----- {} ({}) -----\n{}",
                &id.to_string()[..8],
                name.unwrap_or_else(|| "(sem nome)".into()),
                text_of(&r)
            );
        }
    }

    #[ignore]
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn cron_e_getstate() {
        let _ = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .with_test_writer()
            .try_init();
        let state = build_state();

        dump_states(&state, "ANTES").await;

        println!("\n================= rodando run_once (cron) =================");
        run_once(&state).await.expect("run_once");

        dump_states(&state, "DEPOIS").await;
    }

    /// Refresh cirúrgico de UM cliente (default: o autenticado `c768deea`,
    /// que reusa a sessão válida no Tier 2 sem disparar 2FA). Override via
    /// `CLIENT_ID=<uuid>`. Rode com:
    ///   CLIENT_ID=<uuid> cargo test -p mcp manual_tests::refresh_um -- --ignored --nocapture
    #[ignore]
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn refresh_um() {
        let _ = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .with_test_writer()
            .try_init();
        let state = build_state();

        let id_str = std::env::var("CLIENT_ID")
            .unwrap_or_else(|_| "c768deea-d477-4bd9-8c2d-e98fc0af1bc5".to_string());
        let client_id: uuid::Uuid = id_str.parse().expect("CLIENT_ID uuid inválido");

        let conn = state.pool.get().await.expect("conn");
        let row = conn
            .query_one("SELECT cpf FROM zain.clients WHERE id = $1", &[&client_id])
            .await
            .expect("cliente não encontrado");
        let cpf: Option<String> = row.get("cpf");
        let cpf = cpf.expect("cliente sem cpf");

        let antes = get_client_state::run(&state, client_id, get_client_state::Args {}).await;
        println!("\n===== ANTES =====\n{}", text_of(&antes));

        println!("\n===== refresh_mei_status (RPA real) =====");
        let _ = govbr::refresh_mei_status(&state, client_id, &cpf, None).await;

        let depois = get_client_state::run(&state, client_id, get_client_state::Args {}).await;
        println!("\n===== DEPOIS =====\n{}", text_of(&depois));
    }

    /// Religa via OTP (lê o código da env `OTP`) e, com a sessão fresca,
    /// valida o caminho feliz do CCMEI (consultar_certificado com login).
    /// Rode com:
    ///   OTP=123456 cargo test -p mcp manual_tests::auth_otp_e_refresh -- --ignored --nocapture
    #[ignore]
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn auth_otp_e_refresh() {
        let _ = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .with_test_writer()
            .try_init();
        let state = build_state();

        let id_str = std::env::var("CLIENT_ID")
            .unwrap_or_else(|_| "c768deea-d477-4bd9-8c2d-e98fc0af1bc5".to_string());
        let client_id: uuid::Uuid = id_str.parse().expect("CLIENT_ID uuid inválido");
        let otp = std::env::var("OTP").expect("set OTP=<código de 6 dígitos>");

        println!("\n===== auth_govbr_otp =====");
        let resp = govbr::run_otp(&state, client_id, govbr::OtpArgs { otp }).await;
        println!(
            "resposta: {}",
            serde_json::to_string_pretty(&resp).unwrap_or_default()
        );

        let conn = state.pool.get().await.expect("conn");
        let row = conn
            .query_one("SELECT cpf FROM zain.clients WHERE id = $1", &[&client_id])
            .await
            .expect("cliente");
        let cpf: Option<String> = row.get("cpf");
        let cpf = cpf.expect("sem cpf");

        println!("\n===== refresh_mei_status (CCMEI com login) =====");
        let _ = govbr::refresh_mei_status(&state, client_id, &cpf, None).await;

        let depois = get_client_state::run(&state, client_id, get_client_state::Args {}).await;
        println!("\n===== DEPOIS =====\n{}", text_of(&depois));
    }
}
