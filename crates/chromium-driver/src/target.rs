use std::sync::Arc;
use tokio::sync::OnceCell;

use crate::browser::BrowserContextInner;
use crate::cdp::emulation::{EmulationCommands, SetTouchEmulationEnabledParams};
use crate::cdp::target::{AttachToTargetParams, GetTargetInfoParams, TargetCommands};
use crate::dom::Dom;
use crate::error::Result;
use crate::page::PageSession;
use crate::session::CdpSession;
use crate::types::{TargetId, TargetInfo};

pub(crate) struct TargetInner {
    pub(crate) target_id: TargetId,
    pub(crate) browser_session: CdpSession,
    pub(crate) dom: OnceCell<Dom>,
    /// Holds an Arc to the parent BrowserContext (if any), preventing it
    /// from being dropped (and disposed) while this target is alive.
    pub(crate) _context: Option<Arc<BrowserContextInner>>,
}

impl Drop for TargetInner {
    fn drop(&mut self) {
        let browser_session = self.browser_session.clone();
        let target_id = self.target_id.clone();
        tokio::spawn(async move {
            let _ = browser_session.target_close_target(&target_id).await;
        });
    }
}

/// Reference to a page target (tab) in the browser.
///
/// Obtained via [`Browser::create_page`](crate::Browser::create_page). Represents
/// a "page" type target that exists in the browser.
///
/// Call [`attach`](Self::attach) to get a [`PageSession`] and interact with
/// the page (navigate, listen to events, etc.).
///
/// The tab is automatically closed when all references (this `PageTarget` and
/// any [`PageSession`]s created from it) are dropped.
pub struct PageTarget {
    inner: Arc<TargetInner>,
}

impl PageTarget {
    pub(crate) fn new(browser: crate::browser::Browser, target_id: TargetId) -> Self {
        Self {
            inner: Arc::new(TargetInner {
                target_id,
                browser_session: browser.cdp().clone(),
                dom: OnceCell::new(),
                _context: None,
            }),
        }
    }

    pub(crate) fn new_with_context(
        browser: crate::browser::Browser,
        target_id: TargetId,
        context: Arc<BrowserContextInner>,
    ) -> Self {
        Self {
            inner: Arc::new(TargetInner {
                target_id,
                browser_session: browser.cdp().clone(),
                dom: OnceCell::new(),
                _context: Some(context),
            }),
        }
    }

    /// Returns the unique identifier of this target in the browser.
    pub fn id(&self) -> &TargetId {
        &self.inner.target_id
    }

    /// Returns detailed information about this target (URL, title, type, etc.).
    ///
    /// Does not require the target to be attached.
    pub async fn info(&self) -> Result<TargetInfo> {
        let ret = self
            .inner
            .browser_session
            .target_get_target_info(&GetTargetInfoParams {
                target_id: Some(self.inner.target_id.clone()),
            })
            .await?;
        Ok(ret.target_info)
    }

    /// Attaches a CDP session to this target using flattened mode.
    ///
    /// In flattened mode, session messages are multiplexed over the browser's
    /// WebSocket connection (identified by `sessionId`), without needing a
    /// separate WebSocket.
    ///
    /// Returns a [`PageSession`] with navigation, reload, history and event methods.
    /// The returned session holds a reference to this target, keeping it alive.
    pub async fn attach(&self) -> Result<PageSession> {
        let params = AttachToTargetParams {
            target_id: self.inner.target_id.clone(),
            flatten: Some(true),
        };
        let ret = self
            .inner
            .browser_session
            .target_attach_to_target(&params)
            .await?;
        let session = self
            .inner
            .browser_session
            .for_session(ret.session_id.clone());

        // Enable touch emulation ONCE, eagerly, right after attaching and before
        // any navigation. Two reasons:
        // 1. The input model is touch-based — clicks/swipes dispatch
        //    `Input.dispatchTouchEvent`, so the page must advertise touch support
        //    for those events to be honored.
        // 2. `navigator.maxTouchPoints` is then a stable `1` from the first script
        //    every page runs, instead of mutating 0→1 the first time `dom()` is
        //    called mid-session — a transition no real device exhibits (proven in
        //    tests/fingerprint_probe.rs: two reads on one document gave 0 then 1).
        session
            .emulation_set_touch_emulation_enabled(&SetTouchEmulationEnabledParams {
                enabled: true,
                max_touch_points: Some(1),
            })
            .await?;

        // Present as São Paulo for all intents — `Date`, `getTimezoneOffset()`
        // and `Intl.DateTimeFormat().resolvedOptions().timeZone` would otherwise
        // leak the host's real timezone, incoherent with the pt-BR locale and
        // the Brazilian sites these automate. Set here (before navigation) so it
        // is stable from the first script.
        session
            .emulation_set_timezone_override("America/Sao_Paulo")
            .await?;

        Ok(PageSession::new(
            session,
            ret.session_id,
            self.inner.clone(),
        ))
    }

    /// Brings this target to the foreground in the browser.
    ///
    /// In headless mode this has no visual effect, but updates the browser's
    /// internal state about which target is active.
    pub async fn activate(&self) -> Result<()> {
        self.inner
            .browser_session
            .target_activate_target(&self.inner.target_id)
            .await
    }
}
