use std::sync::Arc;

use crate::cdp::browser::{
    Bounds, BrowserCommands, CancelDownloadParams, DownloadBehavior, GetVersionReturn,
    GetWindowForTargetParams, PermissionDescriptor, PermissionSetting, ResetPermissionsParams,
    SetDownloadBehaviorParams, SetPermissionParams, SetWindowBoundsParams, WindowId,
};
use crate::cdp::target::{
    AttachedToTargetEvent, CreateBrowserContextParams, CreateTargetParams, DetachedFromTargetEvent,
    GetTargetInfoParams, GetTargetsParams, TargetCommands, TargetCreatedEvent,
    TargetDestroyedEvent, TargetInfoChangedEvent,
};
use crate::error::Result;
use crate::session::{CdpEventStream, CdpSession};
use crate::target::PageTarget;
use crate::types::{BrowserContextId, TargetId, TargetInfo};

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

    /// Waits for an event matching `predicate`, with a timeout.
    ///
    /// Returns the matching event, or `CdpError::Timeout` if the deadline
    /// is reached. Non-matching events are discarded.
    pub async fn wait_for(
        &mut self,
        predicate: impl Fn(&BrowserEvent) -> bool,
        timeout: std::time::Duration,
    ) -> crate::Result<BrowserEvent> {
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

    fn parse(raw: crate::transport::CdpEvent) -> BrowserEvent {
        match raw.method.as_str() {
            "Target.targetCreated" => match serde_json::from_value(raw.params.clone()) {
                Ok(e) => BrowserEvent::TargetCreated(e),
                Err(_) => BrowserEvent::Other {
                    method: raw.method,
                    params: raw.params,
                },
            },
            "Target.targetDestroyed" => match serde_json::from_value(raw.params.clone()) {
                Ok(e) => BrowserEvent::TargetDestroyed(e),
                Err(_) => BrowserEvent::Other {
                    method: raw.method,
                    params: raw.params,
                },
            },
            "Target.targetInfoChanged" => match serde_json::from_value(raw.params.clone()) {
                Ok(e) => BrowserEvent::TargetInfoChanged(e),
                Err(_) => BrowserEvent::Other {
                    method: raw.method,
                    params: raw.params,
                },
            },
            "Target.attachedToTarget" => match serde_json::from_value(raw.params.clone()) {
                Ok(e) => BrowserEvent::AttachedToTarget(e),
                Err(_) => BrowserEvent::Other {
                    method: raw.method,
                    params: raw.params,
                },
            },
            "Target.detachedFromTarget" => match serde_json::from_value(raw.params.clone()) {
                Ok(e) => BrowserEvent::DetachedFromTarget(e),
                Err(_) => BrowserEvent::Other {
                    method: raw.method,
                    params: raw.params,
                },
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

    // ── Permissions ──────────────────────────────────────────────────────

    /// Grants, denies or resets a browser permission for all origins.
    ///
    /// - `name`: permission name (e.g. `"geolocation"`, `"notifications"`, `"clipboard-read"`).
    /// - `setting`: whether to grant, deny or prompt.
    ///
    /// See [`PermissionSetting`] for possible values.
    pub async fn set_permission(&self, name: &str, setting: PermissionSetting) -> Result<()> {
        self.session
            .browser_set_permission(&SetPermissionParams {
                permission: PermissionDescriptor {
                    name: name.to_owned(),
                    ..Default::default()
                },
                setting,
                origin: None,
                embedded_origin: None,
                browser_context_id: None,
            })
            .await
    }

    /// Grants, denies or resets a browser permission for a specific origin.
    pub async fn set_permission_for_origin(
        &self,
        name: &str,
        setting: PermissionSetting,
        origin: &str,
    ) -> Result<()> {
        self.session
            .browser_set_permission(&SetPermissionParams {
                permission: PermissionDescriptor {
                    name: name.to_owned(),
                    ..Default::default()
                },
                setting,
                origin: Some(origin.to_owned()),
                embedded_origin: None,
                browser_context_id: None,
            })
            .await
    }

    /// Resets all permission overrides.
    pub async fn reset_permissions(&self) -> Result<()> {
        self.session
            .browser_reset_permissions(&ResetPermissionsParams::default())
            .await
    }

    // ── Downloads ─────────────────────────────────────────────────────────

    /// Configures the download behavior for the browser.
    ///
    /// - `behavior`: allow, deny, or default.
    /// - `download_path`: directory to save files (required for `Allow` / `AllowAndName`).
    pub async fn set_download_behavior(
        &self,
        behavior: DownloadBehavior,
        download_path: Option<&str>,
    ) -> Result<()> {
        self.session
            .browser_set_download_behavior(&SetDownloadBehaviorParams {
                behavior,
                browser_context_id: None,
                download_path: download_path.map(|s| s.to_owned()),
                events_enabled: Some(true),
            })
            .await
    }

    /// Cancels a download in progress by its GUID.
    pub async fn cancel_download(&self, guid: &str) -> Result<()> {
        self.session
            .browser_cancel_download(&CancelDownloadParams {
                guid: guid.to_owned(),
                browser_context_id: None,
            })
            .await
    }

    // ── Target info ───────────────────────────────────────────────────────

    /// Returns detailed information about a specific target.
    pub async fn get_target_info(&self, target_id: &TargetId) -> Result<TargetInfo> {
        let ret = self
            .session
            .target_get_target_info(&GetTargetInfoParams {
                target_id: Some(target_id.clone()),
            })
            .await?;
        Ok(ret.target_info)
    }

    // ── Window management ─────────────────────────────────────────────────

    /// Returns the window ID and bounds for the given target.
    pub async fn get_window_for_target(&self, target_id: &TargetId) -> Result<(WindowId, Bounds)> {
        let ret = self
            .session
            .browser_get_window_for_target(&GetWindowForTargetParams {
                target_id: Some(target_id.clone()),
            })
            .await?;
        Ok((ret.window_id, ret.bounds))
    }

    /// Sets position and/or size of a browser window.
    pub async fn set_window_bounds(&self, window_id: WindowId, bounds: Bounds) -> Result<()> {
        self.session
            .browser_set_window_bounds(&SetWindowBoundsParams { window_id, bounds })
            .await
    }

    // ── Browser contexts (incognito) ──────────────────────────────────────

    /// Creates a new browser context (similar to an incognito profile).
    ///
    /// The returned [`BrowserContext`] automatically disposes itself when
    /// dropped, closing all pages that belong to it.
    pub async fn create_context(&self) -> Result<BrowserContext> {
        let ret = self
            .session
            .target_create_browser_context(&CreateBrowserContextParams {
                dispose_on_detach: Some(true),
                ..Default::default()
            })
            .await?;
        Ok(BrowserContext {
            inner: Arc::new(BrowserContextInner {
                context_id: ret.browser_context_id,
                session: self.session.clone(),
                browser: self.clone(),
            }),
        })
    }

    /// Direct access to the raw CDP session for commands not covered by the typed API.
    ///
    /// Useful for accessing CDP domains or methods that don't have a wrapper yet,
    /// such as `Target.setDiscoverTargets` or experimental domains.
    pub fn cdp(&self) -> &CdpSession {
        &self.session
    }
}

// ── BrowserContext (incognito RAII) ─────────────────────────────────────────

pub(crate) struct BrowserContextInner {
    context_id: BrowserContextId,
    session: CdpSession,
    browser: Browser,
}

impl Drop for BrowserContextInner {
    fn drop(&mut self) {
        let session = self.session.clone();
        let context_id = self.context_id.clone();
        tokio::spawn(async move {
            let _ = session.target_dispose_browser_context(&context_id).await;
        });
    }
}

/// An isolated browser context (similar to incognito mode).
///
/// Pages created via [`create_page`](Self::create_page) are scoped to this
/// context and share cookies/storage only with each other.
///
/// The context is automatically disposed (and all its pages closed) when
/// every `BrowserContext` handle **and** every [`PageTarget`] created from
/// it are dropped. This is pure RAII — there is no explicit close method.
#[derive(Clone)]
pub struct BrowserContext {
    inner: Arc<BrowserContextInner>,
}

impl BrowserContext {
    /// Returns the unique identifier of this browser context.
    pub fn id(&self) -> &BrowserContextId {
        &self.inner.context_id
    }

    /// Creates a new page (tab) within this isolated context.
    ///
    /// The returned [`PageTarget`] holds a reference to this context,
    /// keeping it alive until all pages (and the `BrowserContext` itself)
    /// are dropped.
    pub async fn create_page(&self, url: &str) -> Result<PageTarget> {
        let ret = self
            .inner
            .session
            .target_create_target(&CreateTargetParams {
                url: url.to_owned(),
                browser_context_id: Some(self.inner.context_id.clone()),
                ..Default::default()
            })
            .await?;

        Ok(PageTarget::new_with_context(
            self.inner.browser.clone(),
            ret.target_id,
            self.inner.clone(),
        ))
    }
}
