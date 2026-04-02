use crate::cdp::browser::{BrowserCommands, GetVersionReturn};
use crate::cdp::target::{
    AttachedToTargetEvent, CreateTargetParams, DetachedFromTargetEvent, GetTargetsParams,
    TargetCommands, TargetCreatedEvent, TargetDestroyedEvent, TargetInfoChangedEvent,
};
use crate::error::Result;
use crate::session::{CdpEventStream, CdpSession};
use crate::target::PageTarget;
use crate::types::TargetInfo;

/// Typed browser-level event, parsed from raw CDP events.
///
/// Covers the stable Target domain events emitted at browser scope.
/// Any event that doesn't match a known variant is delivered as [`Other`](Self::Other).
#[derive(Debug, Clone)]
pub enum BrowserEvent {
    /// A new target was created.
    ///
    /// CDP: `Target.targetCreated`
    TargetCreated(TargetCreatedEvent),

    /// A target was destroyed.
    ///
    /// CDP: `Target.targetDestroyed`
    TargetDestroyed(TargetDestroyedEvent),

    /// Information about a target changed.
    ///
    /// CDP: `Target.targetInfoChanged`
    TargetInfoChanged(TargetInfoChangedEvent),

    /// Attached to a target (via auto-attach or explicit attach).
    ///
    /// CDP: `Target.attachedToTarget`
    AttachedToTarget(AttachedToTargetEvent),

    /// Detached from a target.
    ///
    /// CDP: `Target.detachedFromTarget`
    DetachedFromTarget(DetachedFromTargetEvent),

    /// An event not covered by the typed variants above.
    /// Contains the raw method name and JSON params.
    Other {
        method: String,
        params: serde_json::Value,
    },
}

/// Typed event receiver for a [`Browser`].
///
/// Wraps a session-scoped [`CdpEventStream`] (browser-level, no `sessionId`)
/// and parses events into [`BrowserEvent`] variants.
pub struct BrowserEventStream {
    inner: CdpEventStream,
}

impl BrowserEventStream {
    /// Receives the next typed browser event.
    ///
    /// Blocks until an event arrives. Returns `None` if the channel is closed.
    pub async fn recv(&mut self) -> Option<BrowserEvent> {
        self.inner.recv().await.map(Self::parse)
    }

    /// Non-blocking attempt to receive the next typed browser event.
    pub fn try_recv(&mut self) -> Option<BrowserEvent> {
        self.inner.try_recv().map(Self::parse)
    }

    fn parse(raw: crate::transport::CdpEvent) -> BrowserEvent {
        match raw.method.as_str() {
            "Target.targetCreated" => match serde_json::from_value(raw.params.clone()) {
                Ok(e) => BrowserEvent::TargetCreated(e),
                Err(_) => BrowserEvent::Other { method: raw.method, params: raw.params },
            },
            "Target.targetDestroyed" => match serde_json::from_value(raw.params.clone()) {
                Ok(e) => BrowserEvent::TargetDestroyed(e),
                Err(_) => BrowserEvent::Other { method: raw.method, params: raw.params },
            },
            "Target.targetInfoChanged" => match serde_json::from_value(raw.params.clone()) {
                Ok(e) => BrowserEvent::TargetInfoChanged(e),
                Err(_) => BrowserEvent::Other { method: raw.method, params: raw.params },
            },
            "Target.attachedToTarget" => match serde_json::from_value(raw.params.clone()) {
                Ok(e) => BrowserEvent::AttachedToTarget(e),
                Err(_) => BrowserEvent::Other { method: raw.method, params: raw.params },
            },
            "Target.detachedFromTarget" => match serde_json::from_value(raw.params.clone()) {
                Ok(e) => BrowserEvent::DetachedFromTarget(e),
                Err(_) => BrowserEvent::Other { method: raw.method, params: raw.params },
            },
            _ => BrowserEvent::Other {
                method: raw.method,
                params: raw.params,
            },
        }
    }
}

/// High-level connection to a browser instance.
///
/// Created via [`launch`](crate::launch) or [`connect`](crate::connect). Wraps a
/// browser-level CDP session (no `sessionId`), allowing target management (tabs),
/// browser info queries and page creation.
///
/// ```rust,no_run
/// # async fn example() -> chromium_driver::Result<()> {
/// let (mut process, browser) = chromium_driver::launch(Default::default()).await?;
/// let version = browser.get_version().await?;
/// let page = browser.create_page("https://example.com").await?.attach().await?;
/// browser.close().await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct Browser {
    session: CdpSession,
}

impl Browser {
    pub(crate) fn new(session: CdpSession) -> Self {
        Self { session }
    }

    /// Returns browser information: product name, protocol version, user-agent, etc.
    pub async fn get_version(&self) -> Result<GetVersionReturn> {
        self.session.browser_get_version().await
    }

    /// Gracefully closes the browser via CDP.
    ///
    /// The browser process will terminate after this call. Use
    /// [`ChromiumProcess::wait`](crate::ChromiumProcess::wait) to await
    /// full process termination.
    pub async fn close(&self) -> Result<()> {
        self.session.browser_close().await
    }

    /// Lists all active targets in the browser (pages, service workers, etc.).
    ///
    /// Each [`TargetInfo`] contains the target's type, URL, title and attachment state.
    pub async fn get_targets(&self) -> Result<Vec<TargetInfo>> {
        let ret = self
            .session
            .target_get_targets(&GetTargetsParams::default())
            .await?;
        Ok(ret.target_infos)
    }

    /// Creates a new tab (page target) and navigates it to the given URL.
    ///
    /// Returns a [`PageTarget`] that can be connected via [`attach`](PageTarget::attach)
    /// to obtain a [`PageSession`](crate::PageSession) with navigation methods.
    ///
    /// - `url`: initial page URL. Use `"about:blank"` for an empty tab.
    pub async fn create_page(&self, url: &str) -> Result<PageTarget> {
        let ret = self
            .session
            .target_create_target(&CreateTargetParams {
                url: url.to_owned(),
                ..Default::default()
            })
            .await?;

        Ok(PageTarget::new(self.clone(), ret.target_id))
    }

    /// Returns a typed event stream for browser-level events.
    ///
    /// Events are parsed into [`BrowserEvent`] variants. Requires
    /// [`Target.setDiscoverTargets`](crate::cdp::target::TargetCommands::target_set_discover_targets)
    /// to be enabled for target discovery events.
    pub fn events(&self) -> BrowserEventStream {
        BrowserEventStream {
            inner: self.session.events(),
        }
    }

    /// Direct access to the raw CDP session for commands not covered by the typed API.
    ///
    /// Useful for accessing CDP domains or methods that don't have a wrapper yet,
    /// such as `Target.setDiscoverTargets` or experimental domains.
    pub fn cdp(&self) -> &CdpSession {
        &self.session
    }
}
