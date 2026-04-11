//! Detecção e espera passiva de challenges hCaptcha.
//!
//! A resolução em si é delegada para uma extensão instalada no browser
//! (ex: NopeCHA). O Rust só localiza a iframe de challenge e fica
//! pollando até sumir.

use std::time::Duration;

use chromium_driver::{FrameSession, PageSession};

/// Se um challenge hCaptcha estiver presente, espera ele sumir (presumindo
/// que uma extensão instalada no browser vai resolver).
///
/// - Retorna `Ok(false)` se não havia challenge para começo de conversa.
/// - Retorna `Ok(true)` se havia e sumiu dentro do `solve_timeout`.
/// - Retorna `Err` se o challenge persistiu até o timeout.
pub async fn wait_until_gone(
    page: &PageSession,
    detect_timeout: Duration,
    solve_timeout: Duration,
) -> anyhow::Result<bool> {
    if find_challenge_frame(page, detect_timeout).await?.is_none() {
        return Ok(false);
    }

    tracing::info!(
        "hcaptcha: iframe detectada, aguardando extensão resolver (até {solve_timeout:?})"
    );

    let deadline = tokio::time::Instant::now() + solve_timeout;
    loop {
        let frames = page.get_frames().await?;
        let still_present = frames
            .iter()
            .any(|f| f.url.contains("hcaptcha.com") && f.url.contains("frame=challenge"));

        if !still_present {
            tracing::info!("hcaptcha: iframe sumiu, challenge resolvido pela extensão");
            return Ok(true);
        }

        if tokio::time::Instant::now() >= deadline {
            anyhow::bail!("hcaptcha: extensão não resolveu o challenge em {solve_timeout:?}");
        }

        tokio::time::sleep(Duration::from_millis(1000)).await;
    }
}

/// Polla por uma iframe cujo URL identifica um challenge do hCaptcha em
/// estado "challenge" (não o checkbox). Retorna `None` se nada aparecer
/// dentro do `timeout` — isso é normal quando o login passa sem desafio.
async fn find_challenge_frame(
    page: &PageSession,
    timeout: Duration,
) -> anyhow::Result<Option<FrameSession>> {
    let deadline = tokio::time::Instant::now() + timeout;

    loop {
        let frames = page.get_frames().await?;
        let hit = frames.into_iter().find(|f| is_challenge_url(&f.url));

        if let Some(info) = hit {
            tracing::debug!(url = %info.url, "hcaptcha: iframe de challenge encontrada");
            let frame = page.frame(&info.id).await?;
            return Ok(Some(frame));
        }

        if tokio::time::Instant::now() >= deadline {
            return Ok(None);
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

/// Heurística: é uma URL de frame de challenge do hCaptcha.
///
/// hCaptcha renderiza em múltiplas iframes — uma de checkbox pequena
/// (`frame=checkbox`) e uma maior com o desafio propriamente dito
/// (`frame=challenge`). Só queremos essa segunda.
fn is_challenge_url(url: &str) -> bool {
    url.contains("hcaptcha.com") && url.contains("frame=challenge")
}
