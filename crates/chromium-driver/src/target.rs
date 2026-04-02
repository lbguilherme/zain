use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tokio::sync::OnceCell;

use crate::cdp::target::{AttachToTargetParams, TargetCommands};
use crate::dom::Dom;
use crate::error::Result;
use crate::page::PageSession;
use crate::session::CdpSession;
use crate::types::TargetId;

pub(crate) struct TargetInner {
    pub(crate) target_id: TargetId,
    pub(crate) browser_session: CdpSession,
    pub(crate) dom: OnceCell<Dom>,
    closed: AtomicBool,
}

impl TargetInner {
    pub(crate) fn mark_closed(&self) {
        self.closed.store(true, Ordering::Release);
    }
}

impl Drop for TargetInner {
    fn drop(&mut self) {
        if self.closed.load(Ordering::Acquire) {
            return;
        }
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
                closed: AtomicBool::new(false),
            }),
        }
    }

    /// Returns the unique identifier of this target in the browser.
    pub fn id(&self) -> &TargetId {
        &self.inner.target_id
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
        let session = self.inner.browser_session.for_session(ret.session_id.clone());
        Ok(PageSession::new(session, ret.session_id, self.inner.clone()))
    }

    /// Closes this target (tab) immediately.
    ///
    /// Marks the target as closed so the automatic cleanup on drop is skipped.
    /// Returns `true` if the target was closed successfully.
    #[allow(deprecated)]
    pub async fn close(&self) -> Result<bool> {
        self.inner.mark_closed();
        let ret = self
            .inner
            .browser_session
            .target_close_target(&self.inner.target_id)
            .await?;
        Ok(ret.success)
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
