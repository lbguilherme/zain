use std::sync::Arc;

use tokio::sync::OnceCell;

use crate::cdp::dom::DomCommands;
use crate::cdp::emulation::EmulationCommands;
use base64::Engine;
use crate::cdp::page::{
    CaptureScreenshotParams, DomContentEventFiredEvent, FrameNavigatedEvent,
    GetNavigationHistoryReturn, LifecycleEventEvent, LoadEventFiredEvent, NavigateParams,
    NavigateReturn, PageCommands, ReloadParams,
};
use crate::cdp::target::TargetCommands;
use crate::dom::Dom;
use crate::error::Result;
use crate::runtime::{self, EvalResult};
use crate::session::{CdpEventStream, CdpSession};
use crate::target::TargetInner;
use crate::types::{SessionId, TargetId};

/// Typed page event, parsed from raw CDP events.
///
/// Variants cover the stable Page domain events. Any event that doesn't match
/// a known variant is delivered as [`Other`](Self::Other) with the raw payload.
#[derive(Debug, Clone)]
pub enum PageEvent {
    /// The page's `load` event fired.
    ///
    /// CDP: `Page.loadEventFired`
    LoadEventFired(LoadEventFiredEvent),

    /// The page's `DOMContentLoaded` event fired.
    ///
    /// CDP: `Page.domContentEventFired`
    DomContentEventFired(DomContentEventFiredEvent),

    /// A frame completed navigation.
    ///
    /// CDP: `Page.frameNavigated`
    FrameNavigated(FrameNavigatedEvent),

    /// A lifecycle milestone was reached (e.g. `"load"`, `"DOMContentLoaded"`,
    /// `"networkIdle"`, `"commit"`, `"init"`).
    ///
    /// CDP: `Page.lifecycleEvent`
    LifecycleEvent(LifecycleEventEvent),

    /// An event not covered by the typed variants above.
    /// Contains the raw method name and JSON params.
    Other {
        method: String,
        params: serde_json::Value,
    },
}

/// Typed event receiver for a [`PageSession`].
///
/// Wraps a session-scoped [`CdpEventStream`] and parses events into [`PageEvent`] variants.
pub struct PageEventStream {
    inner: CdpEventStream,
}

impl PageEventStream {
    /// Receives the next typed page event.
    ///
    /// Blocks until an event arrives. Returns `None` if the channel is closed.
    pub async fn recv(&mut self) -> Option<PageEvent> {
        self.inner.recv().await.map(Self::parse)
    }

    /// Non-blocking attempt to receive the next typed page event.
    pub fn try_recv(&mut self) -> Option<PageEvent> {
        self.inner.try_recv().map(Self::parse)
    }

    fn parse(raw: crate::transport::CdpEvent) -> PageEvent {
        match raw.method.as_str() {
            "Page.loadEventFired" => match serde_json::from_value(raw.params.clone()) {
                Ok(e) => PageEvent::LoadEventFired(e),
                Err(_) => PageEvent::Other { method: raw.method, params: raw.params },
            },
            "Page.domContentEventFired" => match serde_json::from_value(raw.params.clone()) {
                Ok(e) => PageEvent::DomContentEventFired(e),
                Err(_) => PageEvent::Other { method: raw.method, params: raw.params },
            },
            "Page.frameNavigated" => match serde_json::from_value(raw.params.clone()) {
                Ok(e) => PageEvent::FrameNavigated(e),
                Err(_) => PageEvent::Other { method: raw.method, params: raw.params },
            },
            "Page.lifecycleEvent" => match serde_json::from_value(raw.params.clone()) {
                Ok(e) => PageEvent::LifecycleEvent(e),
                Err(_) => PageEvent::Other { method: raw.method, params: raw.params },
            },
            _ => PageEvent::Other {
                method: raw.method,
                params: raw.params,
            },
        }
    }
}

/// Session attached to a specific page target.
///
/// Created via [`PageTarget::attach`](crate::target::PageTarget::attach). Wraps a CDP
/// session with a bound `sessionId`, exposing Page domain methods with a simplified API.
///
/// Holds a reference to the target — the tab stays alive as long as any `PageSession`
/// or the originating [`PageTarget`](crate::target::PageTarget) exists. When all
/// references are dropped, the tab is automatically closed.
///
/// Call [`detach`](Self::detach) to explicitly release the CDP session binding.
/// On drop without detach, the session is detached automatically (best-effort).
pub struct PageSession {
    session: CdpSession,
    session_id: SessionId,
    target: Arc<TargetInner>,
    dom: OnceCell<Dom>,
}

impl PageSession {
    pub(crate) fn new(
        session: CdpSession,
        session_id: SessionId,
        target: Arc<TargetInner>,
    ) -> Self {
        Self {
            session,
            session_id,
            target,
            dom: OnceCell::new(),
        }
    }

    /// Enables the Page domain, causing the browser to emit events such as
    /// `loadEventFired`, `domContentEventFired` and `frameNavigated` for this session.
    ///
    /// Must be called before listening to events via [`events`](Self::events).
    pub async fn enable(&self) -> Result<()> {
        self.session.page_enable().await
    }

    /// Disables the Page domain, stopping page event emission for this session.
    ///
    /// After calling this, the browser no longer sends Page events over the
    /// WebSocket for this session, reducing unnecessary traffic.
    pub async fn disable(&self) -> Result<()> {
        self.session.page_disable().await
    }

    /// Navigates the page to the given URL.
    ///
    /// Returns the `frame_id` of the main frame and optionally `error_text` if
    /// navigation failed (e.g. DNS resolution failure). Navigation is initiated but
    /// not necessarily complete when this method returns — use lifecycle events
    /// to await full page load.
    ///
    /// - `url`: destination URL (http, https, data:, about:blank, etc.)
    pub async fn navigate(&self, url: &str) -> Result<NavigateReturn> {
        self.session
            .page_navigate(&NavigateParams {
                url: url.to_owned(),
                ..Default::default()
            })
            .await
    }

    /// Navigates with full control over parameters (referrer, transition type, frame).
    ///
    /// Use when you need to specify a `referrer`, `transition_type`, or navigate
    /// a specific frame instead of the top-level frame.
    pub async fn navigate_with(&self, params: &NavigateParams) -> Result<NavigateReturn> {
        self.session.page_navigate(params).await
    }

    /// Reloads the current page.
    ///
    /// - `ignore_cache`: if `true`, bypasses the browser cache (equivalent to Ctrl+Shift+R).
    pub async fn reload(&self, ignore_cache: bool) -> Result<()> {
        self.session
            .page_reload(&ReloadParams {
                ignore_cache: Some(ignore_cache),
                ..Default::default()
            })
            .await
    }

    /// Returns the page's navigation history.
    ///
    /// The result includes `current_index` (active position in history) and `entries`
    /// (list of [`NavigationEntry`](crate::types::NavigationEntry) with id, url and title).
    pub async fn get_navigation_history(&self) -> Result<GetNavigationHistoryReturn> {
        self.session.page_get_navigation_history().await
    }

    /// Navigates to a specific history entry.
    ///
    /// - `entry_id`: the `id` of a [`NavigationEntry`](crate::types::NavigationEntry)
    ///   obtained via [`get_navigation_history`](Self::get_navigation_history).
    pub async fn navigate_to_history_entry(&self, entry_id: i64) -> Result<()> {
        self.session.page_navigate_to_history_entry(entry_id).await
    }

    /// Enables or disables lifecycle events (`init`, `DOMContentLoaded`,
    /// `load`, `networkIdle`, etc.).
    ///
    /// When enabled, the browser emits `Page.lifecycleEvent` at each frame
    /// state transition. Requires [`enable`](Self::enable) to be called first.
    ///
    /// - `enabled`: `true` to activate, `false` to deactivate.
    pub async fn set_lifecycle_events_enabled(&self, enabled: bool) -> Result<()> {
        self.session
            .page_set_lifecycle_events_enabled(enabled)
            .await
    }

    /// Returns a typed event stream for this page session.
    ///
    /// Events are parsed into [`PageEvent`] variants. Events from other sessions
    /// are automatically filtered out by the underlying [`CdpSession`].
    /// Unknown events are delivered as [`PageEvent::Other`].
    ///
    /// Requires [`enable`](Self::enable) to have been called for Page events,
    /// and [`set_lifecycle_events_enabled`](Self::set_lifecycle_events_enabled)
    /// for lifecycle events.
    pub fn events(&self) -> PageEventStream {
        PageEventStream {
            inner: self.session.events(),
        }
    }

    /// Returns the DOM handle, enabling the DOM domain on first call.
    pub async fn dom(&self) -> Result<&Dom> {
        self.dom
            .get_or_try_init(|| async {
                self.session.dom_enable().await?;
                self.session.emulation_set_touch_enabled(true).await?;
                Ok(Dom::new(self.session.clone()))
            })
            .await
    }

    /// Evaluates a JavaScript expression in the page's global scope.
    ///
    /// Returns a managed `EvalResult` — either a `JsObject` (for objects)
    /// or a primitive `RemoteObject`.
    pub async fn eval(&self, expression: &str) -> Result<EvalResult> {
        runtime::evaluate(&self.session, expression).await
    }

    /// Evaluates a JavaScript expression and returns the result by value (JSON).
    pub async fn eval_value(&self, expression: &str) -> Result<serde_json::Value> {
        runtime::evaluate_value(&self.session, expression).await
    }

    /// Takes a PNG screenshot of the full visible page and returns raw bytes.
    pub async fn capture_screenshot(&self) -> Result<Vec<u8>> {
        let ret = self
            .session
            .page_capture_screenshot(&CaptureScreenshotParams {
                format: Some("png".into()),
                ..Default::default()
            })
            .await?;
        base64::engine::general_purpose::STANDARD
            .decode(&ret.data)
            .map_err(|e| crate::error::CdpError::Protocol {
                code: -1,
                message: format!("base64 decode screenshot: {e}"),
            })
    }

    /// Reads a blob by UUID via the CDP IO domain.
    ///
    /// The UUID comes from a `blob:` URL (e.g. `blob:https://.../<uuid>`).
    /// Uses `IO.read` in a loop until EOF, then `IO.close`.
    pub async fn read_blob(&self, uuid: &str) -> Result<Vec<u8>> {
        let handle = format!("blob:{uuid}");
        let mut all_bytes = Vec::new();

        loop {
            let result: serde_json::Value = self
                .session
                .call("IO.read", &serde_json::json!({ "handle": handle, "size": 1_000_000 }))
                .await?;

            let base64_encoded = result
                .get("base64Encoded")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let data = result
                .get("data")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let eof = result
                .get("eof")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);

            if !data.is_empty() {
                if base64_encoded {
                    let chunk = base64::engine::general_purpose::STANDARD
                        .decode(data)
                        .map_err(|e| crate::error::CdpError::Protocol {
                            code: -1,
                            message: format!("base64 decode IO.read: {e}"),
                        })?;
                    all_bytes.extend_from_slice(&chunk);
                } else {
                    all_bytes.extend_from_slice(data.as_bytes());
                }
            }

            if eof {
                break;
            }
        }

        Ok(all_bytes)
    }

    /// Direct access to the raw CDP session for commands not covered by the typed API.
    pub fn cdp(&self) -> &CdpSession {
        &self.session
    }

    /// Returns the target ID associated with this session.
    pub fn target_id(&self) -> &TargetId {
        &self.target.target_id
    }

    /// Waits for the page to finish loading.
    ///
    /// Listens for the `Page.loadEventFired` event, which fires when the page's
    /// `load` event triggers (all resources loaded). Times out if the event
    /// doesn't arrive within `timeout`.
    ///
    /// Requires [`enable`](Self::enable) to have been called.
    pub async fn wait_for_load(&self, timeout: std::time::Duration) -> Result<()> {
        let mut events = self.events();
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            tokio::select! {
                evt = events.recv() => {
                    match evt {
                        Some(PageEvent::LoadEventFired(_)) => return Ok(()),
                        Some(_) => continue,
                        None => return Err(crate::error::CdpError::ConnectionClosed),
                    }
                }
                _ = tokio::time::sleep_until(deadline) => {
                    return Err(crate::error::CdpError::Timeout(timeout));
                }
            }
        }
    }

}

impl Drop for PageSession {
    fn drop(&mut self) {
        let session = self.session.clone();
        let target = self.target.clone();
        let session_id = self.session_id.clone();
        let had_dom = self.dom.get().is_some();
        tokio::spawn(async move {
            if had_dom {
                let _ = session.dom_disable().await;
            }
            let _ = target
                .browser_session
                .target_detach_from_target(&session_id)
                .await;
        });
    }
}
