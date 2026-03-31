use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::session::CdpSession;
use crate::types::{BrowserContextId, SessionId, TargetId, TargetInfo};

// --- Param types ---

/// Parameters for [`TargetCommands::target_create_target`].
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTargetParams {
    /// Initial URL for navigation. Empty string defaults to `about:blank`.
    pub url: String,
    /// Frame width in DIP (device independent pixels).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    /// Frame height in DIP.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    /// Browser context in which to create the target.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser_context_id: Option<BrowserContextId>,
    /// If `true`, opens in a new window instead of a new tab. Default: `false`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_window: Option<bool>,
    /// If `true`, creates the tab in background. Default: `false`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<bool>,
}

/// Parameters for [`TargetCommands::target_attach_to_target`].
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachToTargetParams {
    /// ID of the target to attach to.
    pub target_id: TargetId,
    /// Enables flat session access via `sessionId`, multiplexing messages
    /// over the browser's WebSocket. Recommended: `Some(true)`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flatten: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CloseTargetParams {
    pub target_id: TargetId,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ActivateTargetParams {
    pub target_id: TargetId,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DetachFromTargetParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SetDiscoverTargetsParams {
    pub discover: bool,
}

// --- Return types ---

/// Return type for [`TargetCommands::target_create_target`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTargetReturn {
    /// ID of the created target.
    pub target_id: TargetId,
}

/// Return type for [`TargetCommands::target_attach_to_target`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachToTargetReturn {
    /// Session identifier assigned to the connection.
    pub session_id: SessionId,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CloseTargetReturn {
    pub success: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GetTargetsReturn {
    pub target_infos: Vec<TargetInfo>,
}

// --- Events ---

/// Issued when a target is created.
///
/// CDP: `Target.targetCreated`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetCreatedEvent {
    pub target_info: TargetInfo,
}

/// Issued when a target is destroyed.
///
/// CDP: `Target.targetDestroyed`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetDestroyedEvent {
    pub target_id: TargetId,
}

/// Issued when some information about a target has changed.
///
/// CDP: `Target.targetInfoChanged`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetInfoChangedEvent {
    pub target_info: TargetInfo,
}

/// Issued when attached to a target via auto-attach or `attachToTarget`.
///
/// CDP: `Target.attachedToTarget`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachedToTargetEvent {
    pub session_id: SessionId,
    pub target_info: TargetInfo,
    /// `true` if the target is paused waiting for the debugger.
    pub waiting_for_debugger: bool,
}

/// Issued when detached from a target.
///
/// CDP: `Target.detachedFromTarget`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetachedFromTargetEvent {
    pub session_id: SessionId,
}

// --- Domain trait ---

/// `Target` domain CDP methods.
///
/// Reference: <https://chromedevtools.github.io/devtools-protocol/tot/Target/>
pub trait TargetCommands {
    /// Creates a new page. Equivalent to opening a new tab in the browser.
    ///
    /// CDP: `Target.createTarget`
    async fn target_create_target(
        &self,
        params: &CreateTargetParams,
    ) -> Result<CreateTargetReturn>;

    /// Closes the target. Closing a page target also closes the page.
    /// Returns `true` on success.
    ///
    /// CDP: `Target.closeTarget`
    async fn target_close_target(&self, target_id: &TargetId) -> Result<bool>;

    /// Retrieves a list of available targets (pages, service workers, etc.).
    ///
    /// CDP: `Target.getTargets`
    async fn target_get_targets(&self) -> Result<Vec<TargetInfo>>;

    /// Attaches to the target, creating a bound CDP session.
    ///
    /// CDP: `Target.attachToTarget`
    async fn target_attach_to_target(
        &self,
        params: &AttachToTargetParams,
    ) -> Result<AttachToTargetReturn>;

    /// Detaches the session from the target.
    ///
    /// CDP: `Target.detachFromTarget`
    async fn target_detach_from_target(&self, session_id: &SessionId) -> Result<()>;

    /// Activates (focuses) the target.
    ///
    /// CDP: `Target.activateTarget`
    async fn target_activate_target(&self, target_id: &TargetId) -> Result<()>;

    /// Controls whether to discover available targets and notify via
    /// `targetCreated`/`targetInfoChanged`/`targetDestroyed` events.
    ///
    /// CDP: `Target.setDiscoverTargets`
    async fn target_set_discover_targets(&self, discover: bool) -> Result<()>;
}

impl TargetCommands for CdpSession {
    async fn target_create_target(
        &self,
        params: &CreateTargetParams,
    ) -> Result<CreateTargetReturn> {
        self.call("Target.createTarget", params).await
    }

    async fn target_close_target(&self, target_id: &TargetId) -> Result<bool> {
        let params = CloseTargetParams {
            target_id: target_id.clone(),
        };
        let ret: CloseTargetReturn = self.call("Target.closeTarget", &params).await?;
        Ok(ret.success)
    }

    async fn target_get_targets(&self) -> Result<Vec<TargetInfo>> {
        let ret: GetTargetsReturn =
            self.call("Target.getTargets", &serde_json::json!({})).await?;
        Ok(ret.target_infos)
    }

    async fn target_attach_to_target(
        &self,
        params: &AttachToTargetParams,
    ) -> Result<AttachToTargetReturn> {
        self.call("Target.attachToTarget", params).await
    }

    async fn target_detach_from_target(&self, session_id: &SessionId) -> Result<()> {
        let params = DetachFromTargetParams {
            session_id: Some(session_id.clone()),
        };
        self.call_no_response("Target.detachFromTarget", &params)
            .await
    }

    async fn target_activate_target(&self, target_id: &TargetId) -> Result<()> {
        let params = ActivateTargetParams {
            target_id: target_id.clone(),
        };
        self.call_no_response("Target.activateTarget", &params)
            .await
    }

    async fn target_set_discover_targets(&self, discover: bool) -> Result<()> {
        let params = SetDiscoverTargetsParams { discover };
        self.call_no_response("Target.setDiscoverTargets", &params)
            .await
    }
}
