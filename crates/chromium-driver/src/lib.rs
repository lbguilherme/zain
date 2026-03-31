#![allow(async_fn_in_trait)]

pub mod browser;
pub mod cdp;
pub mod discovery;
pub mod dom;
pub mod error;
pub mod keyboard;
pub mod page;
pub mod process;
pub mod runtime;
pub mod session;
pub mod target;
pub mod transport;
pub mod types;

pub use browser::{Browser, BrowserEvent, BrowserEventStream};
pub use error::{CdpError, Result};
pub use page::{PageEvent, PageEventStream, PageSession};
pub use process::{ChromiumProcess, LaunchOptions};
pub use runtime::{EvalResult, JsObject};
pub use session::{CdpEventStream, CdpSession};
pub use target::PageTarget;
pub use types::*;

pub async fn launch(opts: LaunchOptions) -> Result<(ChromiumProcess, Browser)> {
    let process = ChromiumProcess::launch(opts).await?;
    let transport = transport::Transport::connect(process.ws_url()).await?;
    let session = CdpSession::new(transport);
    let browser = Browser::new(session);
    Ok((process, browser))
}

pub async fn connect(ws_url: &str) -> Result<Browser> {
    let transport = transport::Transport::connect(ws_url).await?;
    let session = CdpSession::new(transport);
    Ok(Browser::new(session))
}
