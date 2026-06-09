use std::sync::Arc;

use std::time::Duration;

use crate::cdp::dom::{DomCommands, EnableParams};
use crate::cdp::emulation::{EmulationCommands, SetUserAgentOverrideParams};
use crate::cdp::io::{IoCommands, ReadParams, StreamHandle};
use crate::cdp::page::{
    CaptureScreenshotParams, DomContentEventFiredEvent, FrameNavigatedEvent,
    GetNavigationHistoryReturn, LifecycleEventEvent, LoadEventFiredEvent, NavigateParams,
    NavigateReturn, PageCommands, ReloadParams,
};
use crate::cdp::target::{DetachFromTargetParams, TargetCommands};
use crate::dom::Dom;
use crate::error::{CdpError, Result};
use crate::frame::{self, FrameInfo, FrameSession};
use crate::runtime::{self, EvalResult};
use crate::session::{CdpEventStream, CdpSession};
use crate::target::TargetInner;
use crate::types::{FrameId, SessionId, TargetId};
use base64::Engine;

/// Typed page event, parsed from raw CDP events.
///
/// Variants cover the stable Page domain events. Any event that doesn't match
/// a known variant is delivered as [`Other`](Self::Other) with the raw payload.
// Variants wrap generated CDP event structs of differing sizes; not worth
// boxing for the event-stream ergonomics.
#[allow(clippy::large_enum_variant)]
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

    /// Waits for an event matching `predicate`, with a timeout.
    ///
    /// Returns the matching event, or `CdpError::Timeout` if the deadline
    /// is reached. Non-matching events are discarded.
    pub async fn wait_for(
        &mut self,
        predicate: impl Fn(&PageEvent) -> bool,
        timeout: std::time::Duration,
    ) -> crate::Result<PageEvent> {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            match tokio::time::timeout_at(deadline, self.inner.recv()).await {
                Ok(Some(raw)) => {
                    let evt = Self::parse(raw);
                    if predicate(&evt) {
                        return Ok(evt);
                    }
                }
                Ok(None) => return Err(crate::CdpError::ConnectionClosed),
                Err(_) => return Err(crate::CdpError::Timeout(timeout)),
            }
        }
    }

    fn parse(raw: crate::transport::CdpEvent) -> PageEvent {
        match raw.method.as_str() {
            "Page.loadEventFired" => match serde_json::from_value(raw.params.clone()) {
                Ok(e) => PageEvent::LoadEventFired(e),
                Err(_) => PageEvent::Other {
                    method: raw.method,
                    params: raw.params,
                },
            },
            "Page.domContentEventFired" => match serde_json::from_value(raw.params.clone()) {
                Ok(e) => PageEvent::DomContentEventFired(e),
                Err(_) => PageEvent::Other {
                    method: raw.method,
                    params: raw.params,
                },
            },
            "Page.frameNavigated" => match serde_json::from_value(raw.params.clone()) {
                Ok(e) => PageEvent::FrameNavigated(e),
                Err(_) => PageEvent::Other {
                    method: raw.method,
                    params: raw.params,
                },
            },
            "Page.lifecycleEvent" => match serde_json::from_value(raw.params.clone()) {
                Ok(e) => PageEvent::LifecycleEvent(e),
                Err(_) => PageEvent::Other {
                    method: raw.method,
                    params: raw.params,
                },
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
        }
    }

    /// Enables the Page domain, causing the browser to emit events such as
    /// `loadEventFired`, `domContentEventFired` and `frameNavigated` for this session.
    ///
    /// Must be called before listening to events via [`events`](Self::events).
    pub async fn enable(&self) -> Result<()> {
        self.session
            .page_enable(&crate::cdp::page::EnableParams::default())
            .await
    }

    /// Disables the Page domain, stopping page event emission for this session.
    ///
    /// After calling this, the browser no longer sends Page events over the
    /// WebSocket for this session, reducing unnecessary traffic.
    pub async fn disable(&self) -> Result<()> {
        self.session.page_disable().await
    }

    /// Overrides the User-Agent for this page, and optionally the
    /// `Accept-Language` header (which also drives `navigator.languages`).
    ///
    /// Apply before navigating so the override is in effect for the first
    /// request. Useful when restoring a saved session whose trust token is
    /// bound to a specific User-Agent.
    pub async fn set_user_agent(
        &self,
        user_agent: &str,
        accept_language: Option<&str>,
    ) -> Result<()> {
        self.session
            .emulation_set_user_agent_override(&SetUserAgentOverrideParams {
                user_agent: user_agent.to_owned(),
                accept_language: accept_language.map(str::to_owned),
                platform: None,
                user_agent_metadata: None,
            })
            .await
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
    ///
    /// The DOM is shared across all sessions attached to the same target.
    pub async fn dom(&self) -> Result<&Dom> {
        self.target
            .dom
            .get_or_try_init(|| async {
                self.session.dom_enable(&EnableParams::default()).await?;
                // Touch emulation is enabled eagerly at `PageTarget::attach`, not
                // here — see the comment there for why (stable maxTouchPoints).
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

    /// Evaluates an async JavaScript expression (or one returning a Promise)
    /// and returns the resolved result by value.
    ///
    /// Uses `awaitPromise: true` — required for `async` IIFEs and any
    /// expression that returns a Promise.
    pub async fn eval_value_async(&self, expression: &str) -> Result<serde_json::Value> {
        runtime::evaluate_value_async(&self.session, expression).await
    }

    /// Evaluates a JS function with arguments passed safely as JSON values,
    /// returning the result by value.
    ///
    /// The function is invoked as `fn.apply(undefined, args)`. Arguments are
    /// serialized to a JSON array literal, so values containing quotes,
    /// backslashes or newlines are passed verbatim — no manual escaping, no
    /// injection. Prefer this over interpolating values into an expression.
    pub async fn eval_value_with_args(
        &self,
        function: &str,
        args: &[serde_json::Value],
    ) -> Result<serde_json::Value> {
        runtime::evaluate_value(&self.session, &apply_expr(function, args)?).await
    }

    /// Like [`eval_value_with_args`](Self::eval_value_with_args) but awaits a
    /// returned Promise — for `async` functions or any returning a thenable.
    pub async fn eval_value_with_args_async(
        &self,
        function: &str,
        args: &[serde_json::Value],
    ) -> Result<serde_json::Value> {
        runtime::evaluate_value_async(&self.session, &apply_expr(function, args)?).await
    }

    /// Takes a PNG screenshot of the full visible page and returns raw bytes.
    pub async fn capture_screenshot(&self) -> Result<Vec<u8>> {
        let ret = self
            .session
            .page_capture_screenshot(&CaptureScreenshotParams {
                format: Some(crate::cdp::page::CaptureScreenshotFormat::Png),
                ..Default::default()
            })
            .await?;
        base64::engine::general_purpose::STANDARD
            .decode(&ret.data)
            .map_err(|e| CdpError::Decode(format!("screenshot base64: {e}")))
    }

    /// Fetches a `blob:` URL from within the browser and returns raw bytes.
    ///
    /// Uses JS `fetch()` + `FileReader.readAsDataURL()` to convert the blob
    /// to base64, then decodes on our side. This is necessary because CDP
    /// `IO.read` doesn't support `blob:` URLs.
    pub async fn fetch_blob_url(&self, blob_url: &str) -> Result<Vec<u8>> {
        let (bytes, _mime) = self.fetch_blob_url_typed(blob_url).await?;
        Ok(bytes)
    }

    /// Like [`fetch_blob_url`](Self::fetch_blob_url) but also returns the MIME type
    /// extracted from the data URL (e.g. `"image/webp"`, `"image/jpeg"`).
    pub async fn fetch_blob_url_typed(&self, blob_url: &str) -> Result<(Vec<u8>, String)> {
        let result = self
            .eval_value_with_args_async(
                r#"async (url) => {
                    const r = await fetch(url);
                    const b = await r.blob();
                    return await new Promise((ok, err) => {
                        const rd = new FileReader();
                        rd.onloadend = () => ok(rd.result);
                        rd.onerror = () => err("read error");
                        rd.readAsDataURL(b);
                    });
                }"#,
                &[serde_json::Value::String(blob_url.to_owned())],
            )
            .await?;

        let data_url = result.as_str().ok_or_else(|| {
            CdpError::Unexpected(format!("blob fetch returned non-string: {result:?}"))
        })?;

        // "data:image/webp;base64,AAAA..."
        let (header, base64_part) = data_url
            .split_once(',')
            .ok_or_else(|| CdpError::Unexpected("invalid data URL from blob fetch".into()))?;

        // header = "data:image/webp;base64"
        let mime = header
            .strip_prefix("data:")
            .and_then(|s| s.split_once(';'))
            .map(|(m, _)| m.to_owned())
            .unwrap_or_default();

        let bytes = base64::engine::general_purpose::STANDARD
            .decode(base64_part)
            .map_err(|e| CdpError::Decode(format!("blob base64: {e}")))?;

        Ok((bytes, mime))
    }

    /// Reads a blob by UUID via the CDP IO domain.
    ///
    /// The UUID comes from a `blob:` URL (e.g. `blob:https://.../<uuid>`).
    /// Reads with `IO.read` in a loop until EOF, then always closes the stream
    /// with `IO.close` (even on read error) so the browser-side backing storage
    /// is released.
    pub async fn read_blob(&self, uuid: &str) -> Result<Vec<u8>> {
        let handle = StreamHandle(format!("blob:{uuid}"));
        let result = self.read_stream(&handle).await;
        // Best-effort close regardless of read outcome.
        let _ = self.session.io_close(&handle).await;
        result
    }

    async fn read_stream(&self, handle: &StreamHandle) -> Result<Vec<u8>> {
        let mut all_bytes = Vec::new();
        loop {
            let ret = self
                .session
                .io_read(&ReadParams {
                    handle: handle.clone(),
                    offset: None,
                    size: Some(1_000_000),
                })
                .await?;

            if !ret.data.is_empty() {
                if ret.base64_encoded.unwrap_or(false) {
                    let chunk = base64::engine::general_purpose::STANDARD
                        .decode(&ret.data)
                        .map_err(|e| CdpError::Decode(format!("IO.read base64: {e}")))?;
                    all_bytes.extend_from_slice(&chunk);
                } else {
                    all_bytes.extend_from_slice(ret.data.as_bytes());
                }
            }

            if ret.eof {
                break;
            }
        }
        Ok(all_bytes)
    }

    // ── Frames / iframes ────────────────────────────────────────────────

    /// Returns all frames in the page's frame tree (main frame + iframes).
    pub async fn get_frames(&self) -> Result<Vec<FrameInfo>> {
        let ret = self.session.page_get_frame_tree().await?;
        Ok(frame::flatten_frame_tree(&ret.frame_tree))
    }

    /// Enters an iframe by its `FrameId`, returning a [`FrameSession`] scoped
    /// to that frame's execution context and document.
    ///
    /// The `FrameSession` provides `dom()` and `eval()` methods that operate
    /// within the iframe, not the top-level page.
    pub async fn frame(&self, frame_id: &FrameId) -> Result<FrameSession> {
        frame::enter_frame(&self.session, frame_id).await
    }

    /// Waits for a frame whose URL contains the given substring to appear.
    ///
    /// Polls `get_frames()` at ~500ms intervals until a match is found or
    /// the timeout expires.
    pub async fn wait_for_frame(&self, url_contains: &str, timeout: Duration) -> Result<FrameInfo> {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let frames = self.get_frames().await?;
            if let Some(f) = frames.into_iter().find(|f| f.url.contains(url_contains)) {
                return Ok(f);
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(CdpError::Timeout(timeout));
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    // ── Debug ──────────────────────────────────────────────────────────────

    /// Saves a debug dump (`{name}.html` + `{name}.png`) under `dumps/` in the
    /// current working directory.
    ///
    /// The HTML is cleaned up: `<script>`, `<style>` and `<link>` tags are
    /// stripped and the output is indented for readability.
    pub async fn debug_dump(&self, name: &str) -> Result<()> {
        self.debug_dump_in(std::path::Path::new("dumps"), name)
            .await
    }

    /// Like [`debug_dump`](Self::debug_dump) but writes under `dir` (created if
    /// absent). Use when the working directory is read-only or dumps should be
    /// grouped elsewhere.
    pub async fn debug_dump_in(&self, dir: &std::path::Path, name: &str) -> Result<()> {
        std::fs::create_dir_all(dir)
            .map_err(|e| CdpError::Unexpected(format!("create dump dir {}: {e}", dir.display())))?;

        let dom = self.dom().await?;
        let html = dom.page_html().await?;
        let clean = beautify_html(&html);
        let html_path = dir.join(format!("{name}.html"));
        std::fs::write(&html_path, &clean)
            .map_err(|e| CdpError::Unexpected(format!("write html dump: {e}")))?;

        let png_path = dir.join(format!("{name}.png"));
        let png_bytes = self.capture_screenshot().await?;
        std::fs::write(&png_path, &png_bytes)
            .map_err(|e| CdpError::Unexpected(format!("write png dump: {e}")))?;

        tracing::debug!(
            html = %html_path.display(),
            png = %png_path.display(),
            "Debug dump saved"
        );
        Ok(())
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

    /// Waits for a `Page.lifecycleEvent` with the given milestone `name`
    /// (e.g. `"DOMContentLoaded"`, `"load"`, `"networkIdle"`,
    /// `"firstMeaningfulPaint"`).
    ///
    /// Requires [`enable`](Self::enable) **and**
    /// [`set_lifecycle_events_enabled(true)`](Self::set_lifecycle_events_enabled).
    /// Arm the wait *before* triggering the navigation/action to avoid missing
    /// the event.
    pub async fn wait_for_lifecycle(&self, name: &str, timeout: Duration) -> Result<()> {
        let mut events = self.events();
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            tokio::select! {
                evt = events.recv() => match evt {
                    Some(PageEvent::LifecycleEvent(le)) if le.name == name => return Ok(()),
                    Some(_) => continue,
                    None => return Err(CdpError::ConnectionClosed),
                },
                _ = tokio::time::sleep_until(deadline) => return Err(CdpError::Timeout(timeout)),
            }
        }
    }

    /// Waits until the network is idle (no in-flight requests for ~500ms),
    /// signalled by the `networkIdle` lifecycle milestone.
    ///
    /// Prefer this over `wait_for_load` + a fixed sleep: it tracks actual
    /// request activity instead of guessing. Same requirements as
    /// [`wait_for_lifecycle`](Self::wait_for_lifecycle).
    pub async fn wait_for_network_idle(&self, timeout: Duration) -> Result<()> {
        self.wait_for_lifecycle("networkIdle", timeout).await
    }

    /// Polls `location.href` (~200ms interval) until it contains `substring`,
    /// returning the full URL. Times out otherwise.
    ///
    /// Robust for SPA navigations (History API) that never fire a `load`
    /// event — the situation that forces ad-hoc URL polling in callers.
    pub async fn wait_for_url(&self, substring: &str, timeout: Duration) -> Result<String> {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            if let Some(href) = self.eval_value("location.href").await?.as_str()
                && href.contains(substring)
            {
                return Ok(href.to_owned());
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(CdpError::Timeout(timeout));
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    }

    /// Polls a JS `expression` in the page's global scope (~150ms interval)
    /// until it evaluates to a truthy value (JS semantics: anything but
    /// `null`/`undefined`/`false`/`0`/`NaN`/`""`), then returns. Times out
    /// otherwise.
    ///
    /// Use to await readiness that is neither a navigation nor a network event
    /// but *some JS having run* — framework hydration, a global flag, a
    /// computed property, etc. (e.g. `"!!(window.ng && app.ready)"`). For
    /// plain element presence prefer the DOM's `wait_for`.
    ///
    /// Write the expression defensively (guard against `undefined`): a thrown
    /// exception propagates as an error rather than counting as "not ready".
    /// The expression must be synchronous — a returned Promise is itself a
    /// truthy object and would resolve the wait immediately.
    pub async fn wait_for_function(&self, expression: &str, timeout: Duration) -> Result<()> {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            if js_truthy(&self.eval_value(expression).await?) {
                return Ok(());
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(CdpError::Timeout(timeout));
            }
            tokio::time::sleep(Duration::from_millis(150)).await;
        }
    }
}

/// JS truthiness of a JSON value — mirrors `if (value)` in the browser:
/// everything is truthy except `null`, `false`, `0`/`NaN`, and `""`.
/// Arrays and objects (including `{}` / `[]`) are truthy, as in JS.
fn js_truthy(v: &serde_json::Value) -> bool {
    match v {
        serde_json::Value::Null => false,
        serde_json::Value::Bool(b) => *b,
        serde_json::Value::Number(n) => n.as_f64().map(|f| f != 0.0 && !f.is_nan()).unwrap_or(true),
        serde_json::Value::String(s) => !s.is_empty(),
        _ => true,
    }
}

/// Builds `(<function>).apply(undefined, <json-args>)`. Arguments are emitted
/// as a JSON array literal — valid JS for data values — so callers never escape
/// strings by hand and untrusted values can't break out of the expression.
fn apply_expr(function: &str, args: &[serde_json::Value]) -> Result<String> {
    let json = serde_json::to_string(args)?;
    Ok(format!("({function}).apply(undefined,{json})"))
}

// ── HTML beautifier for debug dumps ─────────────────────────────────────────

fn beautify_html(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut indent: usize = 0;
    let mut pos = 0;
    let bytes = html.as_bytes();

    while pos < bytes.len() {
        if bytes[pos] == b'<' {
            let tag_end = match html[pos..].find('>') {
                Some(i) => pos + i + 1,
                None => break,
            };
            let tag = &html[pos..tag_end];

            let tag_lower = tag.to_ascii_lowercase();
            if tag_lower.starts_with("<link") {
                pos = tag_end;
                continue;
            }
            if tag_lower.starts_with("<script") || tag_lower.starts_with("<style") {
                let close = if tag_lower.starts_with("<script") {
                    "</script>"
                } else {
                    "</style>"
                };
                if let Some(end) = html[tag_end..].to_ascii_lowercase().find(close) {
                    pos = tag_end + end + close.len();
                } else {
                    pos = bytes.len();
                }
                continue;
            }

            let is_close = tag.starts_with("</");
            let is_void = tag.ends_with("/>") || is_void_tag(tag);

            if is_close {
                indent = indent.saturating_sub(1);
            }

            for _ in 0..indent {
                out.push_str("  ");
            }
            out.push_str(tag);
            out.push('\n');

            if !is_close && !is_void {
                indent += 1;
            }

            pos = tag_end;
        } else {
            let text_end = html[pos..]
                .find('<')
                .map(|i| pos + i)
                .unwrap_or(bytes.len());
            let text = html[pos..text_end].trim();
            if !text.is_empty() {
                for _ in 0..indent {
                    out.push_str("  ");
                }
                out.push_str(text);
                out.push('\n');
            }
            pos = text_end;
        }
    }

    out
}

fn is_void_tag(tag: &str) -> bool {
    const VOIDS: &[&str] = &[
        "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source",
        "track", "wbr",
    ];
    let name = tag
        .trim_start_matches('<')
        .split(|c: char| c.is_whitespace() || c == '>' || c == '/')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    VOIDS.contains(&name.as_str())
}

impl Drop for PageSession {
    fn drop(&mut self) {
        let target = self.target.clone();
        let session_id = self.session_id.clone();
        tokio::spawn(async move {
            let _ = target
                .browser_session
                .target_detach_from_target(&DetachFromTargetParams {
                    session_id: Some(session_id),
                    ..Default::default()
                })
                .await;
        });
    }
}
