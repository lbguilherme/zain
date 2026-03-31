use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::session::CdpSession;
use crate::types::{Frame, FrameId, MonotonicTime, NavigationEntry};

// --- Param types ---

/// Parameters for [`PageCommands::page_navigate`].
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NavigateParams {
    /// Target URL for navigation.
    pub url: String,
    /// Referrer URL. If omitted, the browser uses the default referrer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub referrer: Option<String>,
    /// Intended transition type (e.g. `"link"`, `"typed"`, `"reload"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition_type: Option<String>,
    /// Frame to navigate. If omitted, navigates the top-level frame.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame_id: Option<FrameId>,
}

/// Parameters for [`PageCommands::page_reload`].
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReloadParams {
    /// If `true`, bypasses the browser cache (equivalent to Ctrl+Shift+R).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore_cache: Option<bool>,
    /// Script to inject into all frames after reload.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script_to_evaluate_on_load: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NavigateToHistoryEntryParams {
    pub entry_id: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SetLifecycleEventsEnabledParams {
    pub enabled: bool,
}

/// Viewport clip region for screenshots.
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Viewport {
    /// X offset in CSS pixels.
    pub x: f64,
    /// Y offset in CSS pixels.
    pub y: f64,
    /// Width in CSS pixels.
    pub width: f64,
    /// Height in CSS pixels.
    pub height: f64,
    /// Page scale factor (1.0 = no scaling).
    pub scale: f64,
}

/// Parameters for [`PageCommands::page_capture_screenshot`].
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureScreenshotParams {
    /// Image format: `"jpeg"`, `"png"`, `"webp"`. Default: `"png"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    /// JPEG/WebP compression quality (0-100). Not applicable to PNG.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<i32>,
    /// Clip region of the page to capture. If omitted, captures the full visible page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clip: Option<Viewport>,
}

/// Return type for [`PageCommands::page_capture_screenshot`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureScreenshotReturn {
    /// Base64-encoded image data.
    pub data: String,
}

// --- Return types ---

/// Return type for [`PageCommands::page_navigate`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NavigateReturn {
    /// ID of the frame that navigated (or failed to navigate).
    pub frame_id: FrameId,
    /// Loader identifier. Omitted for same-document navigations.
    #[serde(default)]
    pub loader_id: Option<String>,
    /// User-friendly error message if navigation failed (e.g. DNS resolution failure).
    #[serde(default)]
    pub error_text: Option<String>,
}

/// Return type for [`PageCommands::page_get_navigation_history`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetNavigationHistoryReturn {
    /// Index of the active navigation history entry.
    pub current_index: i64,
    /// Navigation history records.
    pub entries: Vec<NavigationEntry>,
}

// --- Events ---

/// Fired when the page's `load` event fires.
///
/// CDP: `Page.loadEventFired`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadEventFiredEvent {
    pub timestamp: MonotonicTime,
}

/// Fired when `DOMContentLoaded` event fires.
///
/// CDP: `Page.domContentEventFired`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DomContentEventFiredEvent {
    pub timestamp: MonotonicTime,
}

/// Fired once a frame navigation completes.
///
/// CDP: `Page.frameNavigated`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameNavigatedEvent {
    pub frame: Frame,
}

/// Fired for lifecycle milestones (navigation, load, paint, etc.).
///
/// CDP: `Page.lifecycleEvent`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LifecycleEventEvent {
    /// Frame that emitted the event.
    pub frame_id: FrameId,
    /// Lifecycle event name (e.g. `"load"`, `"DOMContentLoaded"`, `"networkIdle"`).
    pub name: String,
    pub timestamp: MonotonicTime,
}

// --- Domain trait ---

/// `Page` domain CDP methods.
///
/// Reference: <https://chromedevtools.github.io/devtools-protocol/tot/Page/>
pub trait PageCommands {
    /// Enables page domain notifications. The browser starts emitting events such as
    /// `loadEventFired`, `domContentEventFired` and `frameNavigated`.
    ///
    /// CDP: `Page.enable`
    async fn page_enable(&self) -> Result<()>;

    /// Disables page domain notifications.
    ///
    /// CDP: `Page.disable`
    async fn page_disable(&self) -> Result<()>;

    /// Navigates the current page to the specified URL.
    ///
    /// CDP: `Page.navigate`
    async fn page_navigate(&self, params: &NavigateParams) -> Result<NavigateReturn>;

    /// Reloads the current page, optionally bypassing cache.
    ///
    /// CDP: `Page.reload`
    async fn page_reload(&self, params: &ReloadParams) -> Result<()>;

    /// Returns the navigation history for the current page.
    ///
    /// CDP: `Page.getNavigationHistory`
    async fn page_get_navigation_history(&self) -> Result<GetNavigationHistoryReturn>;

    /// Navigates to a specific entry in the navigation history.
    ///
    /// - `entry_id`: unique ID of the target history entry, obtained from
    ///   [`page_get_navigation_history`](Self::page_get_navigation_history).
    ///
    /// CDP: `Page.navigateToHistoryEntry`
    async fn page_navigate_to_history_entry(&self, entry_id: i64) -> Result<()>;

    /// Controls whether the page emits lifecycle events (`load`, `DOMContentLoaded`,
    /// `networkIdle`, etc.).
    ///
    /// - `enabled`: `true` to activate, `false` to deactivate.
    ///
    /// CDP: `Page.setLifecycleEventsEnabled`
    async fn page_set_lifecycle_events_enabled(&self, enabled: bool) -> Result<()>;

    /// Captures a screenshot of the page or a specific clip region.
    ///
    /// - `params`: screenshot options (format, clip region, etc.).
    ///
    /// Returns base64-encoded image data.
    ///
    /// CDP: `Page.captureScreenshot`
    async fn page_capture_screenshot(
        &self,
        params: &CaptureScreenshotParams,
    ) -> Result<CaptureScreenshotReturn>;
}

impl PageCommands for CdpSession {
    async fn page_enable(&self) -> Result<()> {
        self.call_no_response("Page.enable", &serde_json::json!({}))
            .await
    }

    async fn page_disable(&self) -> Result<()> {
        self.call_no_response("Page.disable", &serde_json::json!({}))
            .await
    }

    async fn page_navigate(&self, params: &NavigateParams) -> Result<NavigateReturn> {
        self.call("Page.navigate", params).await
    }

    async fn page_reload(&self, params: &ReloadParams) -> Result<()> {
        self.call_no_response("Page.reload", params).await
    }

    async fn page_get_navigation_history(&self) -> Result<GetNavigationHistoryReturn> {
        self.call("Page.getNavigationHistory", &serde_json::json!({}))
            .await
    }

    async fn page_navigate_to_history_entry(&self, entry_id: i64) -> Result<()> {
        let params = NavigateToHistoryEntryParams { entry_id };
        self.call_no_response("Page.navigateToHistoryEntry", &params)
            .await
    }

    async fn page_set_lifecycle_events_enabled(&self, enabled: bool) -> Result<()> {
        let params = SetLifecycleEventsEnabledParams { enabled };
        self.call_no_response("Page.setLifecycleEventsEnabled", &params)
            .await
    }

    async fn page_capture_screenshot(
        &self,
        params: &CaptureScreenshotParams,
    ) -> Result<CaptureScreenshotReturn> {
        self.call("Page.captureScreenshot", params).await
    }
}
