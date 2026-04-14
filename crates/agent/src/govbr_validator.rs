//! Background service que revalida sessões gov.br persistidas.
//!
//! A cada ciclo, varre `zain.clients` procurando sessões cuja última
//! validação passou de 24h e, para cada uma, chama
//! [`rpa::govbr::check_govbr_profile`] com cpf + senha + sessão salva.
//! O `check_govbr_profile` tenta reusar a sessão e cai pra login fresh
//! se ela falhar. Resultados:
//!
//! * `Ok` — atualiza sessão (cookies refrescados), perfil e o
//!   `govbr_session_valid_at`.
//! * `Err(MissingOtp)` ou `Err(InvalidCredentials)` — não dá pra
//!   completar o login em background (precisaria de OTP do app ou de
//!   uma senha nova). A sessão original já foi sondada e rejeitada,
//!   então apaga `govbr_session` + `govbr_session_valid_at`.
//! * `Err(_)` (browser, rede, captcha) — falha transiente, deixa
//!   intacto pro próximo ciclo.

use std::time::Duration;

use cubos_sql::sql;
use deadpool_postgres::Pool;
use rpa::govbr::{CheckOutcome, GovbrError, check_govbr_profile, session::SavedSession};
use tokio::sync::watch;
use uuid::Uuid;

const CYCLE_INTERVAL: Duration = Duration::from_secs(3600);

pub async fn run(pool: Pool, mut shutdown_rx: watch::Receiver<bool>) {
    tracing::info!("govbr validator: iniciado");
    loop {
        if let Err(e) = validate_pending(&pool).await {
            tracing::error!("govbr validator: erro no ciclo: {e:#}");
        }

        tokio::select! {
            _ = tokio::time::sleep(CYCLE_INTERVAL) => {}
            _ = shutdown_rx.wait_for(|v| *v) => {
                tracing::info!("govbr validator: shutdown");
                return;
            }
        }
    }
}

async fn validate_pending(pool: &Pool) -> anyhow::Result<()> {
    let rows = sql!(
        pool,
        "SELECT id, govbr_cpf, govbr_password, govbr_session
         FROM zain.clients
         WHERE govbr_session IS NOT NULL
           AND govbr_cpf IS NOT NULL
           AND govbr_password IS NOT NULL
           AND (govbr_session_valid_at IS NULL
                OR govbr_session_valid_at < now() - interval '24 hours')"
    )
    .fetch_all()
    .await?;

    if rows.is_empty() {
        return Ok(());
    }

    tracing::info!(count = rows.len(), "govbr validator: validando sessões");

    for row in rows {
        let client_id: Uuid = row.id;
        let Some(cpf) = row.govbr_cpf else { continue };
        let Some(password) = row.govbr_password else {
            continue;
        };
        let Some(session) = row.govbr_session else {
            continue;
        };

        match check_govbr_profile(&cpf, &password, None, Some(&session)).await {
            Ok(outcome) => {
                if let Err(e) = save_success(pool, client_id, &outcome).await {
                    tracing::error!(%client_id, "govbr validator: falha ao persistir sessão revalidada: {e:#}");
                } else {
                    tracing::info!(
                        %client_id,
                        fresh = outcome.fresh_login,
                        "govbr validator: sessão revalidada"
                    );
                }
            }
            Err(GovbrError::MissingOtp) | Err(GovbrError::InvalidCredentials(_)) => {
                tracing::info!(%client_id, "govbr validator: reauth impossível em background, removendo sessão");
                if let Err(e) = clear_session(pool, client_id).await {
                    tracing::error!(%client_id, "govbr validator: falha ao limpar sessão: {e:#}");
                }
            }
            Err(e) => {
                tracing::warn!(
                    %client_id,
                    "govbr validator: erro transiente, tentará no próximo ciclo: {e:#}"
                );
            }
        }
    }

    Ok(())
}

async fn save_success(pool: &Pool, client_id: Uuid, outcome: &CheckOutcome) -> anyhow::Result<()> {
    let session: SavedSession = outcome.session.clone();
    let nome: Option<&str> = Some(&outcome.profile.nome);
    let email = outcome.profile.email.as_deref();
    let telefone = outcome.profile.telefone.as_deref();
    let nivel = outcome.profile.nivel;

    sql!(
        pool,
        "UPDATE zain.clients
         SET govbr_session          = $session!,
             govbr_session_valid_at = now(),
             govbr_nome             = $nome,
             govbr_email            = $email,
             govbr_telefone         = $telefone,
             govbr_nivel            = $nivel,
             updated_at             = now()
         WHERE id = $client_id"
    )
    .execute()
    .await?;
    Ok(())
}

async fn clear_session(pool: &Pool, client_id: Uuid) -> anyhow::Result<()> {
    sql!(
        pool,
        "UPDATE zain.clients
         SET govbr_session          = NULL,
             govbr_session_valid_at = NULL,
             updated_at             = now()
         WHERE id = $client_id"
    )
    .execute()
    .await?;
    Ok(())
}
