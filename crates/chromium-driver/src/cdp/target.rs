use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::session::CdpSession;
use crate::types::{BrowserContextId, SessionId, TargetId, TargetInfo};

// ── Types ───────────────────────────────────────────────────────────────────

/// A filter used by target query/discovery/auto-attach operations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilterEntry {
    /// If set, causes exclusion of matching targets from the list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude: Option<bool>,
    /// If not present, matches any type.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    pub target_type: Option<String>,
}

/// The entries in TargetFilter are matched sequentially against targets and
/// the first entry that matches determines if the target is included or not,
/// depending on the value of `exclude` field in the entry.
/// If filter is not specified, the one assumed is
/// [{type: "browser", exclude: true}, {type: "tab", exclude: true}, {}]
/// (i.e. include everything but `browser` and `tab`).
pub type TargetFilter = Vec<FilterEntry>;

/// Remote location for target discovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteLocation {
    pub host: String,
    pub port: i64,
}

/// The state of the target window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WindowState {
    Normal,
    Minimized,
    Maximized,
    Fullscreen,
}

// ── Param types ─────────────────────────────────────────────────────────────

/// Parameters for [`TargetCommands::target_attach_to_target`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachToTargetParams {
    /// ID of the target to attach to.
    pub target_id: TargetId,
    /// Enables "flat" access to the session via specifying sessionId attribute in the commands.
    /// We plan to make this the default, deprecate non-flattened mode,
    /// and eventually retire it. See crbug.com/991325.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flatten: Option<bool>,
}

/// Parameters for [`TargetCommands::target_expose_dev_tools_protocol`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExposeDevToolsProtocolParams {
    pub target_id: TargetId,
    /// Binding name, 'cdp' if not specified.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binding_name: Option<String>,
    /// If true, inherits the current root session's permissions (default: false).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inherit_permissions: Option<bool>,
}

/// Parameters for [`TargetCommands::target_create_browser_context`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateBrowserContextParams {
    /// If specified, disposes this context when debugging session disconnects.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dispose_on_detach: Option<bool>,
    /// Proxy server, similar to the one passed to --proxy-server
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy_server: Option<String>,
    /// Proxy bypass list, similar to the one passed to --proxy-bypass-list
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy_bypass_list: Option<String>,
    /// An optional list of origins to grant unlimited cross-origin access to.
    /// Parts of the URL other than those constituting origin are ignored.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origins_with_universal_network_access: Option<Vec<String>>,
}

/// Parameters for [`TargetCommands::target_create_target`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTargetParams {
    /// The initial URL the page will be navigated to. An empty string indicates about:blank.
    pub url: String,
    /// Frame left origin in DIP (requires newWindow to be true or headless shell).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left: Option<i64>,
    /// Frame top origin in DIP (requires newWindow to be true or headless shell).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top: Option<i64>,
    /// Frame width in DIP (requires newWindow to be true or headless shell).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<i64>,
    /// Frame height in DIP (requires newWindow to be true or headless shell).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<i64>,
    /// Frame window state (requires newWindow to be true or headless shell).
    /// Default is normal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_state: Option<WindowState>,
    /// The browser context to create the page in.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser_context_id: Option<BrowserContextId>,
    /// Whether BeginFrames for this target will be controlled via DevTools (headless shell only,
    /// not supported on MacOS yet, false by default).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_begin_frame_control: Option<bool>,
    /// Whether to create a new Window or Tab (false by default, not supported by headless shell).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_window: Option<bool>,
    /// Whether to create the target in background or foreground (false by default, not supported
    /// by headless shell).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<bool>,
    /// Whether to create the target of type "tab".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub for_tab: Option<bool>,
    /// Whether to create a hidden target. The hidden target is observable via protocol, but not
    /// present in the tab UI strip. Cannot be created with `forTab: true`, `newWindow: true` or
    /// `background: false`. The life-time of the tab is limited to the life-time of the session.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
    /// If specified, the option is used to determine if the new target should
    /// be focused or not. By default, the focus behavior depends on the
    /// value of the background field.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focus: Option<bool>,
}

/// Parameters for [`TargetCommands::target_get_target_info`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTargetInfoParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_id: Option<TargetId>,
}

/// Parameters for [`TargetCommands::target_get_targets`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTargetsParams {
    /// Only targets matching filter will be reported. If filter is not specified
    /// and target discovery is currently enabled, a filter used for target discovery
    /// is used for consistency.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<TargetFilter>,
}

/// Parameters for [`TargetCommands::target_set_auto_attach`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetAutoAttachParams {
    /// Whether to auto-attach to related targets.
    pub auto_attach: bool,
    /// Whether to pause new targets when attaching to them. Use `Runtime.runIfWaitingForDebugger`
    /// to run paused targets.
    pub wait_for_debugger_on_start: bool,
    /// Enables "flat" access to the session via specifying sessionId attribute in the commands.
    /// We plan to make this the default, deprecate non-flattened mode,
    /// and eventually retire it. See crbug.com/991325.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flatten: Option<bool>,
    /// Only targets matching filter will be attached.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<TargetFilter>,
}

/// Parameters for [`TargetCommands::target_auto_attach_related`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoAttachRelatedParams {
    pub target_id: TargetId,
    /// Whether to pause new targets when attaching to them. Use `Runtime.runIfWaitingForDebugger`
    /// to run paused targets.
    pub wait_for_debugger_on_start: bool,
    /// Only targets matching filter will be attached.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<TargetFilter>,
}

/// Parameters for [`TargetCommands::target_set_discover_targets`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetDiscoverTargetsParams {
    /// Whether to discover available targets.
    pub discover: bool,
    /// Only targets matching filter will be attached. If `discover` is false,
    /// `filter` must be omitted or empty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<TargetFilter>,
}

/// Parameters for [`TargetCommands::target_open_dev_tools`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenDevToolsParams {
    /// This can be the page or tab target ID.
    pub target_id: TargetId,
    /// The id of the panel we want DevTools to open initially. Currently
    /// supported panels are elements, console, network, sources, resources
    /// and performance.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub panel_id: Option<String>,
}

// ── Return types ────────────────────────────────────────────────────────────

/// Return type for [`TargetCommands::target_attach_to_target`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachToTargetReturn {
    /// Id assigned to the session.
    pub session_id: SessionId,
}

/// Return type for [`TargetCommands::target_attach_to_browser_target`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachToBrowserTargetReturn {
    /// Id assigned to the session.
    pub session_id: SessionId,
}

/// Return type for [`TargetCommands::target_close_target`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloseTargetReturn {
    /// Always set to true. If an error occurs, the response indicates protocol error.
    #[deprecated]
    pub success: bool,
}

/// Return type for [`TargetCommands::target_create_browser_context`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateBrowserContextReturn {
    /// The id of the context created.
    pub browser_context_id: BrowserContextId,
}

/// Return type for [`TargetCommands::target_get_browser_contexts`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBrowserContextsReturn {
    /// An array of browser context ids.
    pub browser_context_ids: Vec<BrowserContextId>,
    /// The id of the default browser context if available.
    #[serde(default)]
    pub default_browser_context_id: Option<BrowserContextId>,
}

/// Return type for [`TargetCommands::target_create_target`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTargetReturn {
    /// The id of the page opened.
    pub target_id: TargetId,
}

/// Return type for [`TargetCommands::target_get_target_info`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTargetInfoReturn {
    pub target_info: TargetInfo,
}

/// Return type for [`TargetCommands::target_get_targets`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTargetsReturn {
    /// The list of targets.
    pub target_infos: Vec<TargetInfo>,
}

/// Return type for [`TargetCommands::target_get_dev_tools_target`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetDevToolsTargetReturn {
    /// The targetId of DevTools page target if exists.
    #[serde(default)]
    pub target_id: Option<TargetId>,
}

/// Return type for [`TargetCommands::target_open_dev_tools`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenDevToolsReturn {
    /// The targetId of DevTools page target.
    pub target_id: TargetId,
}

// ── Events ──────────────────────────────────────────────────────────────────

/// Issued when attached to target because of auto-attach or `attachToTarget` command.
///
/// CDP: `Target.attachedToTarget`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachedToTargetEvent {
    /// Identifier assigned to the session used to send/receive messages.
    pub session_id: SessionId,
    pub target_info: TargetInfo,
    pub waiting_for_debugger: bool,
}

/// Issued when detached from target for any reason (including `detachFromTarget` command). Can be
/// issued multiple times per target if multiple sessions have been attached to it.
///
/// CDP: `Target.detachedFromTarget`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetachedFromTargetEvent {
    /// Detached session identifier.
    pub session_id: SessionId,
}

/// Notifies about a new protocol message received from the session (as reported in
/// `attachedToTarget` event).
///
/// CDP: `Target.receivedMessageFromTarget`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceivedMessageFromTargetEvent {
    /// Identifier of a session which sends a message.
    pub session_id: SessionId,
    pub message: String,
}

/// Issued when a possible inspection target is created.
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

/// Issued when a target has crashed.
///
/// CDP: `Target.targetCrashed`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetCrashedEvent {
    pub target_id: TargetId,
    /// Termination status type.
    pub status: String,
    /// Termination error code.
    pub error_code: i64,
}

/// Issued when some information about a target has changed. This only happens between
/// `targetCreated` and `targetDestroyed`.
///
/// CDP: `Target.targetInfoChanged`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetInfoChangedEvent {
    pub target_info: TargetInfo,
}

// ── Domain trait ─────────────────────────────────────────────────────────────

/// `Target` domain CDP methods.
///
/// Supports additional targets discovery and allows to attach to them.
///
/// Reference: <https://chromedevtools.github.io/devtools-protocol/tot/Target/>
pub trait TargetCommands {
    /// Activates (focuses) the target.
    ///
    /// CDP: `Target.activateTarget`
    async fn target_activate_target(&self, target_id: &TargetId) -> Result<()>;

    /// Attaches to the target with given id.
    ///
    /// CDP: `Target.attachToTarget`
    async fn target_attach_to_target(
        &self,
        params: &AttachToTargetParams,
    ) -> Result<AttachToTargetReturn>;

    /// Attaches to the browser target, only uses flat sessionId mode.
    ///
    /// CDP: `Target.attachToBrowserTarget`
    async fn target_attach_to_browser_target(&self) -> Result<AttachToBrowserTargetReturn>;

    /// Closes the target. If the target is a page that gets closed too.
    ///
    /// CDP: `Target.closeTarget`
    async fn target_close_target(&self, target_id: &TargetId) -> Result<CloseTargetReturn>;

    /// Inject object to the target's main frame that provides a communication
    /// channel with browser target.
    ///
    /// Injected object will be available as `window[bindingName]`.
    ///
    /// The object has the following API:
    /// - `binding.send(json)` - a method to send messages over the remote debugging protocol
    /// - `binding.onmessage = json => handleMessage(json)` - a callback that will be called for the protocol notifications and command responses.
    ///
    /// CDP: `Target.exposeDevToolsProtocol`
    async fn target_expose_dev_tools_protocol(
        &self,
        params: &ExposeDevToolsProtocolParams,
    ) -> Result<()>;

    /// Creates a new empty BrowserContext. Similar to an incognito profile but you can have more than
    /// one.
    ///
    /// CDP: `Target.createBrowserContext`
    async fn target_create_browser_context(
        &self,
        params: &CreateBrowserContextParams,
    ) -> Result<CreateBrowserContextReturn>;

    /// Returns all browser contexts created with `Target.createBrowserContext` method.
    ///
    /// CDP: `Target.getBrowserContexts`
    async fn target_get_browser_contexts(&self) -> Result<GetBrowserContextsReturn>;

    /// Creates a new page.
    ///
    /// CDP: `Target.createTarget`
    async fn target_create_target(
        &self,
        params: &CreateTargetParams,
    ) -> Result<CreateTargetReturn>;

    /// Detaches session with given id.
    ///
    /// CDP: `Target.detachFromTarget`
    async fn target_detach_from_target(&self, session_id: &SessionId) -> Result<()>;

    /// Deletes a BrowserContext. All the belonging pages will be closed without calling their
    /// beforeunload hooks.
    ///
    /// CDP: `Target.disposeBrowserContext`
    async fn target_dispose_browser_context(
        &self,
        browser_context_id: &BrowserContextId,
    ) -> Result<()>;

    /// Returns information about a target.
    ///
    /// CDP: `Target.getTargetInfo`
    async fn target_get_target_info(
        &self,
        params: &GetTargetInfoParams,
    ) -> Result<GetTargetInfoReturn>;

    /// Retrieves a list of available targets.
    ///
    /// CDP: `Target.getTargets`
    async fn target_get_targets(&self, params: &GetTargetsParams) -> Result<GetTargetsReturn>;

    /// Controls whether to automatically attach to new targets which are considered
    /// to be directly related to this one (for example, iframes or workers).
    /// When turned on, attaches to all existing related targets as well. When turned off,
    /// automatically detaches from all currently attached targets.
    /// This also clears all targets added by `autoAttachRelated` from the list of targets to watch
    /// for creation of related targets.
    /// You might want to call this recursively for auto-attached targets to attach
    /// to all available targets.
    ///
    /// CDP: `Target.setAutoAttach`
    async fn target_set_auto_attach(&self, params: &SetAutoAttachParams) -> Result<()>;

    /// Adds the specified target to the list of targets that will be monitored for any related target
    /// creation (such as child frames, child workers and new versions of service worker) and reported
    /// through `attachedToTarget`. The specified target is also auto-attached.
    /// This cancels the effect of any previous `setAutoAttach` and is also cancelled by subsequent
    /// `setAutoAttach`. Only available at the Browser target.
    ///
    /// CDP: `Target.autoAttachRelated`
    async fn target_auto_attach_related(&self, params: &AutoAttachRelatedParams) -> Result<()>;

    /// Controls whether to discover available targets and notify via
    /// `targetCreated/targetInfoChanged/targetDestroyed` events.
    ///
    /// CDP: `Target.setDiscoverTargets`
    async fn target_set_discover_targets(
        &self,
        params: &SetDiscoverTargetsParams,
    ) -> Result<()>;

    /// Enables target discovery for the specified locations, when `setDiscoverTargets` was set to
    /// `true`.
    ///
    /// CDP: `Target.setRemoteLocations`
    async fn target_set_remote_locations(&self, locations: &[RemoteLocation]) -> Result<()>;

    /// Gets the targetId of the DevTools page target opened for the given target
    /// (if any).
    ///
    /// CDP: `Target.getDevToolsTarget`
    async fn target_get_dev_tools_target(
        &self,
        target_id: &TargetId,
    ) -> Result<GetDevToolsTargetReturn>;

    /// Opens a DevTools window for the target.
    ///
    /// CDP: `Target.openDevTools`
    async fn target_open_dev_tools(
        &self,
        params: &OpenDevToolsParams,
    ) -> Result<OpenDevToolsReturn>;
}

// ── Impl ────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ActivateTargetInternalParams<'a> {
    target_id: &'a TargetId,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CloseTargetInternalParams<'a> {
    target_id: &'a TargetId,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DetachFromTargetInternalParams<'a> {
    session_id: &'a SessionId,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DisposeBrowserContextInternalParams<'a> {
    browser_context_id: &'a BrowserContextId,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetRemoteLocationsInternalParams<'a> {
    locations: &'a [RemoteLocation],
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GetDevToolsTargetInternalParams<'a> {
    target_id: &'a TargetId,
}

impl TargetCommands for CdpSession {
    async fn target_activate_target(&self, target_id: &TargetId) -> Result<()> {
        let params = ActivateTargetInternalParams { target_id };
        self.call_no_response("Target.activateTarget", &params)
            .await
    }

    async fn target_attach_to_target(
        &self,
        params: &AttachToTargetParams,
    ) -> Result<AttachToTargetReturn> {
        self.call("Target.attachToTarget", params).await
    }

    async fn target_attach_to_browser_target(&self) -> Result<AttachToBrowserTargetReturn> {
        self.call("Target.attachToBrowserTarget", &serde_json::json!({}))
            .await
    }

    async fn target_close_target(&self, target_id: &TargetId) -> Result<CloseTargetReturn> {
        let params = CloseTargetInternalParams { target_id };
        self.call("Target.closeTarget", &params).await
    }

    async fn target_expose_dev_tools_protocol(
        &self,
        params: &ExposeDevToolsProtocolParams,
    ) -> Result<()> {
        self.call_no_response("Target.exposeDevToolsProtocol", params)
            .await
    }

    async fn target_create_browser_context(
        &self,
        params: &CreateBrowserContextParams,
    ) -> Result<CreateBrowserContextReturn> {
        self.call("Target.createBrowserContext", params).await
    }

    async fn target_get_browser_contexts(&self) -> Result<GetBrowserContextsReturn> {
        self.call("Target.getBrowserContexts", &serde_json::json!({}))
            .await
    }

    async fn target_create_target(
        &self,
        params: &CreateTargetParams,
    ) -> Result<CreateTargetReturn> {
        self.call("Target.createTarget", params).await
    }

    async fn target_detach_from_target(&self, session_id: &SessionId) -> Result<()> {
        let params = DetachFromTargetInternalParams { session_id };
        self.call_no_response("Target.detachFromTarget", &params)
            .await
    }

    async fn target_dispose_browser_context(
        &self,
        browser_context_id: &BrowserContextId,
    ) -> Result<()> {
        let params = DisposeBrowserContextInternalParams {
            browser_context_id,
        };
        self.call_no_response("Target.disposeBrowserContext", &params)
            .await
    }

    async fn target_get_target_info(
        &self,
        params: &GetTargetInfoParams,
    ) -> Result<GetTargetInfoReturn> {
        self.call("Target.getTargetInfo", params).await
    }

    async fn target_get_targets(&self, params: &GetTargetsParams) -> Result<GetTargetsReturn> {
        self.call("Target.getTargets", params).await
    }

    async fn target_set_auto_attach(&self, params: &SetAutoAttachParams) -> Result<()> {
        self.call_no_response("Target.setAutoAttach", params).await
    }

    async fn target_auto_attach_related(&self, params: &AutoAttachRelatedParams) -> Result<()> {
        self.call_no_response("Target.autoAttachRelated", params)
            .await
    }

    async fn target_set_discover_targets(
        &self,
        params: &SetDiscoverTargetsParams,
    ) -> Result<()> {
        self.call_no_response("Target.setDiscoverTargets", params)
            .await
    }

    async fn target_set_remote_locations(&self, locations: &[RemoteLocation]) -> Result<()> {
        let params = SetRemoteLocationsInternalParams { locations };
        self.call_no_response("Target.setRemoteLocations", &params)
            .await
    }

    async fn target_get_dev_tools_target(
        &self,
        target_id: &TargetId,
    ) -> Result<GetDevToolsTargetReturn> {
        let params = GetDevToolsTargetInternalParams { target_id };
        self.call("Target.getDevToolsTarget", &params).await
    }

    async fn target_open_dev_tools(
        &self,
        params: &OpenDevToolsParams,
    ) -> Result<OpenDevToolsReturn> {
        self.call("Target.openDevTools", params).await
    }
}
