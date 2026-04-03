use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::session::CdpSession;
use crate::types::{BrowserContextId, FrameId, TargetId};

// ── Types ───────────────────────────────────────────────────────────────────

/// Browser window identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WindowId(pub i64);

/// The state of the browser window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WindowState {
    Normal,
    Minimized,
    Maximized,
    Fullscreen,
}

/// Browser window bounds information.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bounds {
    /// The offset from the left edge of the screen to the window in pixels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left: Option<i64>,
    /// The offset from the top edge of the screen to the window in pixels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top: Option<i64>,
    /// The window width in pixels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<i64>,
    /// The window height in pixels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<i64>,
    /// The window state. Default to normal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_state: Option<WindowState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionType {
    Ar,
    AudioCapture,
    AutomaticFullscreen,
    BackgroundFetch,
    BackgroundSync,
    CameraPanTiltZoom,
    CapturedSurfaceControl,
    ClipboardReadWrite,
    ClipboardSanitizedWrite,
    DisplayCapture,
    DurableStorage,
    Geolocation,
    HandTracking,
    IdleDetection,
    KeyboardLock,
    LocalFonts,
    LocalNetwork,
    LocalNetworkAccess,
    LoopbackNetwork,
    Midi,
    MidiSysex,
    Nfc,
    Notifications,
    PaymentHandler,
    PeriodicBackgroundSync,
    PointerLock,
    ProtectedMediaIdentifier,
    Sensors,
    SmartCard,
    SpeakerSelection,
    StorageAccess,
    TopLevelStorageAccess,
    VideoCapture,
    Vr,
    WakeLockScreen,
    WakeLockSystem,
    WebAppInstallation,
    WebPrinting,
    WindowManagement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionSetting {
    Granted,
    Denied,
    Prompt,
}

/// Definition of PermissionDescriptor defined in the Permissions API:
/// https://w3c.github.io/permissions/#dom-permissiondescriptor.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionDescriptor {
    /// Name of permission.
    /// See https://cs.chromium.org/chromium/src/third_party/blink/renderer/modules/permissions/permission_descriptor.idl for valid permission names.
    pub name: String,
    /// For "midi" permission, may also specify sysex control.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sysex: Option<bool>,
    /// For "push" permission, may specify userVisibleOnly.
    /// Note that userVisibleOnly = true is the only currently supported type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_visible_only: Option<bool>,
    /// For "clipboard" permission, may specify allowWithoutSanitization.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_without_sanitization: Option<bool>,
    /// For "fullscreen" permission, must specify allowWithoutGesture:true.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_without_gesture: Option<bool>,
    /// For "camera" permission, may specify panTiltZoom.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pan_tilt_zoom: Option<bool>,
}

/// Browser command ids used by executeBrowserCommand.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BrowserCommandId {
    OpenTabSearch,
    CloseTabSearch,
    OpenGlic,
}

/// Chrome histogram bucket.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bucket {
    /// Minimum value (inclusive).
    pub low: i64,
    /// Maximum value (exclusive).
    pub high: i64,
    /// Number of samples.
    pub count: i64,
}

/// Chrome histogram.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Histogram {
    /// Name.
    pub name: String,
    /// Sum of sample values.
    pub sum: i64,
    /// Total number of samples.
    pub count: i64,
    /// Buckets.
    pub buckets: Vec<Bucket>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrivacySandboxApi {
    BiddingAndAuctionServices,
    TrustedKeyValue,
}

/// Whether to allow all or deny all download requests, or use default Chrome behavior if
/// available (otherwise deny). `AllowAndName` allows download and names files according to
/// their download guids.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DownloadBehavior {
    Deny,
    Allow,
    AllowAndName,
    Default,
}

/// Download status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DownloadProgressState {
    InProgress,
    Completed,
    Canceled,
}

// ── Param types ─────────────────────────────────────────────────────────────

/// Parameters for [`BrowserCommands::browser_set_permission`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetPermissionParams {
    /// Descriptor of permission to override.
    pub permission: PermissionDescriptor,
    /// Setting of the permission.
    pub setting: PermissionSetting,
    /// Embedding origin the permission applies to, all origins if not specified.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
    /// Embedded origin the permission applies to. It is ignored unless the embedding origin is
    /// present and valid. If the embedding origin is provided but the embedded origin isn't, the
    /// embedding origin is used as the embedded origin.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedded_origin: Option<String>,
    /// Context to override. When omitted, default browser context is used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser_context_id: Option<BrowserContextId>,
}

/// Parameters for [`BrowserCommands::browser_reset_permissions`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResetPermissionsParams {
    /// BrowserContext to reset permissions. When omitted, default browser context is used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser_context_id: Option<BrowserContextId>,
}

/// Parameters for [`BrowserCommands::browser_set_download_behavior`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetDownloadBehaviorParams {
    /// Whether to allow all or deny all download requests, or use default Chrome behavior if
    /// available (otherwise deny). `AllowAndName` allows download and names files according to
    /// their download guids.
    pub behavior: DownloadBehavior,
    /// BrowserContext to set download behavior. When omitted, default browser context is used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser_context_id: Option<BrowserContextId>,
    /// The default path to save downloaded files to. This is required if behavior is set to 'allow'
    /// or 'allowAndName'.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_path: Option<String>,
    /// Whether to emit download events (defaults to false).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub events_enabled: Option<bool>,
}

/// Parameters for [`BrowserCommands::browser_cancel_download`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelDownloadParams {
    /// Global unique identifier of the download.
    pub guid: String,
    /// BrowserContext to perform the action in. When omitted, default browser context is used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser_context_id: Option<BrowserContextId>,
}

/// Parameters for [`BrowserCommands::browser_get_histograms`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetHistogramsParams {
    /// Requested substring in name. Only histograms which have query as a
    /// substring in their name are extracted. An empty or absent query returns
    /// all histograms.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    /// If true, retrieve delta since last delta call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<bool>,
}

/// Parameters for [`BrowserCommands::browser_get_histogram`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetHistogramParams {
    /// Requested histogram name.
    pub name: String,
    /// If true, retrieve delta since last delta call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<bool>,
}

/// Parameters for [`BrowserCommands::browser_get_window_for_target`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetWindowForTargetParams {
    /// Devtools agent host id. If called as a part of the session, associated targetId is used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_id: Option<TargetId>,
}

/// Parameters for [`BrowserCommands::browser_set_window_bounds`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetWindowBoundsParams {
    /// Browser window id.
    pub window_id: WindowId,
    /// New window bounds. The 'minimized', 'maximized' and 'fullscreen' states cannot be combined
    /// with 'left', 'top', 'width' or 'height'. Leaves unspecified fields unchanged.
    pub bounds: Bounds,
}

/// Parameters for [`BrowserCommands::browser_set_contents_size`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetContentsSizeParams {
    /// Browser window id.
    pub window_id: WindowId,
    /// The window contents width in DIP. Assumes current width if omitted.
    /// Must be specified if 'height' is omitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<i64>,
    /// The window contents height in DIP. Assumes current height if omitted.
    /// Must be specified if 'width' is omitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<i64>,
}

/// Parameters for [`BrowserCommands::browser_set_dock_tile`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetDockTileParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub badge_label: Option<String>,
    /// Png encoded image. (Encoded as a base64 string when passed over JSON)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
}

/// Parameters for [`BrowserCommands::browser_add_privacy_sandbox_coordinator_key_config`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddPrivacySandboxCoordinatorKeyConfigParams {
    pub api: PrivacySandboxApi,
    pub coordinator_origin: String,
    pub key_config: String,
    /// BrowserContext to perform the action in. When omitted, default browser context is used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser_context_id: Option<BrowserContextId>,
}

// ── Return types ────────────────────────────────────────────────────────────

/// Return type for [`BrowserCommands::browser_get_version`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetVersionReturn {
    /// Protocol version.
    pub protocol_version: String,
    /// Product name.
    pub product: String,
    /// Product revision.
    pub revision: String,
    /// User-Agent.
    pub user_agent: String,
    /// V8 version.
    pub js_version: String,
}

/// Return type for [`BrowserCommands::browser_get_browser_command_line`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBrowserCommandLineReturn {
    /// Commandline parameters.
    pub arguments: Vec<String>,
}

/// Return type for [`BrowserCommands::browser_get_histograms`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetHistogramsReturn {
    /// Histograms.
    pub histograms: Vec<Histogram>,
}

/// Return type for [`BrowserCommands::browser_get_histogram`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetHistogramReturn {
    /// Histogram.
    pub histogram: Histogram,
}

/// Return type for [`BrowserCommands::browser_get_window_bounds`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetWindowBoundsReturn {
    /// Bounds information of the window. When window state is 'minimized', the restored window
    /// position and size are returned.
    pub bounds: Bounds,
}

/// Return type for [`BrowserCommands::browser_get_window_for_target`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetWindowForTargetReturn {
    /// Browser window id.
    pub window_id: WindowId,
    /// Bounds information of the window. When window state is 'minimized', the restored window
    /// position and size are returned.
    pub bounds: Bounds,
}

// ── Events ──────────────────────────────────────────────────────────────────

/// Fired when page is about to start a download.
///
/// CDP: `Browser.downloadWillBegin`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadWillBeginEvent {
    /// Id of the frame that caused the download to begin.
    pub frame_id: FrameId,
    /// Global unique identifier of the download.
    pub guid: String,
    /// URL of the resource being downloaded.
    pub url: String,
    /// Suggested file name of the resource (the actual name of the file saved on disk may differ).
    pub suggested_filename: String,
}

/// Fired when download makes progress. Last call has `done` == true.
///
/// CDP: `Browser.downloadProgress`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgressEvent {
    /// Global unique identifier of the download.
    pub guid: String,
    /// Total expected bytes to download.
    pub total_bytes: f64,
    /// Total bytes received.
    pub received_bytes: f64,
    /// Download status.
    pub state: DownloadProgressState,
    /// If download is "completed", provides the path of the downloaded file.
    /// Depending on the platform, it is not guaranteed to be set, nor the file
    /// is guaranteed to exist.
    #[serde(default)]
    pub file_path: Option<String>,
}

// ── Domain trait ─────────────────────────────────────────────────────────────

/// `Browser` domain CDP methods.
///
/// The Browser domain defines methods and events for browser managing.
///
/// Reference: <https://chromedevtools.github.io/devtools-protocol/tot/Browser/>
pub trait BrowserCommands {
    /// Set permission settings for given embedding and embedded origins.
    ///
    /// CDP: `Browser.setPermission`
    async fn browser_set_permission(&self, params: &SetPermissionParams) -> Result<()>;

    /// Reset all permission management for all origins.
    ///
    /// CDP: `Browser.resetPermissions`
    async fn browser_reset_permissions(&self, params: &ResetPermissionsParams) -> Result<()>;

    /// Set the behavior when downloading a file.
    ///
    /// CDP: `Browser.setDownloadBehavior`
    async fn browser_set_download_behavior(&self, params: &SetDownloadBehaviorParams)
    -> Result<()>;

    /// Cancel a download if in progress.
    ///
    /// CDP: `Browser.cancelDownload`
    async fn browser_cancel_download(&self, params: &CancelDownloadParams) -> Result<()>;

    /// Close browser gracefully.
    ///
    /// CDP: `Browser.close`
    async fn browser_close(&self) -> Result<()>;

    /// Crashes browser on the main thread.
    ///
    /// CDP: `Browser.crash`
    async fn browser_crash(&self) -> Result<()>;

    /// Crashes GPU process.
    ///
    /// CDP: `Browser.crashGpuProcess`
    async fn browser_crash_gpu_process(&self) -> Result<()>;

    /// Returns version information.
    ///
    /// CDP: `Browser.getVersion`
    async fn browser_get_version(&self) -> Result<GetVersionReturn>;

    /// Returns the command line switches for the browser process if, and only if
    /// --enable-automation is on the commandline.
    ///
    /// CDP: `Browser.getBrowserCommandLine`
    async fn browser_get_browser_command_line(&self) -> Result<GetBrowserCommandLineReturn>;

    /// Get Chrome histograms.
    ///
    /// CDP: `Browser.getHistograms`
    async fn browser_get_histograms(
        &self,
        params: &GetHistogramsParams,
    ) -> Result<GetHistogramsReturn>;

    /// Get a Chrome histogram by name.
    ///
    /// CDP: `Browser.getHistogram`
    async fn browser_get_histogram(
        &self,
        params: &GetHistogramParams,
    ) -> Result<GetHistogramReturn>;

    /// Get position and size of the browser window.
    ///
    /// CDP: `Browser.getWindowBounds`
    async fn browser_get_window_bounds(&self, window_id: WindowId)
    -> Result<GetWindowBoundsReturn>;

    /// Get the browser window that contains the devtools target.
    ///
    /// CDP: `Browser.getWindowForTarget`
    async fn browser_get_window_for_target(
        &self,
        params: &GetWindowForTargetParams,
    ) -> Result<GetWindowForTargetReturn>;

    /// Set position and/or size of the browser window.
    ///
    /// CDP: `Browser.setWindowBounds`
    async fn browser_set_window_bounds(&self, params: &SetWindowBoundsParams) -> Result<()>;

    /// Set size of the browser contents resizing browser window as necessary.
    ///
    /// CDP: `Browser.setContentsSize`
    async fn browser_set_contents_size(&self, params: &SetContentsSizeParams) -> Result<()>;

    /// Set dock tile details, platform-specific.
    ///
    /// CDP: `Browser.setDockTile`
    async fn browser_set_dock_tile(&self, params: &SetDockTileParams) -> Result<()>;

    /// Invoke custom browser commands used by telemetry.
    ///
    /// CDP: `Browser.executeBrowserCommand`
    async fn browser_execute_browser_command(&self, command_id: BrowserCommandId) -> Result<()>;

    /// Allows a site to use privacy sandbox features that require enrollment
    /// without the site actually being enrolled. Only supported on page targets.
    ///
    /// CDP: `Browser.addPrivacySandboxEnrollmentOverride`
    async fn browser_add_privacy_sandbox_enrollment_override(&self, url: &str) -> Result<()>;

    /// Configures encryption keys used with a given privacy sandbox API to talk
    /// to a trusted coordinator. Since this is intended for test automation only,
    /// coordinatorOrigin must be a .test domain. No existing coordinator
    /// configuration for the origin may exist.
    ///
    /// CDP: `Browser.addPrivacySandboxCoordinatorKeyConfig`
    async fn browser_add_privacy_sandbox_coordinator_key_config(
        &self,
        params: &AddPrivacySandboxCoordinatorKeyConfigParams,
    ) -> Result<()>;
}

// ── Impl ────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GetWindowBoundsInternalParams {
    window_id: WindowId,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExecuteBrowserCommandInternalParams {
    command_id: BrowserCommandId,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AddPrivacySandboxEnrollmentOverrideInternalParams<'a> {
    url: &'a str,
}

impl BrowserCommands for CdpSession {
    async fn browser_set_permission(&self, params: &SetPermissionParams) -> Result<()> {
        self.call_no_response("Browser.setPermission", params).await
    }

    async fn browser_reset_permissions(&self, params: &ResetPermissionsParams) -> Result<()> {
        self.call_no_response("Browser.resetPermissions", params)
            .await
    }

    async fn browser_set_download_behavior(
        &self,
        params: &SetDownloadBehaviorParams,
    ) -> Result<()> {
        self.call_no_response("Browser.setDownloadBehavior", params)
            .await
    }

    async fn browser_cancel_download(&self, params: &CancelDownloadParams) -> Result<()> {
        self.call_no_response("Browser.cancelDownload", params)
            .await
    }

    async fn browser_close(&self) -> Result<()> {
        self.call_no_response("Browser.close", &serde_json::json!({}))
            .await
    }

    async fn browser_crash(&self) -> Result<()> {
        self.call_no_response("Browser.crash", &serde_json::json!({}))
            .await
    }

    async fn browser_crash_gpu_process(&self) -> Result<()> {
        self.call_no_response("Browser.crashGpuProcess", &serde_json::json!({}))
            .await
    }

    async fn browser_get_version(&self) -> Result<GetVersionReturn> {
        self.call("Browser.getVersion", &serde_json::json!({}))
            .await
    }

    async fn browser_get_browser_command_line(&self) -> Result<GetBrowserCommandLineReturn> {
        self.call("Browser.getBrowserCommandLine", &serde_json::json!({}))
            .await
    }

    async fn browser_get_histograms(
        &self,
        params: &GetHistogramsParams,
    ) -> Result<GetHistogramsReturn> {
        self.call("Browser.getHistograms", params).await
    }

    async fn browser_get_histogram(
        &self,
        params: &GetHistogramParams,
    ) -> Result<GetHistogramReturn> {
        self.call("Browser.getHistogram", params).await
    }

    async fn browser_get_window_bounds(
        &self,
        window_id: WindowId,
    ) -> Result<GetWindowBoundsReturn> {
        let params = GetWindowBoundsInternalParams { window_id };
        self.call("Browser.getWindowBounds", &params).await
    }

    async fn browser_get_window_for_target(
        &self,
        params: &GetWindowForTargetParams,
    ) -> Result<GetWindowForTargetReturn> {
        self.call("Browser.getWindowForTarget", params).await
    }

    async fn browser_set_window_bounds(&self, params: &SetWindowBoundsParams) -> Result<()> {
        self.call_no_response("Browser.setWindowBounds", params)
            .await
    }

    async fn browser_set_contents_size(&self, params: &SetContentsSizeParams) -> Result<()> {
        self.call_no_response("Browser.setContentsSize", params)
            .await
    }

    async fn browser_set_dock_tile(&self, params: &SetDockTileParams) -> Result<()> {
        self.call_no_response("Browser.setDockTile", params).await
    }

    async fn browser_execute_browser_command(&self, command_id: BrowserCommandId) -> Result<()> {
        let params = ExecuteBrowserCommandInternalParams { command_id };
        self.call_no_response("Browser.executeBrowserCommand", &params)
            .await
    }

    async fn browser_add_privacy_sandbox_enrollment_override(&self, url: &str) -> Result<()> {
        let params = AddPrivacySandboxEnrollmentOverrideInternalParams { url };
        self.call_no_response("Browser.addPrivacySandboxEnrollmentOverride", &params)
            .await
    }

    async fn browser_add_privacy_sandbox_coordinator_key_config(
        &self,
        params: &AddPrivacySandboxCoordinatorKeyConfigParams,
    ) -> Result<()> {
        self.call_no_response("Browser.addPrivacySandboxCoordinatorKeyConfig", params)
            .await
    }
}
