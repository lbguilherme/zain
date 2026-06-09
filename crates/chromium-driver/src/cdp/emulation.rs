use serde::{Deserialize, Serialize};

use crate::cdp::common::TimeSinceEpoch;
use crate::cdp::dom::RGBA;
use crate::cdp::page::Viewport;
use crate::error::Result;
use crate::session::CdpSession;

// ── Types ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SafeAreaInsets {
    /// Overrides safe-area-inset-top.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top: Option<i64>,
    /// Overrides safe-area-max-inset-top.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_max: Option<i64>,
    /// Overrides safe-area-inset-left.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub left: Option<i64>,
    /// Overrides safe-area-max-inset-left.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub left_max: Option<i64>,
    /// Overrides safe-area-inset-bottom.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bottom: Option<i64>,
    /// Overrides safe-area-max-inset-bottom.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bottom_max: Option<i64>,
    /// Overrides safe-area-inset-right.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub right: Option<i64>,
    /// Overrides safe-area-max-inset-right.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub right_max: Option<i64>,
}

/// Screen orientation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenOrientation {
    /// Orientation type.
    pub r#type: ScreenOrientationType,
    /// Orientation angle.
    pub angle: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayFeature {
    /// Orientation of a display feature in relation to screen.
    pub orientation: DisplayFeatureOrientation,
    /// The offset from the screen origin in either the x (for vertical
    /// orientation) or y (for horizontal orientation) direction.
    pub offset: i64,
    /// A display feature may mask content such that it is not physically
    /// displayed - this length along with the offset describes this area.
    /// A display feature that only splits content will have a 0 mask_length.
    pub mask_length: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DevicePosture {
    /// Current posture of the device.
    pub r#type: DevicePostureType,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaFeature {
    pub name: String,
    pub value: String,
}

/// advance: If the scheduler runs out of immediate work, the virtual time base may fast forward to
/// allow the next delayed task (if any) to run; pause: The virtual time base may not advance;
/// pauseIfNetworkFetchesPending: The virtual time base may not advance if there are any pending
/// resource fetches.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum VirtualTimePolicy {
    #[default]
    #[serde(rename = "advance")]
    Advance,
    #[serde(rename = "pause")]
    Pause,
    #[serde(rename = "pauseIfNetworkFetchesPending")]
    PauseIfNetworkFetchesPending,
}

/// Used to specify User Agent Client Hints to emulate. See https://wicg.github.io/ua-client-hints.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserAgentBrandVersion {
    pub brand: String,
    pub version: String,
}

/// Used to specify User Agent Client Hints to emulate. See https://wicg.github.io/ua-client-hints
/// Missing optional values will be filled in by the target with what it would normally use.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserAgentMetadata {
    /// Brands appearing in Sec-CH-UA.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub brands: Option<Vec<UserAgentBrandVersion>>,
    /// Brands appearing in Sec-CH-UA-Full-Version-List.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub full_version_list: Option<Vec<UserAgentBrandVersion>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub full_version: Option<String>,
    pub platform: String,
    pub platform_version: String,
    pub architecture: String,
    pub model: String,
    pub mobile: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bitness: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wow64: Option<bool>,
    /// Used to specify User Agent form-factor values.
    /// See https://wicg.github.io/ua-client-hints/#sec-ch-ua-form-factors.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub form_factors: Option<Vec<String>>,
}

/// Used to specify sensor types to emulate.
/// See https://w3c.github.io/sensors/#automation for more information.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SensorType {
    #[default]
    #[serde(rename = "absolute-orientation")]
    AbsoluteOrientation,
    #[serde(rename = "accelerometer")]
    Accelerometer,
    #[serde(rename = "ambient-light")]
    AmbientLight,
    #[serde(rename = "gravity")]
    Gravity,
    #[serde(rename = "gyroscope")]
    Gyroscope,
    #[serde(rename = "linear-acceleration")]
    LinearAcceleration,
    #[serde(rename = "magnetometer")]
    Magnetometer,
    #[serde(rename = "relative-orientation")]
    RelativeOrientation,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SensorMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub available: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum_frequency: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximum_frequency: Option<f64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SensorReadingSingle {
    pub value: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SensorReadingXYZ {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SensorReadingQuaternion {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SensorReading {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub single: Option<SensorReadingSingle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub xyz: Option<SensorReadingXYZ>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quaternion: Option<SensorReadingQuaternion>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PressureSource {
    #[default]
    #[serde(rename = "cpu")]
    Cpu,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PressureState {
    #[default]
    #[serde(rename = "nominal")]
    Nominal,
    #[serde(rename = "fair")]
    Fair,
    #[serde(rename = "serious")]
    Serious,
    #[serde(rename = "critical")]
    Critical,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PressureMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub available: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkAreaInsets {
    /// Work area top inset in pixels. Default is 0;.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top: Option<i64>,
    /// Work area left inset in pixels. Default is 0;.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub left: Option<i64>,
    /// Work area bottom inset in pixels. Default is 0;.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bottom: Option<i64>,
    /// Work area right inset in pixels. Default is 0;.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub right: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ScreenId(pub String);

/// Screen information similar to the one returned by window.getScreenDetails() method,
/// see https://w3c.github.io/window-management/#screendetailed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenInfo {
    /// Offset of the left edge of the screen.
    pub left: i64,
    /// Offset of the top edge of the screen.
    pub top: i64,
    /// Width of the screen.
    pub width: i64,
    /// Height of the screen.
    pub height: i64,
    /// Offset of the left edge of the available screen area.
    pub avail_left: i64,
    /// Offset of the top edge of the available screen area.
    pub avail_top: i64,
    /// Width of the available screen area.
    pub avail_width: i64,
    /// Height of the available screen area.
    pub avail_height: i64,
    /// Specifies the screen's device pixel ratio.
    pub device_pixel_ratio: f64,
    /// Specifies the screen's orientation.
    pub orientation: ScreenOrientation,
    /// Specifies the screen's color depth in bits.
    pub color_depth: i64,
    /// Indicates whether the device has multiple screens.
    pub is_extended: bool,
    /// Indicates whether the screen is internal to the device or external, attached to the device.
    pub is_internal: bool,
    /// Indicates whether the screen is set as the the operating system primary screen.
    pub is_primary: bool,
    /// Specifies the descriptive label for the screen.
    pub label: String,
    /// Specifies the unique identifier of the screen.
    pub id: ScreenId,
}

/// Enum of image types that can be disabled.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisabledImageType {
    #[default]
    #[serde(rename = "avif")]
    Avif,
    #[serde(rename = "jxl")]
    Jxl,
    #[serde(rename = "webp")]
    Webp,
}

// ── Inline enums ─────────────────────────────────────────────────────────────

/// Orientation type.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScreenOrientationType {
    #[default]
    #[serde(rename = "portraitPrimary")]
    PortraitPrimary,
    #[serde(rename = "portraitSecondary")]
    PortraitSecondary,
    #[serde(rename = "landscapePrimary")]
    LandscapePrimary,
    #[serde(rename = "landscapeSecondary")]
    LandscapeSecondary,
}

/// Orientation of a display feature in relation to screen.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisplayFeatureOrientation {
    #[default]
    #[serde(rename = "vertical")]
    Vertical,
    #[serde(rename = "horizontal")]
    Horizontal,
}

/// Current posture of the device.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DevicePostureType {
    #[default]
    #[serde(rename = "continuous")]
    Continuous,
    #[serde(rename = "folded")]
    Folded,
}

/// Scrollbar type. Default: `default`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SetDeviceMetricsOverrideScrollbarType {
    #[default]
    #[serde(rename = "overlay")]
    Overlay,
    #[serde(rename = "default")]
    Default,
}

/// Touch/gesture events configuration. Default: current platform.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SetEmitTouchEventsForMouseConfiguration {
    #[default]
    #[serde(rename = "mobile")]
    Mobile,
    #[serde(rename = "desktop")]
    Desktop,
}

/// Vision deficiency to emulate. Order: best-effort emulations come first, followed by any
/// physiologically accurate emulations for medically recognized color vision deficiencies.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SetEmulatedVisionDeficiencyType {
    #[default]
    #[serde(rename = "none")]
    None,
    #[serde(rename = "blurredVision")]
    BlurredVision,
    #[serde(rename = "reducedContrast")]
    ReducedContrast,
    #[serde(rename = "achromatopsia")]
    Achromatopsia,
    #[serde(rename = "deuteranopia")]
    Deuteranopia,
    #[serde(rename = "protanopia")]
    Protanopia,
    #[serde(rename = "tritanopia")]
    Tritanopia,
}

// ── Param types ──────────────────────────────────────────────────────────────

/// Parameters for [`EmulationCommands::emulation_set_auto_dark_mode_override`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetAutoDarkModeOverrideParams {
    /// Whether to enable or disable automatic dark mode.
    /// If not specified, any existing override will be cleared.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

/// Parameters for [`EmulationCommands::emulation_set_default_background_color_override`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetDefaultBackgroundColorOverrideParams {
    /// RGBA of the default background color. If not specified, any existing override will be
    /// cleared.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<RGBA>,
}

/// Parameters for [`EmulationCommands::emulation_set_device_metrics_override`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetDeviceMetricsOverrideParams {
    /// Overriding width value in pixels (minimum 0, maximum 10000000). 0 disables the override.
    pub width: i64,
    /// Overriding height value in pixels (minimum 0, maximum 10000000). 0 disables the override.
    pub height: i64,
    /// Overriding device scale factor value. 0 disables the override.
    pub device_scale_factor: f64,
    /// Whether to emulate mobile device. This includes viewport meta tag, overlay scrollbars, text
    /// autosizing and more.
    pub mobile: bool,
    /// Scale to apply to resulting view image.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<f64>,
    /// Overriding screen width value in pixels (minimum 0, maximum 10000000).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screen_width: Option<i64>,
    /// Overriding screen height value in pixels (minimum 0, maximum 10000000).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screen_height: Option<i64>,
    /// Overriding view X position on screen in pixels (minimum 0, maximum 10000000).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_x: Option<i64>,
    /// Overriding view Y position on screen in pixels (minimum 0, maximum 10000000).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_y: Option<i64>,
    /// Do not set visible view size, rely upon explicit setVisibleSize call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dont_set_visible_size: Option<bool>,
    /// Screen orientation override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screen_orientation: Option<ScreenOrientation>,
    /// If set, the visible area of the page will be overridden to this viewport. This viewport
    /// change is not observed by the page, e.g. viewport-relative elements do not change positions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub viewport: Option<Viewport>,
    /// If set, the display feature of a multi-segment screen. If not set, multi-segment support
    /// is turned-off.
    /// Deprecated, use Emulation.setDisplayFeaturesOverride.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_feature: Option<DisplayFeature>,
    /// If set, the posture of a foldable device. If not set the posture is set
    /// to continuous.
    /// Deprecated, use Emulation.setDevicePostureOverride.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_posture: Option<DevicePosture>,
    /// Scrollbar type. Default: `default`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scrollbar_type: Option<SetDeviceMetricsOverrideScrollbarType>,
    /// If set to true, enables screen orientation lock emulation, which
    /// intercepts screen.orientation.lock() calls from the page and reports
    /// orientation changes via screenOrientationLockChanged events. This is
    /// useful for emulating mobile device orientation lock behavior in
    /// responsive design mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screen_orientation_lock_emulation: Option<bool>,
}

/// Parameters for [`EmulationCommands::emulation_set_emit_touch_events_for_mouse`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetEmitTouchEventsForMouseParams {
    /// Whether touch emulation based on mouse input should be enabled.
    pub enabled: bool,
    /// Touch/gesture events configuration. Default: current platform.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub configuration: Option<SetEmitTouchEventsForMouseConfiguration>,
}

/// Parameters for [`EmulationCommands::emulation_set_emulated_media`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetEmulatedMediaParams {
    /// Media type to emulate. Empty string disables the override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media: Option<String>,
    /// Media features to emulate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub features: Option<Vec<MediaFeature>>,
}

/// Parameters for [`EmulationCommands::emulation_set_emulated_vision_deficiency`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetEmulatedVisionDeficiencyParams {
    /// Vision deficiency to emulate. Order: best-effort emulations come first, followed by any
    /// physiologically accurate emulations for medically recognized color vision deficiencies.
    pub r#type: SetEmulatedVisionDeficiencyType,
}

/// Parameters for [`EmulationCommands::emulation_set_emulated_os_text_scale`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetEmulatedOSTextScaleParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<f64>,
}

/// Parameters for [`EmulationCommands::emulation_set_geolocation_override`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetGeolocationOverrideParams {
    /// Mock latitude.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latitude: Option<f64>,
    /// Mock longitude.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub longitude: Option<f64>,
    /// Mock accuracy.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accuracy: Option<f64>,
    /// Mock altitude.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub altitude: Option<f64>,
    /// Mock altitudeAccuracy.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub altitude_accuracy: Option<f64>,
    /// Mock heading.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heading: Option<f64>,
    /// Mock speed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<f64>,
}

/// Parameters for [`EmulationCommands::emulation_set_sensor_override_enabled`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetSensorOverrideEnabledParams {
    pub enabled: bool,
    pub r#type: SensorType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<SensorMetadata>,
}

/// Parameters for [`EmulationCommands::emulation_set_pressure_source_override_enabled`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetPressureSourceOverrideEnabledParams {
    pub enabled: bool,
    pub source: PressureSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<PressureMetadata>,
}

/// Parameters for [`EmulationCommands::emulation_set_pressure_data_override`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetPressureDataOverrideParams {
    pub source: PressureSource,
    pub state: PressureState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub own_contribution_estimate: Option<f64>,
}

/// Parameters for [`EmulationCommands::emulation_set_touch_emulation_enabled`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetTouchEmulationEnabledParams {
    /// Whether the touch event emulation should be enabled.
    pub enabled: bool,
    /// Maximum touch points supported. Defaults to one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_touch_points: Option<i64>,
}

/// Parameters for [`EmulationCommands::emulation_set_virtual_time_policy`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetVirtualTimePolicyParams {
    pub policy: VirtualTimePolicy,
    /// If set, after this many virtual milliseconds have elapsed virtual time will be paused and a
    /// virtualTimeBudgetExpired event is sent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget: Option<f64>,
    /// If set this specifies the maximum number of tasks that can be run before virtual is forced
    /// forwards to prevent deadlock.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_virtual_time_task_starvation_count: Option<i64>,
    /// If set, base::Time::Now will be overridden to initially return this value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_virtual_time: Option<TimeSinceEpoch>,
}

/// Parameters for [`EmulationCommands::emulation_set_locale_override`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetLocaleOverrideParams {
    /// ICU style C locale (e.g. "en_US"). If not specified or empty, disables the override and
    /// restores default host system locale.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
}

/// Parameters for [`EmulationCommands::emulation_set_data_saver_override`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetDataSaverOverrideParams {
    /// Override value. Omitting the parameter disables the override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_saver_enabled: Option<bool>,
}

/// Parameters for [`EmulationCommands::emulation_set_user_agent_override`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetUserAgentOverrideParams {
    /// User agent to use.
    pub user_agent: String,
    /// Browser language to emulate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accept_language: Option<String>,
    /// The platform navigator.platform should return.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
    /// To be sent in Sec-CH-UA-* headers and returned in navigator.userAgentData.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent_metadata: Option<UserAgentMetadata>,
}

/// Parameters for [`EmulationCommands::emulation_add_screen`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddScreenParams {
    /// Offset of the left edge of the screen in pixels.
    pub left: i64,
    /// Offset of the top edge of the screen in pixels.
    pub top: i64,
    /// The width of the screen in pixels.
    pub width: i64,
    /// The height of the screen in pixels.
    pub height: i64,
    /// Specifies the screen's work area. Default is entire screen.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_area_insets: Option<WorkAreaInsets>,
    /// Specifies the screen's device pixel ratio. Default is 1.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_pixel_ratio: Option<f64>,
    /// Specifies the screen's rotation angle. Available values are 0, 90, 180 and 270. Default is 0.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rotation: Option<i64>,
    /// Specifies the screen's color depth in bits. Default is 24.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_depth: Option<i64>,
    /// Specifies the descriptive label for the screen. Default is none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Indicates whether the screen is internal to the device or external, attached to the device. Default is false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_internal: Option<bool>,
}

/// Parameters for [`EmulationCommands::emulation_update_screen`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateScreenParams {
    /// Target screen identifier.
    pub screen_id: ScreenId,
    /// Offset of the left edge of the screen in pixels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left: Option<i64>,
    /// Offset of the top edge of the screen in pixels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top: Option<i64>,
    /// The width of the screen in pixels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<i64>,
    /// The height of the screen in pixels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<i64>,
    /// Specifies the screen's work area.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_area_insets: Option<WorkAreaInsets>,
    /// Specifies the screen's device pixel ratio.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_pixel_ratio: Option<f64>,
    /// Specifies the screen's rotation angle. Available values are 0, 90, 180 and 270.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rotation: Option<i64>,
    /// Specifies the screen's color depth in bits.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_depth: Option<i64>,
    /// Specifies the descriptive label for the screen.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Indicates whether the screen is internal to the device or external, attached to the device. Default is false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_internal: Option<bool>,
}

// ── Return types ─────────────────────────────────────────────────────────────

/// Return type for [`EmulationCommands::emulation_get_overridden_sensor_information`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetOverriddenSensorInformationReturn {
    pub requested_sampling_frequency: f64,
}

/// Return type for [`EmulationCommands::emulation_set_virtual_time_policy`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetVirtualTimePolicyReturn {
    /// Absolute timestamp at which virtual time was first enabled (up time in milliseconds).
    pub virtual_time_ticks_base: f64,
}

/// Return type for [`EmulationCommands::emulation_get_screen_infos`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetScreenInfosReturn {
    pub screen_infos: Vec<ScreenInfo>,
}

/// Return type for [`EmulationCommands::emulation_add_screen`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddScreenReturn {
    pub screen_info: ScreenInfo,
}

/// Return type for [`EmulationCommands::emulation_update_screen`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateScreenReturn {
    pub screen_info: ScreenInfo,
}

// ── Events ───────────────────────────────────────────────────────────────────

/// Notification sent after the virtual time budget for the current VirtualTimePolicy has run out.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VirtualTimeBudgetExpiredEvent {
}

/// Fired when a page calls screen.orientation.lock() or screen.orientation.unlock()
/// while device emulation is enabled. This allows the DevTools frontend to update the
/// emulated device orientation accordingly.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenOrientationLockChangedEvent {
    /// Whether the screen orientation is currently locked.
    pub locked: bool,
    /// The orientation lock type requested by the page. Only set when locked is true.
    #[serde(default)]
    pub orientation: Option<ScreenOrientation>,
}

// ── Domain trait ─────────────────────────────────────────────────────────────

/// `Emulation` domain CDP methods.
///
/// This domain emulates different environments for the page.
///
/// Reference: <https://chromedevtools.github.io/devtools-protocol/tot/Emulation/>
pub trait EmulationCommands {
    /// Clears the overridden device metrics.
    ///
    /// CDP: `Emulation.clearDeviceMetricsOverride`
    async fn emulation_clear_device_metrics_override(&self) -> Result<()>;

    /// Clears the overridden Geolocation Position and Error.
    ///
    /// CDP: `Emulation.clearGeolocationOverride`
    async fn emulation_clear_geolocation_override(&self) -> Result<()>;

    /// Requests that page scale factor is reset to initial values.
    ///
    /// CDP: `Emulation.resetPageScaleFactor`
    async fn emulation_reset_page_scale_factor(&self) -> Result<()>;

    /// Enables or disables simulating a focused and active page.
    ///
    /// CDP: `Emulation.setFocusEmulationEnabled`
    async fn emulation_set_focus_emulation_enabled(&self, enabled: bool) -> Result<()>;

    /// Automatically render all web contents using a dark theme.
    ///
    /// CDP: `Emulation.setAutoDarkModeOverride`
    async fn emulation_set_auto_dark_mode_override(&self, params: &SetAutoDarkModeOverrideParams) -> Result<()>;

    /// Enables CPU throttling to emulate slow CPUs.
    ///
    /// CDP: `Emulation.setCPUThrottlingRate`
    async fn emulation_set_cpu_throttling_rate(&self, rate: f64) -> Result<()>;

    /// Sets or clears an override of the default background color of the frame. This override is used
    /// if the content does not specify one.
    ///
    /// CDP: `Emulation.setDefaultBackgroundColorOverride`
    async fn emulation_set_default_background_color_override(&self, params: &SetDefaultBackgroundColorOverrideParams) -> Result<()>;

    /// Overrides the values for env(safe-area-inset-*) and env(safe-area-max-inset-*). Unset values will cause the
    /// respective variables to be undefined, even if previously overridden.
    ///
    /// CDP: `Emulation.setSafeAreaInsetsOverride`
    async fn emulation_set_safe_area_insets_override(&self, insets: &SafeAreaInsets) -> Result<()>;

    /// Overrides the values of device screen dimensions (window.screen.width, window.screen.height,
    /// window.innerWidth, window.innerHeight, and "device-width"/"device-height"-related CSS media
    /// query results).
    ///
    /// CDP: `Emulation.setDeviceMetricsOverride`
    async fn emulation_set_device_metrics_override(&self, params: &SetDeviceMetricsOverrideParams) -> Result<()>;

    /// Start reporting the given posture value to the Device Posture API.
    /// This override can also be set in setDeviceMetricsOverride().
    ///
    /// CDP: `Emulation.setDevicePostureOverride`
    async fn emulation_set_device_posture_override(&self, posture: &DevicePosture) -> Result<()>;

    /// Clears a device posture override set with either setDeviceMetricsOverride()
    /// or setDevicePostureOverride() and starts using posture information from the
    /// platform again.
    /// Does nothing if no override is set.
    ///
    /// CDP: `Emulation.clearDevicePostureOverride`
    async fn emulation_clear_device_posture_override(&self) -> Result<()>;

    /// Start using the given display features to pupulate the Viewport Segments API.
    /// This override can also be set in setDeviceMetricsOverride().
    ///
    /// CDP: `Emulation.setDisplayFeaturesOverride`
    async fn emulation_set_display_features_override(&self, features: &[DisplayFeature]) -> Result<()>;

    /// Clears the display features override set with either setDeviceMetricsOverride()
    /// or setDisplayFeaturesOverride() and starts using display features from the
    /// platform again.
    /// Does nothing if no override is set.
    ///
    /// CDP: `Emulation.clearDisplayFeaturesOverride`
    async fn emulation_clear_display_features_override(&self) -> Result<()>;

    ///
    /// CDP: `Emulation.setScrollbarsHidden`
    async fn emulation_set_scrollbars_hidden(&self, hidden: bool) -> Result<()>;

    ///
    /// CDP: `Emulation.setDocumentCookieDisabled`
    async fn emulation_set_document_cookie_disabled(&self, disabled: bool) -> Result<()>;

    ///
    /// CDP: `Emulation.setEmitTouchEventsForMouse`
    async fn emulation_set_emit_touch_events_for_mouse(&self, params: &SetEmitTouchEventsForMouseParams) -> Result<()>;

    /// Emulates the given media type or media feature for CSS media queries.
    ///
    /// CDP: `Emulation.setEmulatedMedia`
    async fn emulation_set_emulated_media(&self, params: &SetEmulatedMediaParams) -> Result<()>;

    /// Emulates the given vision deficiency.
    ///
    /// CDP: `Emulation.setEmulatedVisionDeficiency`
    async fn emulation_set_emulated_vision_deficiency(&self, params: &SetEmulatedVisionDeficiencyParams) -> Result<()>;

    /// Emulates the given OS text scale.
    ///
    /// CDP: `Emulation.setEmulatedOSTextScale`
    async fn emulation_set_emulated_os_text_scale(&self, params: &SetEmulatedOSTextScaleParams) -> Result<()>;

    /// Overrides the Geolocation Position or Error. Omitting latitude, longitude or
    /// accuracy emulates position unavailable.
    ///
    /// CDP: `Emulation.setGeolocationOverride`
    async fn emulation_set_geolocation_override(&self, params: &SetGeolocationOverrideParams) -> Result<()>;

    ///
    /// CDP: `Emulation.getOverriddenSensorInformation`
    async fn emulation_get_overridden_sensor_information(&self, r#type: &SensorType) -> Result<GetOverriddenSensorInformationReturn>;

    /// Overrides a platform sensor of a given type. If |enabled| is true, calls to
    /// Sensor.start() will use a virtual sensor as backend rather than fetching
    /// data from a real hardware sensor. Otherwise, existing virtual
    /// sensor-backend Sensor objects will fire an error event and new calls to
    /// Sensor.start() will attempt to use a real sensor instead.
    ///
    /// CDP: `Emulation.setSensorOverrideEnabled`
    async fn emulation_set_sensor_override_enabled(&self, params: &SetSensorOverrideEnabledParams) -> Result<()>;

    /// Updates the sensor readings reported by a sensor type previously overridden
    /// by setSensorOverrideEnabled.
    ///
    /// CDP: `Emulation.setSensorOverrideReadings`
    async fn emulation_set_sensor_override_readings(&self, r#type: &SensorType, reading: &SensorReading) -> Result<()>;

    /// Overrides a pressure source of a given type, as used by the Compute
    /// Pressure API, so that updates to PressureObserver.observe() are provided
    /// via setPressureStateOverride instead of being retrieved from
    /// platform-provided telemetry data.
    ///
    /// CDP: `Emulation.setPressureSourceOverrideEnabled`
    async fn emulation_set_pressure_source_override_enabled(&self, params: &SetPressureSourceOverrideEnabledParams) -> Result<()>;

    /// TODO: OBSOLETE: To remove when setPressureDataOverride is merged.
    /// Provides a given pressure state that will be processed and eventually be
    /// delivered to PressureObserver users. |source| must have been previously
    /// overridden by setPressureSourceOverrideEnabled.
    ///
    /// CDP: `Emulation.setPressureStateOverride`
    async fn emulation_set_pressure_state_override(&self, source: &PressureSource, state: &PressureState) -> Result<()>;

    /// Provides a given pressure data set that will be processed and eventually be
    /// delivered to PressureObserver users. |source| must have been previously
    /// overridden by setPressureSourceOverrideEnabled.
    ///
    /// CDP: `Emulation.setPressureDataOverride`
    async fn emulation_set_pressure_data_override(&self, params: &SetPressureDataOverrideParams) -> Result<()>;

    /// Overrides the Idle state.
    ///
    /// CDP: `Emulation.setIdleOverride`
    async fn emulation_set_idle_override(&self, is_user_active: bool, is_screen_unlocked: bool) -> Result<()>;

    /// Clears Idle state overrides.
    ///
    /// CDP: `Emulation.clearIdleOverride`
    async fn emulation_clear_idle_override(&self) -> Result<()>;

    /// Sets a specified page scale factor.
    ///
    /// CDP: `Emulation.setPageScaleFactor`
    async fn emulation_set_page_scale_factor(&self, page_scale_factor: f64) -> Result<()>;

    /// Switches script execution in the page.
    ///
    /// CDP: `Emulation.setScriptExecutionDisabled`
    async fn emulation_set_script_execution_disabled(&self, value: bool) -> Result<()>;

    /// Enables touch on platforms which do not support them.
    ///
    /// CDP: `Emulation.setTouchEmulationEnabled`
    async fn emulation_set_touch_emulation_enabled(&self, params: &SetTouchEmulationEnabledParams) -> Result<()>;

    /// Turns on virtual time for all frames (replacing real-time with a synthetic time source) and sets
    /// the current virtual time policy.  Note this supersedes any previous time budget.
    ///
    /// CDP: `Emulation.setVirtualTimePolicy`
    async fn emulation_set_virtual_time_policy(&self, params: &SetVirtualTimePolicyParams) -> Result<SetVirtualTimePolicyReturn>;

    /// Overrides default host system locale with the specified one.
    ///
    /// CDP: `Emulation.setLocaleOverride`
    async fn emulation_set_locale_override(&self, params: &SetLocaleOverrideParams) -> Result<()>;

    /// Overrides default host system timezone with the specified one.
    ///
    /// CDP: `Emulation.setTimezoneOverride`
    async fn emulation_set_timezone_override(&self, timezone_id: &str) -> Result<()>;

    ///
    /// CDP: `Emulation.setDisabledImageTypes`
    async fn emulation_set_disabled_image_types(&self, image_types: &[DisabledImageType]) -> Result<()>;

    /// Override the value of navigator.connection.saveData.
    ///
    /// CDP: `Emulation.setDataSaverOverride`
    async fn emulation_set_data_saver_override(&self, params: &SetDataSaverOverrideParams) -> Result<()>;

    ///
    /// CDP: `Emulation.setHardwareConcurrencyOverride`
    async fn emulation_set_hardware_concurrency_override(&self, hardware_concurrency: i64) -> Result<()>;

    /// Allows overriding user agent with the given string.
    /// `userAgentMetadata` must be set for Client Hint headers to be sent.
    ///
    /// CDP: `Emulation.setUserAgentOverride`
    async fn emulation_set_user_agent_override(&self, params: &SetUserAgentOverrideParams) -> Result<()>;

    /// Allows overriding the automation flag.
    ///
    /// CDP: `Emulation.setAutomationOverride`
    async fn emulation_set_automation_override(&self, enabled: bool) -> Result<()>;

    /// Allows overriding the difference between the small and large viewport sizes, which determine the
    /// value of the `svh` and `lvh` unit, respectively. Only supported for top-level frames.
    ///
    /// CDP: `Emulation.setSmallViewportHeightDifferenceOverride`
    async fn emulation_set_small_viewport_height_difference_override(&self, difference: i64) -> Result<()>;

    /// Returns device's screen configuration. In headful mode, the physical screens configuration is returned,
    /// whereas in headless mode, a virtual headless screen configuration is provided instead.
    ///
    /// CDP: `Emulation.getScreenInfos`
    async fn emulation_get_screen_infos(&self) -> Result<GetScreenInfosReturn>;

    /// Add a new screen to the device. Only supported in headless mode.
    ///
    /// CDP: `Emulation.addScreen`
    async fn emulation_add_screen(&self, params: &AddScreenParams) -> Result<AddScreenReturn>;

    /// Updates specified screen parameters. Only supported in headless mode.
    ///
    /// CDP: `Emulation.updateScreen`
    async fn emulation_update_screen(&self, params: &UpdateScreenParams) -> Result<UpdateScreenReturn>;

    /// Remove screen from the device. Only supported in headless mode.
    ///
    /// CDP: `Emulation.removeScreen`
    async fn emulation_remove_screen(&self, screen_id: &ScreenId) -> Result<()>;

    /// Set primary screen. Only supported in headless mode.
    /// Note that this changes the coordinate system origin to the top-left
    /// of the new primary screen, updating the bounds and work areas
    /// of all existing screens accordingly.
    ///
    /// CDP: `Emulation.setPrimaryScreen`
    async fn emulation_set_primary_screen(&self, screen_id: &ScreenId) -> Result<()>;
}

// ── Impl ─────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetFocusEmulationEnabledInternalParams {
    enabled: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetCPUThrottlingRateInternalParams {
    rate: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetSafeAreaInsetsOverrideInternalParams<'a> {
    insets: &'a SafeAreaInsets,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetDevicePostureOverrideInternalParams<'a> {
    posture: &'a DevicePosture,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetDisplayFeaturesOverrideInternalParams<'a> {
    features: &'a [DisplayFeature],
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetScrollbarsHiddenInternalParams {
    hidden: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetDocumentCookieDisabledInternalParams {
    disabled: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GetOverriddenSensorInformationInternalParams<'a> {
    r#type: &'a SensorType,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetSensorOverrideReadingsInternalParams<'a> {
    r#type: &'a SensorType,
    reading: &'a SensorReading,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetPressureStateOverrideInternalParams<'a> {
    source: &'a PressureSource,
    state: &'a PressureState,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetIdleOverrideInternalParams {
    is_user_active: bool,
    is_screen_unlocked: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetPageScaleFactorInternalParams {
    page_scale_factor: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetScriptExecutionDisabledInternalParams {
    value: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetTimezoneOverrideInternalParams<'a> {
    timezone_id: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetDisabledImageTypesInternalParams<'a> {
    image_types: &'a [DisabledImageType],
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetHardwareConcurrencyOverrideInternalParams {
    hardware_concurrency: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetAutomationOverrideInternalParams {
    enabled: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetSmallViewportHeightDifferenceOverrideInternalParams {
    difference: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RemoveScreenInternalParams<'a> {
    screen_id: &'a ScreenId,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetPrimaryScreenInternalParams<'a> {
    screen_id: &'a ScreenId,
}

impl EmulationCommands for CdpSession {
    async fn emulation_clear_device_metrics_override(&self) -> Result<()> {
        self.call_no_response("Emulation.clearDeviceMetricsOverride", &serde_json::json!({})).await
    }

    async fn emulation_clear_geolocation_override(&self) -> Result<()> {
        self.call_no_response("Emulation.clearGeolocationOverride", &serde_json::json!({})).await
    }

    async fn emulation_reset_page_scale_factor(&self) -> Result<()> {
        self.call_no_response("Emulation.resetPageScaleFactor", &serde_json::json!({})).await
    }

    async fn emulation_set_focus_emulation_enabled(&self, enabled: bool) -> Result<()> {
        let params = SetFocusEmulationEnabledInternalParams { enabled };
        self.call_no_response("Emulation.setFocusEmulationEnabled", &params).await
    }

    async fn emulation_set_auto_dark_mode_override(&self, params: &SetAutoDarkModeOverrideParams) -> Result<()> {
        self.call_no_response("Emulation.setAutoDarkModeOverride", params).await
    }

    async fn emulation_set_cpu_throttling_rate(&self, rate: f64) -> Result<()> {
        let params = SetCPUThrottlingRateInternalParams { rate };
        self.call_no_response("Emulation.setCPUThrottlingRate", &params).await
    }

    async fn emulation_set_default_background_color_override(&self, params: &SetDefaultBackgroundColorOverrideParams) -> Result<()> {
        self.call_no_response("Emulation.setDefaultBackgroundColorOverride", params).await
    }

    async fn emulation_set_safe_area_insets_override(&self, insets: &SafeAreaInsets) -> Result<()> {
        let params = SetSafeAreaInsetsOverrideInternalParams { insets };
        self.call_no_response("Emulation.setSafeAreaInsetsOverride", &params).await
    }

    async fn emulation_set_device_metrics_override(&self, params: &SetDeviceMetricsOverrideParams) -> Result<()> {
        self.call_no_response("Emulation.setDeviceMetricsOverride", params).await
    }

    async fn emulation_set_device_posture_override(&self, posture: &DevicePosture) -> Result<()> {
        let params = SetDevicePostureOverrideInternalParams { posture };
        self.call_no_response("Emulation.setDevicePostureOverride", &params).await
    }

    async fn emulation_clear_device_posture_override(&self) -> Result<()> {
        self.call_no_response("Emulation.clearDevicePostureOverride", &serde_json::json!({})).await
    }

    async fn emulation_set_display_features_override(&self, features: &[DisplayFeature]) -> Result<()> {
        let params = SetDisplayFeaturesOverrideInternalParams { features };
        self.call_no_response("Emulation.setDisplayFeaturesOverride", &params).await
    }

    async fn emulation_clear_display_features_override(&self) -> Result<()> {
        self.call_no_response("Emulation.clearDisplayFeaturesOverride", &serde_json::json!({})).await
    }

    async fn emulation_set_scrollbars_hidden(&self, hidden: bool) -> Result<()> {
        let params = SetScrollbarsHiddenInternalParams { hidden };
        self.call_no_response("Emulation.setScrollbarsHidden", &params).await
    }

    async fn emulation_set_document_cookie_disabled(&self, disabled: bool) -> Result<()> {
        let params = SetDocumentCookieDisabledInternalParams { disabled };
        self.call_no_response("Emulation.setDocumentCookieDisabled", &params).await
    }

    async fn emulation_set_emit_touch_events_for_mouse(&self, params: &SetEmitTouchEventsForMouseParams) -> Result<()> {
        self.call_no_response("Emulation.setEmitTouchEventsForMouse", params).await
    }

    async fn emulation_set_emulated_media(&self, params: &SetEmulatedMediaParams) -> Result<()> {
        self.call_no_response("Emulation.setEmulatedMedia", params).await
    }

    async fn emulation_set_emulated_vision_deficiency(&self, params: &SetEmulatedVisionDeficiencyParams) -> Result<()> {
        self.call_no_response("Emulation.setEmulatedVisionDeficiency", params).await
    }

    async fn emulation_set_emulated_os_text_scale(&self, params: &SetEmulatedOSTextScaleParams) -> Result<()> {
        self.call_no_response("Emulation.setEmulatedOSTextScale", params).await
    }

    async fn emulation_set_geolocation_override(&self, params: &SetGeolocationOverrideParams) -> Result<()> {
        self.call_no_response("Emulation.setGeolocationOverride", params).await
    }

    async fn emulation_get_overridden_sensor_information(&self, r#type: &SensorType) -> Result<GetOverriddenSensorInformationReturn> {
        let params = GetOverriddenSensorInformationInternalParams { r#type };
        self.call("Emulation.getOverriddenSensorInformation", &params).await
    }

    async fn emulation_set_sensor_override_enabled(&self, params: &SetSensorOverrideEnabledParams) -> Result<()> {
        self.call_no_response("Emulation.setSensorOverrideEnabled", params).await
    }

    async fn emulation_set_sensor_override_readings(&self, r#type: &SensorType, reading: &SensorReading) -> Result<()> {
        let params = SetSensorOverrideReadingsInternalParams { r#type, reading };
        self.call_no_response("Emulation.setSensorOverrideReadings", &params).await
    }

    async fn emulation_set_pressure_source_override_enabled(&self, params: &SetPressureSourceOverrideEnabledParams) -> Result<()> {
        self.call_no_response("Emulation.setPressureSourceOverrideEnabled", params).await
    }

    async fn emulation_set_pressure_state_override(&self, source: &PressureSource, state: &PressureState) -> Result<()> {
        let params = SetPressureStateOverrideInternalParams { source, state };
        self.call_no_response("Emulation.setPressureStateOverride", &params).await
    }

    async fn emulation_set_pressure_data_override(&self, params: &SetPressureDataOverrideParams) -> Result<()> {
        self.call_no_response("Emulation.setPressureDataOverride", params).await
    }

    async fn emulation_set_idle_override(&self, is_user_active: bool, is_screen_unlocked: bool) -> Result<()> {
        let params = SetIdleOverrideInternalParams { is_user_active, is_screen_unlocked };
        self.call_no_response("Emulation.setIdleOverride", &params).await
    }

    async fn emulation_clear_idle_override(&self) -> Result<()> {
        self.call_no_response("Emulation.clearIdleOverride", &serde_json::json!({})).await
    }

    async fn emulation_set_page_scale_factor(&self, page_scale_factor: f64) -> Result<()> {
        let params = SetPageScaleFactorInternalParams { page_scale_factor };
        self.call_no_response("Emulation.setPageScaleFactor", &params).await
    }

    async fn emulation_set_script_execution_disabled(&self, value: bool) -> Result<()> {
        let params = SetScriptExecutionDisabledInternalParams { value };
        self.call_no_response("Emulation.setScriptExecutionDisabled", &params).await
    }

    async fn emulation_set_touch_emulation_enabled(&self, params: &SetTouchEmulationEnabledParams) -> Result<()> {
        self.call_no_response("Emulation.setTouchEmulationEnabled", params).await
    }

    async fn emulation_set_virtual_time_policy(&self, params: &SetVirtualTimePolicyParams) -> Result<SetVirtualTimePolicyReturn> {
        self.call("Emulation.setVirtualTimePolicy", params).await
    }

    async fn emulation_set_locale_override(&self, params: &SetLocaleOverrideParams) -> Result<()> {
        self.call_no_response("Emulation.setLocaleOverride", params).await
    }

    async fn emulation_set_timezone_override(&self, timezone_id: &str) -> Result<()> {
        let params = SetTimezoneOverrideInternalParams { timezone_id };
        self.call_no_response("Emulation.setTimezoneOverride", &params).await
    }

    async fn emulation_set_disabled_image_types(&self, image_types: &[DisabledImageType]) -> Result<()> {
        let params = SetDisabledImageTypesInternalParams { image_types };
        self.call_no_response("Emulation.setDisabledImageTypes", &params).await
    }

    async fn emulation_set_data_saver_override(&self, params: &SetDataSaverOverrideParams) -> Result<()> {
        self.call_no_response("Emulation.setDataSaverOverride", params).await
    }

    async fn emulation_set_hardware_concurrency_override(&self, hardware_concurrency: i64) -> Result<()> {
        let params = SetHardwareConcurrencyOverrideInternalParams { hardware_concurrency };
        self.call_no_response("Emulation.setHardwareConcurrencyOverride", &params).await
    }

    async fn emulation_set_user_agent_override(&self, params: &SetUserAgentOverrideParams) -> Result<()> {
        self.call_no_response("Emulation.setUserAgentOverride", params).await
    }

    async fn emulation_set_automation_override(&self, enabled: bool) -> Result<()> {
        let params = SetAutomationOverrideInternalParams { enabled };
        self.call_no_response("Emulation.setAutomationOverride", &params).await
    }

    async fn emulation_set_small_viewport_height_difference_override(&self, difference: i64) -> Result<()> {
        let params = SetSmallViewportHeightDifferenceOverrideInternalParams { difference };
        self.call_no_response("Emulation.setSmallViewportHeightDifferenceOverride", &params).await
    }

    async fn emulation_get_screen_infos(&self) -> Result<GetScreenInfosReturn> {
        self.call("Emulation.getScreenInfos", &serde_json::json!({})).await
    }

    async fn emulation_add_screen(&self, params: &AddScreenParams) -> Result<AddScreenReturn> {
        self.call("Emulation.addScreen", params).await
    }

    async fn emulation_update_screen(&self, params: &UpdateScreenParams) -> Result<UpdateScreenReturn> {
        self.call("Emulation.updateScreen", params).await
    }

    async fn emulation_remove_screen(&self, screen_id: &ScreenId) -> Result<()> {
        let params = RemoveScreenInternalParams { screen_id };
        self.call_no_response("Emulation.removeScreen", &params).await
    }

    async fn emulation_set_primary_screen(&self, screen_id: &ScreenId) -> Result<()> {
        let params = SetPrimaryScreenInternalParams { screen_id };
        self.call_no_response("Emulation.setPrimaryScreen", &params).await
    }
}
