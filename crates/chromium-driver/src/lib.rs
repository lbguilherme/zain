#![allow(async_fn_in_trait)]

pub mod browser;
pub mod cdp;
pub mod discovery;
pub mod dom;
pub mod error;
pub mod frame;
pub mod keyboard;
pub mod page;
pub mod process;
pub mod runtime;
pub mod session;
pub mod target;
pub mod transport;
pub mod types;

pub use browser::{Browser, BrowserContext, BrowserEvent, BrowserEventStream};
pub use error::{CdpError, Result};
pub use frame::{FrameInfo, FrameSession};
pub use page::{PageEvent, PageEventStream, PageSession};
pub use process::{ChromiumProcess, LaunchOptions};
pub use runtime::{EvalResult, JsObject};
pub use session::{CdpEventStream, CdpSession};
pub use target::PageTarget;
pub use types::*;

use cdp::schema::SchemaCommands;

/// Limits concurrent browser instances to avoid resource contention.
/// Set to num_cpus / 2 (minimum 1). The permit is held for the
/// lifetime of the `ChromiumProcess`.
static LAUNCH_SEMAPHORE: std::sync::LazyLock<std::sync::Arc<tokio::sync::Semaphore>> =
    std::sync::LazyLock::new(|| {
        let max = (std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(2)
            / 2)
        .max(1);
        tracing::debug!(max, "Browser concurrency limit");
        std::sync::Arc::new(tokio::sync::Semaphore::new(max))
    });

/// Launches a Chromium process and connects to its CDP endpoint.
///
/// Automatically retries on transient `BrowserCrashed` errors (the Chromium
/// process dying before printing `DevTools listening on ...` to stderr),
/// which happens occasionally under resource pressure.
pub async fn launch(opts: LaunchOptions) -> Result<(ChromiumProcess, Browser)> {
    const MAX_ATTEMPTS: u32 = 3;

    let permit = LAUNCH_SEMAPHORE
        .clone()
        .acquire_owned()
        .await
        .map_err(|_| CdpError::Protocol {
            code: -1,
            message: "launch semaphore closed".into(),
        })?;

    let mut last_err = None;
    for attempt in 1..=MAX_ATTEMPTS {
        match launch_once(opts.clone()).await {
            Ok((mut process, browser)) => {
                process._launch_permit = Some(permit);
                return Ok((process, browser));
            }
            Err(e @ CdpError::BrowserCrashed) => {
                tracing::warn!(
                    attempt,
                    max_attempts = MAX_ATTEMPTS,
                    "Browser crashed on launch, retrying…"
                );
                last_err = Some(e);
                tokio::time::sleep(std::time::Duration::from_millis(500 * attempt as u64)).await;
            }
            Err(e) => return Err(e),
        }
    }

    Err(last_err.unwrap())
}

async fn launch_once(opts: LaunchOptions) -> Result<(ChromiumProcess, Browser)> {
    let process = ChromiumProcess::launch(opts).await?;

    // Fetch HTTP discovery info before connecting WebSocket.
    match discovery::get_version("127.0.0.1", process.debug_port).await {
        Ok(info) => {
            tracing::debug!(
                browser = %info.browser,
                protocol = %info.protocol_version,
                user_agent = %info.user_agent,
                v8 = %info.v8_version,
                "Browser version info"
            );
        }
        Err(e) => {
            tracing::debug!("Failed to fetch discovery info: {e:#}");
        }
    }

    let transport = transport::Transport::connect(process.ws_url()).await?;
    let session = CdpSession::new(transport);

    // Log available CDP domains.
    match session.schema_get_domains().await {
        Ok(ret) => {
            let domain_list: Vec<_> = ret
                .domains
                .iter()
                .map(|d| format!("{} v{}", d.name, d.version))
                .collect();
            tracing::debug!(count = domain_list.len(), domains = ?domain_list, "CDP domains");
        }
        Err(e) => {
            tracing::debug!("Failed to fetch CDP domains: {e:#}");
        }
    }

    let browser = Browser::new(session);
    Ok((process, browser))
}
