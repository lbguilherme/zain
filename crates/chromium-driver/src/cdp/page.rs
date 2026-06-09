use serde::{Deserialize, Serialize};

use crate::cdp::common::AdAncestry;
use crate::cdp::common::LoaderId;
use crate::cdp::common::MonotonicTime;
use crate::cdp::common::ResourceType;
use crate::cdp::common::SearchMatch;
use crate::cdp::common::TimeSinceEpoch;
use crate::cdp::dom::BackendNodeId;
use crate::cdp::dom::Rect;
use crate::cdp::io::StreamHandle;
use crate::cdp::runtime::ExecutionContextId;
use crate::cdp::runtime::StackTrace;
use crate::error::Result;
use crate::session::CdpSession;

// ── Types ────────────────────────────────────────────────────────────────────

/// Unique frame identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FrameId(pub String);

/// Indicates whether a frame has been identified as an ad.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdFrameType {
    #[default]
    #[serde(rename = "none")]
    None,
    #[serde(rename = "child")]
    Child,
    #[serde(rename = "root")]
    Root,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdFrameExplanation {
    #[default]
    ParentIsAd,
    CreatedByAdScript,
    MatchedBlockingRule,
}

/// Indicates whether a frame has been identified as an ad and why.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdFrameStatus {
    pub ad_frame_type: AdFrameType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub explanations: Option<Vec<AdFrameExplanation>>,
}

/// Indicates whether the frame is a secure context and why it is the case.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecureContextType {
    #[default]
    Secure,
    SecureLocalhost,
    InsecureScheme,
    InsecureAncestor,
}

/// Indicates whether the frame is cross-origin isolated and why it is the case.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CrossOriginIsolatedContextType {
    #[default]
    Isolated,
    NotIsolated,
    NotIsolatedFeatureDisabled,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum GatedAPIFeatures {
    #[default]
    SharedArrayBuffers,
    SharedArrayBuffersTransferAllowed,
    PerformanceMeasureMemory,
    PerformanceProfile,
}

/// All Permissions Policy features. This enum should match the one defined
/// in services/network/public/cpp/permissions_policy/permissions_policy_features.json5.
/// LINT.IfChange(PermissionsPolicyFeature)
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionsPolicyFeature {
    #[default]
    #[serde(rename = "accelerometer")]
    Accelerometer,
    #[serde(rename = "all-screens-capture")]
    AllScreensCapture,
    #[serde(rename = "ambient-light-sensor")]
    AmbientLightSensor,
    #[serde(rename = "aria-notify")]
    AriaNotify,
    #[serde(rename = "attribution-reporting")]
    AttributionReporting,
    #[serde(rename = "autofill")]
    Autofill,
    #[serde(rename = "autoplay")]
    Autoplay,
    #[serde(rename = "bluetooth")]
    Bluetooth,
    #[serde(rename = "browsing-topics")]
    BrowsingTopics,
    #[serde(rename = "camera")]
    Camera,
    #[serde(rename = "captured-surface-control")]
    CapturedSurfaceControl,
    #[serde(rename = "ch-dpr")]
    ChDpr,
    #[serde(rename = "ch-device-memory")]
    ChDeviceMemory,
    #[serde(rename = "ch-downlink")]
    ChDownlink,
    #[serde(rename = "ch-ect")]
    ChEct,
    #[serde(rename = "ch-prefers-color-scheme")]
    ChPrefersColorScheme,
    #[serde(rename = "ch-prefers-reduced-motion")]
    ChPrefersReducedMotion,
    #[serde(rename = "ch-prefers-reduced-transparency")]
    ChPrefersReducedTransparency,
    #[serde(rename = "ch-rtt")]
    ChRtt,
    #[serde(rename = "ch-save-data")]
    ChSaveData,
    #[serde(rename = "ch-ua")]
    ChUa,
    #[serde(rename = "ch-ua-arch")]
    ChUaArch,
    #[serde(rename = "ch-ua-bitness")]
    ChUaBitness,
    #[serde(rename = "ch-ua-high-entropy-values")]
    ChUaHighEntropyValues,
    #[serde(rename = "ch-ua-platform")]
    ChUaPlatform,
    #[serde(rename = "ch-ua-model")]
    ChUaModel,
    #[serde(rename = "ch-ua-mobile")]
    ChUaMobile,
    #[serde(rename = "ch-ua-form-factors")]
    ChUaFormFactors,
    #[serde(rename = "ch-ua-full-version")]
    ChUaFullVersion,
    #[serde(rename = "ch-ua-full-version-list")]
    ChUaFullVersionList,
    #[serde(rename = "ch-ua-platform-version")]
    ChUaPlatformVersion,
    #[serde(rename = "ch-ua-wow64")]
    ChUaWow64,
    #[serde(rename = "ch-viewport-height")]
    ChViewportHeight,
    #[serde(rename = "ch-viewport-width")]
    ChViewportWidth,
    #[serde(rename = "ch-width")]
    ChWidth,
    #[serde(rename = "clipboard-read")]
    ClipboardRead,
    #[serde(rename = "clipboard-write")]
    ClipboardWrite,
    #[serde(rename = "compute-pressure")]
    ComputePressure,
    #[serde(rename = "controlled-frame")]
    ControlledFrame,
    #[serde(rename = "cross-origin-isolated")]
    CrossOriginIsolated,
    #[serde(rename = "deferred-fetch")]
    DeferredFetch,
    #[serde(rename = "deferred-fetch-minimal")]
    DeferredFetchMinimal,
    #[serde(rename = "device-attributes")]
    DeviceAttributes,
    #[serde(rename = "digital-credentials-create")]
    DigitalCredentialsCreate,
    #[serde(rename = "digital-credentials-get")]
    DigitalCredentialsGet,
    #[serde(rename = "direct-sockets")]
    DirectSockets,
    #[serde(rename = "direct-sockets-multicast")]
    DirectSocketsMulticast,
    #[serde(rename = "direct-sockets-private")]
    DirectSocketsPrivate,
    #[serde(rename = "display-capture")]
    DisplayCapture,
    #[serde(rename = "document-domain")]
    DocumentDomain,
    #[serde(rename = "encrypted-media")]
    EncryptedMedia,
    #[serde(rename = "execution-while-out-of-viewport")]
    ExecutionWhileOutOfViewport,
    #[serde(rename = "execution-while-not-rendered")]
    ExecutionWhileNotRendered,
    #[serde(rename = "focus-without-user-activation")]
    FocusWithoutUserActivation,
    #[serde(rename = "fullscreen")]
    Fullscreen,
    #[serde(rename = "frobulate")]
    Frobulate,
    #[serde(rename = "gamepad")]
    Gamepad,
    #[serde(rename = "geolocation")]
    Geolocation,
    #[serde(rename = "gyroscope")]
    Gyroscope,
    #[serde(rename = "hid")]
    Hid,
    #[serde(rename = "identity-credentials-get")]
    IdentityCredentialsGet,
    #[serde(rename = "idle-detection")]
    IdleDetection,
    #[serde(rename = "interest-cohort")]
    InterestCohort,
    #[serde(rename = "join-ad-interest-group")]
    JoinAdInterestGroup,
    #[serde(rename = "keyboard-map")]
    KeyboardMap,
    #[serde(rename = "language-detector")]
    LanguageDetector,
    #[serde(rename = "language-model")]
    LanguageModel,
    #[serde(rename = "local-fonts")]
    LocalFonts,
    #[serde(rename = "local-network")]
    LocalNetwork,
    #[serde(rename = "local-network-access")]
    LocalNetworkAccess,
    #[serde(rename = "loopback-network")]
    LoopbackNetwork,
    #[serde(rename = "magnetometer")]
    Magnetometer,
    #[serde(rename = "manual-text")]
    ManualText,
    #[serde(rename = "media-playback-while-not-visible")]
    MediaPlaybackWhileNotVisible,
    #[serde(rename = "microphone")]
    Microphone,
    #[serde(rename = "midi")]
    Midi,
    #[serde(rename = "on-device-speech-recognition")]
    OnDeviceSpeechRecognition,
    #[serde(rename = "otp-credentials")]
    OtpCredentials,
    #[serde(rename = "payment")]
    Payment,
    #[serde(rename = "picture-in-picture")]
    PictureInPicture,
    #[serde(rename = "private-aggregation")]
    PrivateAggregation,
    #[serde(rename = "private-state-token-issuance")]
    PrivateStateTokenIssuance,
    #[serde(rename = "private-state-token-redemption")]
    PrivateStateTokenRedemption,
    #[serde(rename = "publickey-credentials-create")]
    PublickeyCredentialsCreate,
    #[serde(rename = "publickey-credentials-get")]
    PublickeyCredentialsGet,
    #[serde(rename = "record-ad-auction-events")]
    RecordAdAuctionEvents,
    #[serde(rename = "rewriter")]
    Rewriter,
    #[serde(rename = "run-ad-auction")]
    RunAdAuction,
    #[serde(rename = "screen-wake-lock")]
    ScreenWakeLock,
    #[serde(rename = "serial")]
    Serial,
    #[serde(rename = "shared-storage")]
    SharedStorage,
    #[serde(rename = "shared-storage-select-url")]
    SharedStorageSelectUrl,
    #[serde(rename = "smart-card")]
    SmartCard,
    #[serde(rename = "speaker-selection")]
    SpeakerSelection,
    #[serde(rename = "storage-access")]
    StorageAccess,
    #[serde(rename = "sub-apps")]
    SubApps,
    #[serde(rename = "summarizer")]
    Summarizer,
    #[serde(rename = "sync-xhr")]
    SyncXhr,
    #[serde(rename = "tools")]
    Tools,
    #[serde(rename = "translator")]
    Translator,
    #[serde(rename = "unload")]
    Unload,
    #[serde(rename = "usb")]
    Usb,
    #[serde(rename = "usb-unrestricted")]
    UsbUnrestricted,
    #[serde(rename = "vertical-scroll")]
    VerticalScroll,
    #[serde(rename = "web-app-installation")]
    WebAppInstallation,
    #[serde(rename = "webnn")]
    Webnn,
    #[serde(rename = "web-printing")]
    WebPrinting,
    #[serde(rename = "web-share")]
    WebShare,
    #[serde(rename = "window-management")]
    WindowManagement,
    #[serde(rename = "writer")]
    Writer,
    #[serde(rename = "xr-spatial-tracking")]
    XrSpatialTracking,
}

/// Reason for a permissions policy feature to be disabled.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionsPolicyBlockReason {
    #[default]
    Header,
    IframeAttribute,
    InFencedFrameTree,
    InIsolatedApp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionsPolicyBlockLocator {
    pub frame_id: FrameId,
    pub block_reason: PermissionsPolicyBlockReason,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionsPolicyFeatureState {
    pub feature: PermissionsPolicyFeature,
    pub allowed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locator: Option<PermissionsPolicyBlockLocator>,
}

/// Origin Trial(https://www.chromium.org/blink/origin-trials) support.
/// Status for an Origin Trial token.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum OriginTrialTokenStatus {
    #[default]
    Success,
    NotSupported,
    Insecure,
    Expired,
    WrongOrigin,
    InvalidSignature,
    Malformed,
    WrongVersion,
    FeatureDisabled,
    TokenDisabled,
    FeatureDisabledForUser,
    UnknownTrial,
}

/// Status for an Origin Trial.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum OriginTrialStatus {
    #[default]
    Enabled,
    ValidTokenNotProvided,
    OSNotSupported,
    TrialNotAllowed,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum OriginTrialUsageRestriction {
    #[default]
    None,
    Subset,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OriginTrialToken {
    pub origin: String,
    pub match_sub_domains: bool,
    pub trial_name: String,
    pub expiry_time: TimeSinceEpoch,
    pub is_third_party: bool,
    pub usage_restriction: OriginTrialUsageRestriction,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OriginTrialTokenWithStatus {
    pub raw_token_text: String,
    /// `parsedToken` is present only when the token is extractable and
    /// parsable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parsed_token: Option<OriginTrialToken>,
    pub status: OriginTrialTokenStatus,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OriginTrial {
    pub trial_name: String,
    pub status: OriginTrialStatus,
    pub tokens_with_status: Vec<OriginTrialTokenWithStatus>,
}

/// Additional information about the frame document's security origin.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityOriginDetails {
    /// Indicates whether the frame document's security origin is one
    /// of the local hostnames (e.g. "localhost") or IP addresses (IPv4
    /// 127.0.0.0/8 or IPv6 ::1).
    pub is_localhost: bool,
}

/// Information about the Frame on the page.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Frame {
    /// Frame unique identifier.
    pub id: FrameId,
    /// Parent frame identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<FrameId>,
    /// Identifier of the loader associated with this frame.
    pub loader_id: LoaderId,
    /// Frame's name as specified in the tag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Frame document's URL without fragment.
    pub url: String,
    /// Frame document's URL fragment including the '#'.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url_fragment: Option<String>,
    /// Frame document's registered domain, taking the public suffixes list into account.
    /// Extracted from the Frame's url.
    /// Example URLs: http://www.google.com/file.html -> "google.com"
    ///               http://a.b.co.uk/file.html      -> "b.co.uk".
    pub domain_and_registry: String,
    /// Frame document's security origin.
    pub security_origin: String,
    /// Additional details about the frame document's security origin.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub security_origin_details: Option<SecurityOriginDetails>,
    /// Frame document's mimeType as determined by the browser.
    pub mime_type: String,
    /// If the frame failed to load, this contains the URL that could not be loaded. Note that unlike url above, this URL may contain a fragment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unreachable_url: Option<String>,
    /// Indicates whether this frame was tagged as an ad and why.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ad_frame_status: Option<AdFrameStatus>,
    /// Indicates whether the main document is a secure context and explains why that is the case.
    pub secure_context_type: SecureContextType,
    /// Indicates whether this is a cross origin isolated context.
    pub cross_origin_isolated_context_type: CrossOriginIsolatedContextType,
    /// Indicated which gated APIs / features are available.
    #[serde(rename = "gatedAPIFeatures")]
    pub gated_api_features: Vec<GatedAPIFeatures>,
}

/// Information about the Resource on the page.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameResource {
    /// Resource URL.
    pub url: String,
    /// Type of this resource.
    pub r#type: ResourceType,
    /// Resource mimeType as determined by the browser.
    pub mime_type: String,
    /// last-modified timestamp as reported by server.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_modified: Option<TimeSinceEpoch>,
    /// Resource content size.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_size: Option<f64>,
    /// True if the resource failed to load.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failed: Option<bool>,
    /// True if the resource was canceled during loading.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canceled: Option<bool>,
}

/// Information about the Frame hierarchy along with their cached resources.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameResourceTree {
    /// Frame information for this tree item.
    pub frame: Frame,
    /// Child frames.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub child_frames: Option<Vec<Box<FrameResourceTree>>>,
    /// Information about frame resources.
    pub resources: Vec<FrameResource>,
}

/// Information about the Frame hierarchy.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameTree {
    /// Frame information for this tree item.
    pub frame: Frame,
    /// Child frames.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub child_frames: Option<Vec<Box<FrameTree>>>,
}

/// Unique script identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ScriptIdentifier(pub String);

/// Transition type.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransitionType {
    #[default]
    #[serde(rename = "link")]
    Link,
    #[serde(rename = "typed")]
    Typed,
    #[serde(rename = "address_bar")]
    AddressBar,
    #[serde(rename = "auto_bookmark")]
    AutoBookmark,
    #[serde(rename = "auto_subframe")]
    AutoSubframe,
    #[serde(rename = "manual_subframe")]
    ManualSubframe,
    #[serde(rename = "generated")]
    Generated,
    #[serde(rename = "auto_toplevel")]
    AutoToplevel,
    #[serde(rename = "form_submit")]
    FormSubmit,
    #[serde(rename = "reload")]
    Reload,
    #[serde(rename = "keyword")]
    Keyword,
    #[serde(rename = "keyword_generated")]
    KeywordGenerated,
    #[serde(rename = "other")]
    Other,
}

/// Navigation history entry.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NavigationEntry {
    /// Unique id of the navigation history entry.
    pub id: i64,
    /// URL of the navigation history entry.
    pub url: String,
    /// URL that the user typed in the url bar.
    #[serde(rename = "userTypedURL")]
    pub user_typed_url: String,
    /// Title of the navigation history entry.
    pub title: String,
    /// Transition type.
    pub transition_type: TransitionType,
}

/// Screencast frame metadata.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreencastFrameMetadata {
    /// Top offset in DIP.
    pub offset_top: f64,
    /// Page scale factor.
    pub page_scale_factor: f64,
    /// Device screen width in DIP.
    pub device_width: f64,
    /// Device screen height in DIP.
    pub device_height: f64,
    /// Position of horizontal scroll in CSS pixels.
    pub scroll_offset_x: f64,
    /// Position of vertical scroll in CSS pixels.
    pub scroll_offset_y: f64,
    /// Frame swap timestamp.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<TimeSinceEpoch>,
}

/// Javascript dialog type.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DialogType {
    #[default]
    #[serde(rename = "alert")]
    Alert,
    #[serde(rename = "confirm")]
    Confirm,
    #[serde(rename = "prompt")]
    Prompt,
    #[serde(rename = "beforeunload")]
    Beforeunload,
}

/// Error while paring app manifest.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppManifestError {
    /// Error message.
    pub message: String,
    /// If critical, this is a non-recoverable parse error.
    pub critical: i64,
    /// Error line.
    pub line: i64,
    /// Error column.
    pub column: i64,
}

/// Parsed app manifest properties.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppManifestParsedProperties {
    /// Computed scope value.
    pub scope: String,
}

/// Layout viewport position and dimensions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutViewport {
    /// Horizontal offset relative to the document (CSS pixels).
    pub page_x: i64,
    /// Vertical offset relative to the document (CSS pixels).
    pub page_y: i64,
    /// Width (CSS pixels), excludes scrollbar if present.
    pub client_width: i64,
    /// Height (CSS pixels), excludes scrollbar if present.
    pub client_height: i64,
}

/// Visual viewport position, dimensions, and scale.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisualViewport {
    /// Horizontal offset relative to the layout viewport (CSS pixels).
    pub offset_x: f64,
    /// Vertical offset relative to the layout viewport (CSS pixels).
    pub offset_y: f64,
    /// Horizontal offset relative to the document (CSS pixels).
    pub page_x: f64,
    /// Vertical offset relative to the document (CSS pixels).
    pub page_y: f64,
    /// Width (CSS pixels), excludes scrollbar if present.
    pub client_width: f64,
    /// Height (CSS pixels), excludes scrollbar if present.
    pub client_height: f64,
    /// Scale relative to the ideal viewport (size at width=device-width).
    pub scale: f64,
    /// Page zoom factor (CSS to device independent pixels ratio).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zoom: Option<f64>,
}

/// Viewport for capturing screenshot.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Viewport {
    /// X offset in device independent pixels (dip).
    pub x: f64,
    /// Y offset in device independent pixels (dip).
    pub y: f64,
    /// Rectangle width in device independent pixels (dip).
    pub width: f64,
    /// Rectangle height in device independent pixels (dip).
    pub height: f64,
    /// Page scale factor.
    pub scale: f64,
}

/// Generic font families collection.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FontFamilies {
    /// The standard font-family.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub standard: Option<String>,
    /// The fixed font-family.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixed: Option<String>,
    /// The serif font-family.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub serif: Option<String>,
    /// The sansSerif font-family.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sans_serif: Option<String>,
    /// The cursive font-family.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursive: Option<String>,
    /// The fantasy font-family.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fantasy: Option<String>,
    /// The math font-family.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub math: Option<String>,
}

/// Font families collection for a script.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScriptFontFamilies {
    /// Name of the script which these font families are defined for.
    pub script: String,
    /// Generic font families collection for the script.
    pub font_families: FontFamilies,
}

/// Default font sizes.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FontSizes {
    /// Default standard font size.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub standard: Option<i64>,
    /// Default fixed font size.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixed: Option<i64>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClientNavigationReason {
    #[default]
    #[serde(rename = "anchorClick")]
    AnchorClick,
    #[serde(rename = "formSubmissionGet")]
    FormSubmissionGet,
    #[serde(rename = "formSubmissionPost")]
    FormSubmissionPost,
    #[serde(rename = "httpHeaderRefresh")]
    HttpHeaderRefresh,
    #[serde(rename = "initialFrameNavigation")]
    InitialFrameNavigation,
    #[serde(rename = "metaTagRefresh")]
    MetaTagRefresh,
    #[serde(rename = "other")]
    Other,
    #[serde(rename = "pageBlockInterstitial")]
    PageBlockInterstitial,
    #[serde(rename = "reload")]
    Reload,
    #[serde(rename = "scriptInitiated")]
    ScriptInitiated,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClientNavigationDisposition {
    #[default]
    #[serde(rename = "currentTab")]
    CurrentTab,
    #[serde(rename = "newTab")]
    NewTab,
    #[serde(rename = "newWindow")]
    NewWindow,
    #[serde(rename = "download")]
    Download,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallabilityErrorArgument {
    /// Argument name (e.g. name:'minimum-icon-size-in-pixels').
    pub name: String,
    /// Argument value (e.g. value:'64').
    pub value: String,
}

/// The installability error.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallabilityError {
    /// The error id (e.g. 'manifest-missing-suitable-icon').
    pub error_id: String,
    /// The list of error arguments (e.g. {name:'minimum-icon-size-in-pixels', value:'64'}).
    pub error_arguments: Vec<InstallabilityErrorArgument>,
}

/// The referring-policy used for the navigation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReferrerPolicy {
    #[default]
    #[serde(rename = "noReferrer")]
    NoReferrer,
    #[serde(rename = "noReferrerWhenDowngrade")]
    NoReferrerWhenDowngrade,
    #[serde(rename = "origin")]
    Origin,
    #[serde(rename = "originWhenCrossOrigin")]
    OriginWhenCrossOrigin,
    #[serde(rename = "sameOrigin")]
    SameOrigin,
    #[serde(rename = "strictOrigin")]
    StrictOrigin,
    #[serde(rename = "strictOriginWhenCrossOrigin")]
    StrictOriginWhenCrossOrigin,
    #[serde(rename = "unsafeUrl")]
    UnsafeUrl,
}

/// Per-script compilation cache parameters for `Page.produceCompilationCache`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompilationCacheParams {
    /// The URL of the script to produce a compilation cache entry for.
    pub url: String,
    /// A hint to the backend whether eager compilation is recommended.
    /// (the actual compilation mode used is upon backend discretion).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eager: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileFilter {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accepts: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileHandler {
    pub action: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icons: Option<Vec<ImageResource>>,
    /// Mimic a map, name is the key, accepts is the value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accepts: Option<Vec<FileFilter>>,
    /// Won't repeat the enums, using string for easy comparison. Same as the
    /// other enums below.
    pub launch_type: String,
}

/// The image definition used in both icon and screenshot.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageResource {
    /// The src field in the definition, but changing to url in favor of
    /// consistency.
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sizes: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchHandler {
    pub client_mode: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolHandler {
    pub protocol: String,
    pub url: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelatedApplication {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub url: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopeExtension {
    /// Instead of using tuple, this field always returns the serialized string
    /// for easy understanding and comparison.
    pub origin: String,
    pub has_origin_wildcard: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Screenshot {
    pub image: ImageResource,
    pub form_factor: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareTarget {
    pub action: String,
    pub method: String,
    pub enctype: String,
    /// Embed the ShareTargetParams.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub files: Option<Vec<FileFilter>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Shortcut {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebAppManifest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_color: Option<String>,
    /// The extra description provided by the manifest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
    /// The overrided display mode controlled by the user.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_overrides: Option<Vec<String>>,
    /// The handlers to open files.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_handlers: Option<Vec<FileHandler>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icons: Option<Vec<ImageResource>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    /// TODO(crbug.com/1231886): This field is non-standard and part of a Chrome
    /// experiment. See:
    /// https://github.com/WICG/web-app-launch/blob/main/launch_handler.md.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub launch_handler: Option<LaunchHandler>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orientation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefer_related_applications: Option<bool>,
    /// The handlers to open protocols.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protocol_handlers: Option<Vec<ProtocolHandler>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub related_applications: Option<Vec<RelatedApplication>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    /// Non-standard, see
    /// https://github.com/WICG/manifest-incubations/blob/gh-pages/scope_extensions-explainer.md.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope_extensions: Option<Vec<ScopeExtension>>,
    /// The screenshots used by chromium.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub screenshots: Option<Vec<Screenshot>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub share_target: Option<ShareTarget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub short_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shortcuts: Option<Vec<Shortcut>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme_color: Option<String>,
}

/// The type of a frameNavigated event.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum NavigationType {
    #[default]
    Navigation,
    BackForwardCacheRestore,
}

/// List of not restored reasons for back-forward cache.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackForwardCacheNotRestoredReason {
    #[default]
    NotPrimaryMainFrame,
    BackForwardCacheDisabled,
    RelatedActiveContentsExist,
    HTTPStatusNotOK,
    SchemeNotHTTPOrHTTPS,
    Loading,
    WasGrantedMediaAccess,
    DisableForRenderFrameHostCalled,
    DomainNotAllowed,
    HTTPMethodNotGET,
    SubframeIsNavigating,
    Timeout,
    CacheLimit,
    JavaScriptExecution,
    RendererProcessKilled,
    RendererProcessCrashed,
    SchedulerTrackedFeatureUsed,
    ConflictingBrowsingInstance,
    CacheFlushed,
    ServiceWorkerVersionActivation,
    SessionRestored,
    ServiceWorkerPostMessage,
    EnteredBackForwardCacheBeforeServiceWorkerHostAdded,
    #[serde(rename = "RenderFrameHostReused_SameSite")]
    RenderFrameHostReusedSameSite,
    #[serde(rename = "RenderFrameHostReused_CrossSite")]
    RenderFrameHostReusedCrossSite,
    ServiceWorkerClaim,
    IgnoreEventAndEvict,
    HaveInnerContents,
    TimeoutPuttingInCache,
    BackForwardCacheDisabledByLowMemory,
    BackForwardCacheDisabledByCommandLine,
    NetworkRequestDatapipeDrainedAsBytesConsumer,
    NetworkRequestRedirected,
    NetworkRequestTimeout,
    NetworkExceedsBufferLimit,
    NavigationCancelledWhileRestoring,
    NotMostRecentNavigationEntry,
    BackForwardCacheDisabledForPrerender,
    UserAgentOverrideDiffers,
    ForegroundCacheLimit,
    ForwardCacheDisabled,
    BrowsingInstanceNotSwapped,
    BackForwardCacheDisabledForDelegate,
    UnloadHandlerExistsInMainFrame,
    UnloadHandlerExistsInSubFrame,
    ServiceWorkerUnregistration,
    CacheControlNoStore,
    CacheControlNoStoreCookieModified,
    CacheControlNoStoreHTTPOnlyCookieModified,
    NoResponseHead,
    Unknown,
    ActivationNavigationsDisallowedForBug1234857,
    ErrorDocument,
    FencedFramesEmbedder,
    CookieDisabled,
    HTTPAuthRequired,
    CookieFlushed,
    BroadcastChannelOnMessage,
    WebViewSettingsChanged,
    WebViewJavaScriptObjectChanged,
    WebViewMessageListenerInjected,
    WebViewSafeBrowsingAllowlistChanged,
    WebViewDocumentStartJavascriptChanged,
    WebSocket,
    WebTransport,
    WebRTC,
    MainResourceHasCacheControlNoStore,
    MainResourceHasCacheControlNoCache,
    SubresourceHasCacheControlNoStore,
    SubresourceHasCacheControlNoCache,
    ContainsPlugins,
    DocumentLoaded,
    OutstandingNetworkRequestOthers,
    RequestedMIDIPermission,
    RequestedAudioCapturePermission,
    RequestedVideoCapturePermission,
    RequestedBackForwardCacheBlockedSensors,
    RequestedBackgroundWorkPermission,
    BroadcastChannel,
    WebXR,
    SharedWorker,
    SharedWorkerMessage,
    SharedWorkerWithNoActiveClient,
    WebLocks,
    WebLocksContention,
    WebHID,
    WebBluetooth,
    WebShare,
    RequestedStorageAccessGrant,
    WebNfc,
    OutstandingNetworkRequestFetch,
    OutstandingNetworkRequestXHR,
    AppBanner,
    Printing,
    WebDatabase,
    PictureInPicture,
    SpeechRecognizer,
    IdleManager,
    PaymentManager,
    SpeechSynthesis,
    KeyboardLock,
    WebOTPService,
    OutstandingNetworkRequestDirectSocket,
    InjectedJavascript,
    InjectedStyleSheet,
    KeepaliveRequest,
    IndexedDBEvent,
    Dummy,
    JsNetworkRequestReceivedCacheControlNoStoreResource,
    WebRTCUsedWithCCNS,
    WebTransportUsedWithCCNS,
    WebSocketUsedWithCCNS,
    SmartCard,
    LiveMediaStreamTrack,
    UnloadHandler,
    ParserAborted,
    ContentSecurityHandler,
    ContentWebAuthenticationAPI,
    ContentFileChooser,
    ContentSerial,
    ContentFileSystemAccess,
    ContentMediaDevicesDispatcherHost,
    ContentWebBluetooth,
    ContentWebUSB,
    ContentMediaSessionService,
    ContentScreenReader,
    ContentDiscarded,
    EmbedderPopupBlockerTabHelper,
    EmbedderSafeBrowsingTriggeredPopupBlocker,
    EmbedderSafeBrowsingThreatDetails,
    EmbedderAppBannerManager,
    EmbedderDomDistillerViewerSource,
    EmbedderDomDistillerSelfDeletingRequestDelegate,
    EmbedderOomInterventionTabHelper,
    EmbedderOfflinePage,
    EmbedderChromePasswordManagerClientBindCredentialManager,
    EmbedderPermissionRequestManager,
    EmbedderModalDialog,
    EmbedderExtensions,
    EmbedderExtensionMessaging,
    EmbedderExtensionMessagingForOpenPort,
    EmbedderExtensionSentMessageToCachedFrame,
    EmbedderExtensionFrame,
    RequestedByWebViewClient,
    PostMessageByWebViewClient,
    CacheControlNoStoreDeviceBoundSessionTerminated,
    CacheLimitPrunedOnModerateMemoryPressure,
    CacheLimitPrunedOnCriticalMemoryPressure,
}

/// Types of not restored reasons for back-forward cache.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackForwardCacheNotRestoredReasonType {
    #[default]
    SupportPending,
    PageSupportNeeded,
    Circumstantial,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackForwardCacheBlockingDetails {
    /// Url of the file where blockage happened. Optional because of tests.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Function name where blockage happened. Optional because of anonymous functions and tests.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub function: Option<String>,
    /// Line number in the script (0-based).
    pub line_number: i64,
    /// Column number in the script (0-based).
    pub column_number: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackForwardCacheNotRestoredExplanation {
    /// Type of the reason.
    pub r#type: BackForwardCacheNotRestoredReasonType,
    /// Not restored reason.
    pub reason: BackForwardCacheNotRestoredReason,
    /// Context associated with the reason. The meaning of this context is
    /// dependent on the reason:
    /// - EmbedderExtensionSentMessageToCachedFrame: the extension ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<Vec<BackForwardCacheBlockingDetails>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackForwardCacheNotRestoredExplanationTree {
    /// URL of each frame.
    pub url: String,
    /// Not restored reasons of each frame.
    pub explanations: Vec<BackForwardCacheNotRestoredExplanation>,
    /// Array of children frame.
    pub children: Vec<Box<BackForwardCacheNotRestoredExplanationTree>>,
}

// ── Inline enums ─────────────────────────────────────────────────────────────

/// Image compression format (defaults to png).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CaptureScreenshotFormat {
    #[default]
    #[serde(rename = "jpeg")]
    Jpeg,
    #[serde(rename = "png")]
    Png,
    #[serde(rename = "webp")]
    Webp,
}

/// Format (defaults to mhtml).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CaptureSnapshotFormat {
    #[default]
    #[serde(rename = "mhtml")]
    Mhtml,
}

/// return as stream.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrintToPDFTransferMode {
    #[default]
    ReturnAsBase64,
    ReturnAsStream,
}

/// Image compression format.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum StartScreencastFormat {
    #[default]
    #[serde(rename = "jpeg")]
    Jpeg,
    #[serde(rename = "png")]
    Png,
}

/// Target lifecycle state.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SetWebLifecycleStateState {
    #[default]
    #[serde(rename = "frozen")]
    Frozen,
    #[serde(rename = "active")]
    Active,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SetSPCTransactionModeMode {
    #[default]
    #[serde(rename = "none")]
    None,
    #[serde(rename = "autoAccept")]
    AutoAccept,
    #[serde(rename = "autoChooseToAuthAnotherWay")]
    AutoChooseToAuthAnotherWay,
    #[serde(rename = "autoReject")]
    AutoReject,
    #[serde(rename = "autoOptOut")]
    AutoOptOut,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SetRPHRegistrationModeMode {
    #[default]
    #[serde(rename = "none")]
    None,
    #[serde(rename = "autoAccept")]
    AutoAccept,
    #[serde(rename = "autoReject")]
    AutoReject,
}

/// Input mode.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileChooserOpenedMode {
    #[default]
    #[serde(rename = "selectSingle")]
    SelectSingle,
    #[serde(rename = "selectMultiple")]
    SelectMultiple,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrameDetachedReason {
    #[default]
    #[serde(rename = "remove")]
    Remove,
    #[serde(rename = "swap")]
    Swap,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrameStartedNavigatingNavigationType {
    #[default]
    #[serde(rename = "reload")]
    Reload,
    #[serde(rename = "reloadBypassingCache")]
    ReloadBypassingCache,
    #[serde(rename = "restore")]
    Restore,
    #[serde(rename = "restoreWithPost")]
    RestoreWithPost,
    #[serde(rename = "historySameDocument")]
    HistorySameDocument,
    #[serde(rename = "historyDifferentDocument")]
    HistoryDifferentDocument,
    #[serde(rename = "sameDocument")]
    SameDocument,
    #[serde(rename = "differentDocument")]
    DifferentDocument,
}

/// Navigation type.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum NavigatedWithinDocumentNavigationType {
    #[default]
    #[serde(rename = "fragment")]
    Fragment,
    #[serde(rename = "historyApi")]
    HistoryApi,
    #[serde(rename = "other")]
    Other,
}

// ── Param types ──────────────────────────────────────────────────────────────

/// Parameters for [`PageCommands::page_add_script_to_evaluate_on_new_document`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddScriptToEvaluateOnNewDocumentParams {
    pub source: String,
    /// If specified, creates an isolated world with the given name and evaluates given script in it.
    /// This world name will be used as the ExecutionContextDescription::name when the corresponding
    /// event is emitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub world_name: Option<String>,
    /// Specifies whether command line API should be available to the script, defaults
    /// to false.
    #[serde(rename = "includeCommandLineAPI")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_command_line_api: Option<bool>,
    /// If true, runs the script immediately on existing execution contexts or worlds.
    /// Default: false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_immediately: Option<bool>,
}

/// Parameters for [`PageCommands::page_capture_screenshot`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureScreenshotParams {
    /// Image compression format (defaults to png).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<CaptureScreenshotFormat>,
    /// Compression quality from range [0..100] (jpeg only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<i64>,
    /// Capture the screenshot of a given region only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clip: Option<Viewport>,
    /// Capture the screenshot from the surface, rather than the view. Defaults to true.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_surface: Option<bool>,
    /// Capture the screenshot beyond the viewport. Defaults to false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capture_beyond_viewport: Option<bool>,
    /// Optimize image encoding for speed, not for resulting size (defaults to false)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optimize_for_speed: Option<bool>,
}

/// Parameters for [`PageCommands::page_capture_snapshot`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureSnapshotParams {
    /// Format (defaults to mhtml).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<CaptureSnapshotFormat>,
}

/// Parameters for [`PageCommands::page_create_isolated_world`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateIsolatedWorldParams {
    /// Id of the frame in which the isolated world should be created.
    pub frame_id: FrameId,
    /// An optional name which is reported in the Execution Context.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub world_name: Option<String>,
    /// Whether or not universal access should be granted to the isolated world. This is a powerful
    /// option, use with caution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grant_univeral_access: Option<bool>,
}

/// Parameters for [`PageCommands::page_enable`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnableParams {
    /// If true, the `Page.fileChooserOpened` event will be emitted regardless of the state set by
    /// `Page.setInterceptFileChooserDialog` command (default: false).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_file_chooser_opened_event: Option<bool>,
}

/// Parameters for [`PageCommands::page_get_app_manifest`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetAppManifestParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manifest_id: Option<String>,
}

/// Parameters for [`PageCommands::page_handle_java_script_dialog`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HandleJavaScriptDialogParams {
    /// Whether to accept or dismiss the dialog.
    pub accept: bool,
    /// The text to enter into the dialog prompt before accepting. Used only if this is a prompt
    /// dialog.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_text: Option<String>,
}

/// Parameters for [`PageCommands::page_navigate`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NavigateParams {
    /// URL to navigate the page to.
    pub url: String,
    /// Referrer URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub referrer: Option<String>,
    /// Intended transition type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition_type: Option<TransitionType>,
    /// Frame id to navigate, if not specified navigates the top frame.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame_id: Option<FrameId>,
    /// Referrer-policy used for the navigation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub referrer_policy: Option<ReferrerPolicy>,
}

/// Parameters for [`PageCommands::page_print_to_pdf`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrintToPDFParams {
    /// Paper orientation. Defaults to false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub landscape: Option<bool>,
    /// Display header and footer. Defaults to false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_header_footer: Option<bool>,
    /// Print background graphics. Defaults to false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub print_background: Option<bool>,
    /// Scale of the webpage rendering. Defaults to 1.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<f64>,
    /// Paper width in inches. Defaults to 8.5 inches.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paper_width: Option<f64>,
    /// Paper height in inches. Defaults to 11 inches.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paper_height: Option<f64>,
    /// Top margin in inches. Defaults to 1cm (~0.4 inches).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub margin_top: Option<f64>,
    /// Bottom margin in inches. Defaults to 1cm (~0.4 inches).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub margin_bottom: Option<f64>,
    /// Left margin in inches. Defaults to 1cm (~0.4 inches).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub margin_left: Option<f64>,
    /// Right margin in inches. Defaults to 1cm (~0.4 inches).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub margin_right: Option<f64>,
    /// Paper ranges to print, one based, e.g., '1-5, 8, 11-13'. Pages are
    /// printed in the document order, not in the order specified, and no
    /// more than once.
    /// Defaults to empty string, which implies the entire document is printed.
    /// The page numbers are quietly capped to actual page count of the
    /// document, and ranges beyond the end of the document are ignored.
    /// If this results in no pages to print, an error is reported.
    /// It is an error to specify a range with start greater than end.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_ranges: Option<String>,
    /// HTML template for the print header. Should be valid HTML markup with following
    /// classes used to inject printing values into them:
    /// - `date`: formatted print date
    /// - `title`: document title
    /// - `url`: document location
    /// - `pageNumber`: current page number
    /// - `totalPages`: total pages in the document
    ///
    /// For example, `<span class=title></span>` would generate span containing the title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header_template: Option<String>,
    /// HTML template for the print footer. Should use the same format as the `headerTemplate`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub footer_template: Option<String>,
    /// Whether or not to prefer page size as defined by css. Defaults to false,
    /// in which case the content will be scaled to fit the paper size.
    #[serde(rename = "preferCSSPageSize")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefer_css_page_size: Option<bool>,
    /// return as stream.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transfer_mode: Option<PrintToPDFTransferMode>,
    /// Whether or not to generate tagged (accessible) PDF. Defaults to embedder choice.
    #[serde(rename = "generateTaggedPDF")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generate_tagged_pdf: Option<bool>,
    /// Whether or not to embed the document outline into the PDF.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generate_document_outline: Option<bool>,
}

/// Parameters for [`PageCommands::page_reload`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReloadParams {
    /// If true, browser cache is ignored (as if the user pressed Shift+refresh).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore_cache: Option<bool>,
    /// If set, the script will be injected into all frames of the inspected page after reload.
    /// Argument will be ignored if reloading dataURL origin.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script_to_evaluate_on_load: Option<String>,
    /// If set, an error will be thrown if the target page's main frame's
    /// loader id does not match the provided id. This prevents accidentally
    /// reloading an unintended target in case there's a racing navigation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loader_id: Option<LoaderId>,
}

/// Parameters for [`PageCommands::page_search_in_resource`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchInResourceParams {
    /// Frame id for resource to search in.
    pub frame_id: FrameId,
    /// URL of the resource to search in.
    pub url: String,
    /// String to search for.
    pub query: String,
    /// If true, search is case sensitive.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub case_sensitive: Option<bool>,
    /// If true, treats string parameter as regex.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_regex: Option<bool>,
}

/// Parameters for [`PageCommands::page_set_font_families`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetFontFamiliesParams {
    /// Specifies font families to set. If a font family is not specified, it won't be changed.
    pub font_families: FontFamilies,
    /// Specifies font families to set for individual scripts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub for_scripts: Option<Vec<ScriptFontFamilies>>,
}

/// Parameters for [`PageCommands::page_start_screencast`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartScreencastParams {
    /// Image compression format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<StartScreencastFormat>,
    /// Compression quality from range [0..100].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<i64>,
    /// Maximum screenshot width.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_width: Option<i64>,
    /// Maximum screenshot height.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_height: Option<i64>,
    /// Send every n-th frame.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub every_nth_frame: Option<i64>,
}

/// Parameters for [`PageCommands::page_set_web_lifecycle_state`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetWebLifecycleStateParams {
    /// Target lifecycle state.
    pub state: SetWebLifecycleStateState,
}

/// Parameters for [`PageCommands::page_set_spc_transaction_mode`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetSPCTransactionModeParams {
    pub mode: SetSPCTransactionModeMode,
}

/// Parameters for [`PageCommands::page_set_rph_registration_mode`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetRPHRegistrationModeParams {
    pub mode: SetRPHRegistrationModeMode,
}

/// Parameters for [`PageCommands::page_generate_test_report`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateTestReportParams {
    /// Message to be displayed in the report.
    pub message: String,
    /// Specifies the endpoint group to deliver the report to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
}

/// Parameters for [`PageCommands::page_set_intercept_file_chooser_dialog`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetInterceptFileChooserDialogParams {
    pub enabled: bool,
    /// If true, cancels the dialog by emitting relevant events (if any)
    /// in addition to not showing it if the interception is enabled
    /// (default: false).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancel: Option<bool>,
}

/// Parameters for [`PageCommands::page_get_annotated_page_content`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetAnnotatedPageContentParams {
    /// Whether to include actionable information. Defaults to true.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_actionable_information: Option<bool>,
}

// ── Return types ─────────────────────────────────────────────────────────────

/// Return type for [`PageCommands::page_add_script_to_evaluate_on_new_document`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddScriptToEvaluateOnNewDocumentReturn {
    /// Identifier of the added script.
    pub identifier: ScriptIdentifier,
}

/// Return type for [`PageCommands::page_capture_screenshot`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureScreenshotReturn {
    /// Base64-encoded image data. (Encoded as a base64 string when passed over JSON)
    pub data: String,
}

/// Return type for [`PageCommands::page_capture_snapshot`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureSnapshotReturn {
    /// Serialized page data.
    pub data: String,
}

/// Return type for [`PageCommands::page_create_isolated_world`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateIsolatedWorldReturn {
    /// Execution context of the isolated world.
    pub execution_context_id: ExecutionContextId,
}

/// Return type for [`PageCommands::page_get_app_manifest`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetAppManifestReturn {
    /// Manifest location.
    pub url: String,
    pub errors: Vec<AppManifestError>,
    /// Manifest content.
    #[serde(default)]
    pub data: Option<String>,
    /// Parsed manifest properties. Deprecated, use manifest instead.
    #[serde(default)]
    pub parsed: Option<AppManifestParsedProperties>,
    pub manifest: WebAppManifest,
}

/// Return type for [`PageCommands::page_get_installability_errors`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetInstallabilityErrorsReturn {
    pub installability_errors: Vec<InstallabilityError>,
}

/// Return type for [`PageCommands::page_get_app_id`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetAppIdReturn {
    /// App id, either from manifest's id attribute or computed from start_url.
    #[serde(default)]
    pub app_id: Option<String>,
    /// Recommendation for manifest's id attribute to match current id computed from start_url.
    #[serde(default)]
    pub recommended_id: Option<String>,
}

/// Return type for [`PageCommands::page_get_ad_script_ancestry`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetAdScriptAncestryReturn {
    /// The ancestry chain of ad script identifiers leading to this frame's
    /// creation, along with the root script's filterlist rule. The ancestry
    /// chain is ordered from the most immediate script (in the frame creation
    /// stack) to more distant ancestors (that created the immediately preceding
    /// script). Only sent if frame is labelled as an ad and ids are available.
    #[serde(default)]
    pub ad_script_ancestry: Option<AdAncestry>,
}

/// Return type for [`PageCommands::page_get_frame_tree`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetFrameTreeReturn {
    /// Present frame tree structure.
    pub frame_tree: FrameTree,
}

/// Return type for [`PageCommands::page_get_layout_metrics`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetLayoutMetricsReturn {
    /// Deprecated metrics relating to the layout viewport. Is in device pixels. Use `cssLayoutViewport` instead.
    pub layout_viewport: LayoutViewport,
    /// Deprecated metrics relating to the visual viewport. Is in device pixels. Use `cssVisualViewport` instead.
    pub visual_viewport: VisualViewport,
    /// Deprecated size of scrollable area. Is in DP. Use `cssContentSize` instead.
    pub content_size: Rect,
    /// Metrics relating to the layout viewport in CSS pixels.
    pub css_layout_viewport: LayoutViewport,
    /// Metrics relating to the visual viewport in CSS pixels.
    pub css_visual_viewport: VisualViewport,
    /// Size of scrollable area in CSS pixels.
    pub css_content_size: Rect,
}

/// Return type for [`PageCommands::page_get_navigation_history`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetNavigationHistoryReturn {
    /// Index of the current navigation history entry.
    pub current_index: i64,
    /// Array of navigation history entries.
    pub entries: Vec<NavigationEntry>,
}

/// Return type for [`PageCommands::page_get_resource_content`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetResourceContentReturn {
    /// Resource content.
    pub content: String,
    /// True, if content was served as base64.
    pub base64_encoded: bool,
}

/// Return type for [`PageCommands::page_get_resource_tree`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetResourceTreeReturn {
    /// Present frame / resource tree structure.
    pub frame_tree: FrameResourceTree,
}

/// Return type for [`PageCommands::page_navigate`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NavigateReturn {
    /// Frame id that has navigated (or failed to navigate)
    pub frame_id: FrameId,
    /// Loader identifier. This is omitted in case of same-document navigation,
    /// as the previously committed loaderId would not change.
    #[serde(default)]
    pub loader_id: Option<LoaderId>,
    /// User friendly error message, present if and only if navigation has failed.
    #[serde(default)]
    pub error_text: Option<String>,
    /// Whether the navigation resulted in a download.
    #[serde(default)]
    pub is_download: Option<bool>,
}

/// Return type for [`PageCommands::page_print_to_pdf`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrintToPDFReturn {
    /// Base64-encoded pdf data. Empty if |returnAsStream| is specified. (Encoded as a base64 string when passed over JSON)
    pub data: String,
    /// A handle of the stream that holds resulting PDF data.
    #[serde(default)]
    pub stream: Option<StreamHandle>,
}

/// Return type for [`PageCommands::page_search_in_resource`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchInResourceReturn {
    /// List of search matches.
    pub result: Vec<SearchMatch>,
}

/// Return type for [`PageCommands::page_get_permissions_policy_state`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPermissionsPolicyStateReturn {
    pub states: Vec<PermissionsPolicyFeatureState>,
}

/// Return type for [`PageCommands::page_get_origin_trials`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetOriginTrialsReturn {
    pub origin_trials: Vec<OriginTrial>,
}

/// Return type for [`PageCommands::page_get_annotated_page_content`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetAnnotatedPageContentReturn {
    /// The annotated page content as a base64 encoded protobuf.
    /// The format is defined by the `AnnotatedPageContent` message in
    /// components/optimization_guide/proto/features/common_quality_data.proto (Encoded as a base64 string when passed over JSON)
    pub content: String,
}

// ── Events ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DomContentEventFiredEvent {
    pub timestamp: MonotonicTime,
}

/// Emitted only when `page.interceptFileChooser` is enabled.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileChooserOpenedEvent {
    /// Id of the frame containing input node.
    pub frame_id: FrameId,
    /// Input mode.
    pub mode: FileChooserOpenedMode,
    /// Input node id. Only present for file choosers opened via an `<input type="file">` element.
    #[serde(default)]
    pub backend_node_id: Option<BackendNodeId>,
}

/// Fired when frame has been attached to its parent.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameAttachedEvent {
    /// Id of the frame that has been attached.
    pub frame_id: FrameId,
    /// Parent frame identifier.
    pub parent_frame_id: FrameId,
    /// JavaScript stack trace of when frame was attached, only set if frame initiated from script.
    #[serde(default)]
    pub stack: Option<StackTrace>,
}

/// Fired when frame has been detached from its parent.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameDetachedEvent {
    /// Id of the frame that has been detached.
    pub frame_id: FrameId,
    pub reason: FrameDetachedReason,
}

/// Fired before frame subtree is detached. Emitted before any frame of the
/// subtree is actually detached.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameSubtreeWillBeDetachedEvent {
    /// Id of the frame that is the root of the subtree that will be detached.
    pub frame_id: FrameId,
}

/// Fired once navigation of the frame has completed. Frame is now associated with the new loader.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameNavigatedEvent {
    /// Frame object.
    pub frame: Frame,
    pub r#type: NavigationType,
}

/// Fired when opening document to write to.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentOpenedEvent {
    /// Frame object.
    pub frame: Frame,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameResizedEvent {}

/// Fired when a navigation starts. This event is fired for both
/// renderer-initiated and browser-initiated navigations. For renderer-initiated
/// navigations, the event is fired after `frameRequestedNavigation`.
/// Navigation may still be cancelled after the event is issued. Multiple events
/// can be fired for a single navigation, for example, when a same-document
/// navigation becomes a cross-document navigation (such as in the case of a
/// frameset).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameStartedNavigatingEvent {
    /// ID of the frame that is being navigated.
    pub frame_id: FrameId,
    /// The URL the navigation started with. The final URL can be different.
    pub url: String,
    /// Loader identifier. Even though it is present in case of same-document
    /// navigation, the previously committed loaderId would not change unless
    /// the navigation changes from a same-document to a cross-document
    /// navigation.
    pub loader_id: LoaderId,
    pub navigation_type: FrameStartedNavigatingNavigationType,
}

/// Fired when a renderer-initiated navigation is requested.
/// Navigation may still be cancelled after the event is issued.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameRequestedNavigationEvent {
    /// Id of the frame that is being navigated.
    pub frame_id: FrameId,
    /// The reason for the navigation.
    pub reason: ClientNavigationReason,
    /// The destination URL for the requested navigation.
    pub url: String,
    /// The disposition for the navigation.
    pub disposition: ClientNavigationDisposition,
}

/// Fired when frame has started loading.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameStartedLoadingEvent {
    /// Id of the frame that has started loading.
    pub frame_id: FrameId,
}

/// Fired when frame has stopped loading.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameStoppedLoadingEvent {
    /// Id of the frame that has stopped loading.
    pub frame_id: FrameId,
}

/// Fired when interstitial page was hidden.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InterstitialHiddenEvent {}

/// Fired when interstitial page was shown.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InterstitialShownEvent {}

/// Fired when a JavaScript initiated dialog (alert, confirm, prompt, or onbeforeunload) has been
/// closed.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavascriptDialogClosedEvent {
    /// Frame id.
    pub frame_id: FrameId,
    /// Whether dialog was confirmed.
    pub result: bool,
    /// User input in case of prompt.
    pub user_input: String,
}

/// Fired when a JavaScript initiated dialog (alert, confirm, prompt, or onbeforeunload) is about to
/// open.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavascriptDialogOpeningEvent {
    /// Frame url.
    pub url: String,
    /// Frame id.
    pub frame_id: FrameId,
    /// Message that will be displayed by the dialog.
    pub message: String,
    /// Dialog type.
    pub r#type: DialogType,
    /// True iff browser is capable showing or acting on the given dialog. When browser has no
    /// dialog handler for given target, calling alert while Page domain is engaged will stall
    /// the page execution. Execution can be resumed via calling Page.handleJavaScriptDialog.
    pub has_browser_handler: bool,
    /// Default dialog prompt.
    #[serde(default)]
    pub default_prompt: Option<String>,
}

/// Fired for lifecycle events (navigation, load, paint, etc) in the current
/// target (including local frames).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LifecycleEventEvent {
    /// Id of the frame.
    pub frame_id: FrameId,
    /// Loader identifier. Empty string if the request is fetched from worker.
    pub loader_id: LoaderId,
    pub name: String,
    pub timestamp: MonotonicTime,
}

/// Fired for failed bfcache history navigations if BackForwardCache feature is enabled. Do
/// not assume any ordering with the Page.frameNavigated event. This event is fired only for
/// main-frame history navigation where the document changes (non-same-document navigations),
/// when bfcache navigation fails.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackForwardCacheNotUsedEvent {
    /// The loader id for the associated navigation.
    pub loader_id: LoaderId,
    /// The frame id of the associated frame.
    pub frame_id: FrameId,
    /// Array of reasons why the page could not be cached. This must not be empty.
    pub not_restored_explanations: Vec<BackForwardCacheNotRestoredExplanation>,
    /// Tree structure of reasons why the page could not be cached for each frame.
    #[serde(default)]
    pub not_restored_explanations_tree: Option<BackForwardCacheNotRestoredExplanationTree>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadEventFiredEvent {
    pub timestamp: MonotonicTime,
}

/// Fired when same-document navigation happens, e.g. due to history API usage or anchor navigation.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NavigatedWithinDocumentEvent {
    /// Id of the frame.
    pub frame_id: FrameId,
    /// Frame's new url.
    pub url: String,
    /// Navigation type.
    pub navigation_type: NavigatedWithinDocumentNavigationType,
}

/// Compressed image data requested by the `startScreencast`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreencastFrameEvent {
    /// Base64-encoded compressed image. (Encoded as a base64 string when passed over JSON)
    pub data: String,
    /// Screencast frame metadata.
    pub metadata: ScreencastFrameMetadata,
    /// Frame number.
    pub session_id: i64,
}

/// Fired when the page with currently enabled screencast was shown or hidden `.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreencastVisibilityChangedEvent {
    /// True if the page is visible.
    pub visible: bool,
}

/// Fired when a new window is going to be opened, via window.open(), link click, form submission,
/// etc.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowOpenEvent {
    /// The URL for the new window.
    pub url: String,
    /// Window name.
    pub window_name: String,
    /// An array of enabled window features.
    pub window_features: Vec<String>,
    /// Whether or not it was triggered by user gesture.
    pub user_gesture: bool,
}

/// Issued for every compilation cache generated.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompilationCacheProducedEvent {
    pub url: String,
    /// Base64-encoded data (Encoded as a base64 string when passed over JSON)
    pub data: String,
}

// ── Domain trait ─────────────────────────────────────────────────────────────

/// `Page` domain CDP methods.
///
/// Actions and events related to the inspected page belong to the page domain.
///
/// Reference: <https://chromedevtools.github.io/devtools-protocol/tot/Page/>
pub trait PageCommands {
    /// Evaluates given script in every frame upon creation (before loading frame's scripts).
    ///
    /// CDP: `Page.addScriptToEvaluateOnNewDocument`
    async fn page_add_script_to_evaluate_on_new_document(
        &self,
        params: &AddScriptToEvaluateOnNewDocumentParams,
    ) -> Result<AddScriptToEvaluateOnNewDocumentReturn>;

    /// Brings page to front (activates tab).
    ///
    /// CDP: `Page.bringToFront`
    async fn page_bring_to_front(&self) -> Result<()>;

    /// Capture page screenshot.
    ///
    /// CDP: `Page.captureScreenshot`
    async fn page_capture_screenshot(
        &self,
        params: &CaptureScreenshotParams,
    ) -> Result<CaptureScreenshotReturn>;

    /// Returns a snapshot of the page as a string. For MHTML format, the serialization includes
    /// iframes, shadow DOM, external resources, and element-inline styles.
    ///
    /// CDP: `Page.captureSnapshot`
    async fn page_capture_snapshot(
        &self,
        params: &CaptureSnapshotParams,
    ) -> Result<CaptureSnapshotReturn>;

    /// Creates an isolated world for the given frame.
    ///
    /// CDP: `Page.createIsolatedWorld`
    async fn page_create_isolated_world(
        &self,
        params: &CreateIsolatedWorldParams,
    ) -> Result<CreateIsolatedWorldReturn>;

    /// Disables page domain notifications.
    ///
    /// CDP: `Page.disable`
    async fn page_disable(&self) -> Result<()>;

    /// Enables page domain notifications.
    ///
    /// CDP: `Page.enable`
    async fn page_enable(&self, params: &EnableParams) -> Result<()>;

    /// Gets the processed manifest for this current document.
    ///   This API always waits for the manifest to be loaded.
    ///   If manifestId is provided, and it does not match the manifest of the
    ///     current document, this API errors out.
    ///   If there is not a loaded page, this API errors out immediately.
    ///
    /// CDP: `Page.getAppManifest`
    async fn page_get_app_manifest(
        &self,
        params: &GetAppManifestParams,
    ) -> Result<GetAppManifestReturn>;

    ///
    /// CDP: `Page.getInstallabilityErrors`
    async fn page_get_installability_errors(&self) -> Result<GetInstallabilityErrorsReturn>;

    /// Returns the unique (PWA) app id.
    /// Only returns values if the feature flag 'WebAppEnableManifestId' is enabled.
    ///
    /// CDP: `Page.getAppId`
    async fn page_get_app_id(&self) -> Result<GetAppIdReturn>;

    ///
    /// CDP: `Page.getAdScriptAncestry`
    async fn page_get_ad_script_ancestry(
        &self,
        frame_id: &FrameId,
    ) -> Result<GetAdScriptAncestryReturn>;

    /// Returns present frame tree structure.
    ///
    /// CDP: `Page.getFrameTree`
    async fn page_get_frame_tree(&self) -> Result<GetFrameTreeReturn>;

    /// Returns metrics relating to the layouting of the page, such as viewport bounds/scale.
    ///
    /// CDP: `Page.getLayoutMetrics`
    async fn page_get_layout_metrics(&self) -> Result<GetLayoutMetricsReturn>;

    /// Returns navigation history for the current page.
    ///
    /// CDP: `Page.getNavigationHistory`
    async fn page_get_navigation_history(&self) -> Result<GetNavigationHistoryReturn>;

    /// Resets navigation history for the current page.
    ///
    /// CDP: `Page.resetNavigationHistory`
    async fn page_reset_navigation_history(&self) -> Result<()>;

    /// Returns content of the given resource.
    ///
    /// CDP: `Page.getResourceContent`
    async fn page_get_resource_content(
        &self,
        frame_id: &FrameId,
        url: &str,
    ) -> Result<GetResourceContentReturn>;

    /// Returns present frame / resource tree structure.
    ///
    /// CDP: `Page.getResourceTree`
    async fn page_get_resource_tree(&self) -> Result<GetResourceTreeReturn>;

    /// Accepts or dismisses a JavaScript initiated dialog (alert, confirm, prompt, or onbeforeunload).
    ///
    /// CDP: `Page.handleJavaScriptDialog`
    async fn page_handle_java_script_dialog(
        &self,
        params: &HandleJavaScriptDialogParams,
    ) -> Result<()>;

    /// Navigates current page to the given URL.
    ///
    /// CDP: `Page.navigate`
    async fn page_navigate(&self, params: &NavigateParams) -> Result<NavigateReturn>;

    /// Navigates current page to the given history entry.
    ///
    /// CDP: `Page.navigateToHistoryEntry`
    async fn page_navigate_to_history_entry(&self, entry_id: i64) -> Result<()>;

    /// Print page as PDF.
    ///
    /// CDP: `Page.printToPDF`
    async fn page_print_to_pdf(&self, params: &PrintToPDFParams) -> Result<PrintToPDFReturn>;

    /// Reloads given page optionally ignoring the cache.
    ///
    /// CDP: `Page.reload`
    async fn page_reload(&self, params: &ReloadParams) -> Result<()>;

    /// Removes given script from the list.
    ///
    /// CDP: `Page.removeScriptToEvaluateOnNewDocument`
    async fn page_remove_script_to_evaluate_on_new_document(
        &self,
        identifier: &ScriptIdentifier,
    ) -> Result<()>;

    /// Acknowledges that a screencast frame has been received by the frontend.
    ///
    /// CDP: `Page.screencastFrameAck`
    async fn page_screencast_frame_ack(&self, session_id: i64) -> Result<()>;

    /// Searches for given string in resource content.
    ///
    /// CDP: `Page.searchInResource`
    async fn page_search_in_resource(
        &self,
        params: &SearchInResourceParams,
    ) -> Result<SearchInResourceReturn>;

    /// Enable Chrome's experimental ad filter on all sites.
    ///
    /// CDP: `Page.setAdBlockingEnabled`
    async fn page_set_ad_blocking_enabled(&self, enabled: bool) -> Result<()>;

    /// Enable page Content Security Policy by-passing.
    ///
    /// CDP: `Page.setBypassCSP`
    async fn page_set_bypass_csp(&self, enabled: bool) -> Result<()>;

    /// Get Permissions Policy state on given frame.
    ///
    /// CDP: `Page.getPermissionsPolicyState`
    async fn page_get_permissions_policy_state(
        &self,
        frame_id: &FrameId,
    ) -> Result<GetPermissionsPolicyStateReturn>;

    /// Get Origin Trials on given frame.
    ///
    /// CDP: `Page.getOriginTrials`
    async fn page_get_origin_trials(&self, frame_id: &FrameId) -> Result<GetOriginTrialsReturn>;

    /// Set generic font families.
    ///
    /// CDP: `Page.setFontFamilies`
    async fn page_set_font_families(&self, params: &SetFontFamiliesParams) -> Result<()>;

    /// Set default font sizes.
    ///
    /// CDP: `Page.setFontSizes`
    async fn page_set_font_sizes(&self, font_sizes: &FontSizes) -> Result<()>;

    /// Sets given markup as the document's HTML.
    ///
    /// CDP: `Page.setDocumentContent`
    async fn page_set_document_content(&self, frame_id: &FrameId, html: &str) -> Result<()>;

    /// Controls whether page will emit lifecycle events.
    ///
    /// CDP: `Page.setLifecycleEventsEnabled`
    async fn page_set_lifecycle_events_enabled(&self, enabled: bool) -> Result<()>;

    /// Starts sending each frame using the `screencastFrame` event.
    ///
    /// CDP: `Page.startScreencast`
    async fn page_start_screencast(&self, params: &StartScreencastParams) -> Result<()>;

    /// Force the page stop all navigations and pending resource fetches.
    ///
    /// CDP: `Page.stopLoading`
    async fn page_stop_loading(&self) -> Result<()>;

    /// Crashes renderer on the IO thread, generates minidumps.
    ///
    /// CDP: `Page.crash`
    async fn page_crash(&self) -> Result<()>;

    /// Tries to close page, running its beforeunload hooks, if any.
    ///
    /// CDP: `Page.close`
    async fn page_close(&self) -> Result<()>;

    /// Tries to update the web lifecycle state of the page.
    /// It will transition the page to the given state according to:
    /// https://github.com/WICG/web-lifecycle/.
    ///
    /// CDP: `Page.setWebLifecycleState`
    async fn page_set_web_lifecycle_state(&self, params: &SetWebLifecycleStateParams)
    -> Result<()>;

    /// Stops sending each frame in the `screencastFrame`.
    ///
    /// CDP: `Page.stopScreencast`
    async fn page_stop_screencast(&self) -> Result<()>;

    /// Requests backend to produce compilation cache for the specified scripts.
    /// `scripts` are appended to the list of scripts for which the cache
    /// would be produced. The list may be reset during page navigation.
    /// When script with a matching URL is encountered, the cache is optionally
    /// produced upon backend discretion, based on internal heuristics.
    /// See also: `Page.compilationCacheProduced`.
    ///
    /// CDP: `Page.produceCompilationCache`
    async fn page_produce_compilation_cache(
        &self,
        scripts: &[CompilationCacheParams],
    ) -> Result<()>;

    /// Seeds compilation cache for given url. Compilation cache does not survive
    /// cross-process navigation.
    ///
    /// CDP: `Page.addCompilationCache`
    async fn page_add_compilation_cache(&self, url: &str, data: &str) -> Result<()>;

    /// Clears seeded compilation cache.
    ///
    /// CDP: `Page.clearCompilationCache`
    async fn page_clear_compilation_cache(&self) -> Result<()>;

    /// Sets the Secure Payment Confirmation transaction mode.
    /// https://w3c.github.io/secure-payment-confirmation/#sctn-automation-set-spc-transaction-mode.
    ///
    /// CDP: `Page.setSPCTransactionMode`
    async fn page_set_spc_transaction_mode(
        &self,
        params: &SetSPCTransactionModeParams,
    ) -> Result<()>;

    /// Extensions for Custom Handlers API:
    /// https://html.spec.whatwg.org/multipage/system-state.html#rph-automation.
    ///
    /// CDP: `Page.setRPHRegistrationMode`
    async fn page_set_rph_registration_mode(
        &self,
        params: &SetRPHRegistrationModeParams,
    ) -> Result<()>;

    /// Generates a report for testing.
    ///
    /// CDP: `Page.generateTestReport`
    async fn page_generate_test_report(&self, params: &GenerateTestReportParams) -> Result<()>;

    /// Pauses page execution. Can be resumed using generic Runtime.runIfWaitingForDebugger.
    ///
    /// CDP: `Page.waitForDebugger`
    async fn page_wait_for_debugger(&self) -> Result<()>;

    /// Intercept file chooser requests and transfer control to protocol clients.
    /// When file chooser interception is enabled, native file chooser dialog is not shown.
    /// Instead, a protocol event `Page.fileChooserOpened` is emitted.
    ///
    /// CDP: `Page.setInterceptFileChooserDialog`
    async fn page_set_intercept_file_chooser_dialog(
        &self,
        params: &SetInterceptFileChooserDialogParams,
    ) -> Result<()>;

    /// Enable/disable prerendering manually.
    ///
    /// This command is a short-term solution for https://crbug.com/1440085.
    /// See https://docs.google.com/document/d/12HVmFxYj5Jc-eJr5OmWsa2bqTJsbgGLKI6ZIyx0_wpA
    /// for more details.
    ///
    /// TODO(https://crbug.com/1440085): Remove this once Puppeteer supports tab targets.
    ///
    /// CDP: `Page.setPrerenderingAllowed`
    async fn page_set_prerendering_allowed(&self, is_allowed: bool) -> Result<()>;

    /// Get the annotated page content for the main frame.
    /// This is an experimental command that is subject to change.
    ///
    /// CDP: `Page.getAnnotatedPageContent`
    async fn page_get_annotated_page_content(
        &self,
        params: &GetAnnotatedPageContentParams,
    ) -> Result<GetAnnotatedPageContentReturn>;
}

// ── Impl ─────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GetAdScriptAncestryInternalParams<'a> {
    frame_id: &'a FrameId,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GetResourceContentInternalParams<'a> {
    frame_id: &'a FrameId,
    url: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct NavigateToHistoryEntryInternalParams {
    entry_id: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RemoveScriptToEvaluateOnNewDocumentInternalParams<'a> {
    identifier: &'a ScriptIdentifier,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ScreencastFrameAckInternalParams {
    session_id: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetAdBlockingEnabledInternalParams {
    enabled: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetBypassCSPInternalParams {
    enabled: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GetPermissionsPolicyStateInternalParams<'a> {
    frame_id: &'a FrameId,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GetOriginTrialsInternalParams<'a> {
    frame_id: &'a FrameId,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetFontSizesInternalParams<'a> {
    font_sizes: &'a FontSizes,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetDocumentContentInternalParams<'a> {
    frame_id: &'a FrameId,
    html: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetLifecycleEventsEnabledInternalParams {
    enabled: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProduceCompilationCacheInternalParams<'a> {
    scripts: &'a [CompilationCacheParams],
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AddCompilationCacheInternalParams<'a> {
    url: &'a str,
    data: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetPrerenderingAllowedInternalParams {
    is_allowed: bool,
}

impl PageCommands for CdpSession {
    async fn page_add_script_to_evaluate_on_new_document(
        &self,
        params: &AddScriptToEvaluateOnNewDocumentParams,
    ) -> Result<AddScriptToEvaluateOnNewDocumentReturn> {
        self.call("Page.addScriptToEvaluateOnNewDocument", params)
            .await
    }

    async fn page_bring_to_front(&self) -> Result<()> {
        self.call_no_response("Page.bringToFront", &serde_json::json!({}))
            .await
    }

    async fn page_capture_screenshot(
        &self,
        params: &CaptureScreenshotParams,
    ) -> Result<CaptureScreenshotReturn> {
        self.call("Page.captureScreenshot", params).await
    }

    async fn page_capture_snapshot(
        &self,
        params: &CaptureSnapshotParams,
    ) -> Result<CaptureSnapshotReturn> {
        self.call("Page.captureSnapshot", params).await
    }

    async fn page_create_isolated_world(
        &self,
        params: &CreateIsolatedWorldParams,
    ) -> Result<CreateIsolatedWorldReturn> {
        self.call("Page.createIsolatedWorld", params).await
    }

    async fn page_disable(&self) -> Result<()> {
        self.call_no_response("Page.disable", &serde_json::json!({}))
            .await
    }

    async fn page_enable(&self, params: &EnableParams) -> Result<()> {
        self.call_no_response("Page.enable", params).await
    }

    async fn page_get_app_manifest(
        &self,
        params: &GetAppManifestParams,
    ) -> Result<GetAppManifestReturn> {
        self.call("Page.getAppManifest", params).await
    }

    async fn page_get_installability_errors(&self) -> Result<GetInstallabilityErrorsReturn> {
        self.call("Page.getInstallabilityErrors", &serde_json::json!({}))
            .await
    }

    async fn page_get_app_id(&self) -> Result<GetAppIdReturn> {
        self.call("Page.getAppId", &serde_json::json!({})).await
    }

    async fn page_get_ad_script_ancestry(
        &self,
        frame_id: &FrameId,
    ) -> Result<GetAdScriptAncestryReturn> {
        let params = GetAdScriptAncestryInternalParams { frame_id };
        self.call("Page.getAdScriptAncestry", &params).await
    }

    async fn page_get_frame_tree(&self) -> Result<GetFrameTreeReturn> {
        self.call("Page.getFrameTree", &serde_json::json!({})).await
    }

    async fn page_get_layout_metrics(&self) -> Result<GetLayoutMetricsReturn> {
        self.call("Page.getLayoutMetrics", &serde_json::json!({}))
            .await
    }

    async fn page_get_navigation_history(&self) -> Result<GetNavigationHistoryReturn> {
        self.call("Page.getNavigationHistory", &serde_json::json!({}))
            .await
    }

    async fn page_reset_navigation_history(&self) -> Result<()> {
        self.call_no_response("Page.resetNavigationHistory", &serde_json::json!({}))
            .await
    }

    async fn page_get_resource_content(
        &self,
        frame_id: &FrameId,
        url: &str,
    ) -> Result<GetResourceContentReturn> {
        let params = GetResourceContentInternalParams { frame_id, url };
        self.call("Page.getResourceContent", &params).await
    }

    async fn page_get_resource_tree(&self) -> Result<GetResourceTreeReturn> {
        self.call("Page.getResourceTree", &serde_json::json!({}))
            .await
    }

    async fn page_handle_java_script_dialog(
        &self,
        params: &HandleJavaScriptDialogParams,
    ) -> Result<()> {
        self.call_no_response("Page.handleJavaScriptDialog", params)
            .await
    }

    async fn page_navigate(&self, params: &NavigateParams) -> Result<NavigateReturn> {
        self.call("Page.navigate", params).await
    }

    async fn page_navigate_to_history_entry(&self, entry_id: i64) -> Result<()> {
        let params = NavigateToHistoryEntryInternalParams { entry_id };
        self.call_no_response("Page.navigateToHistoryEntry", &params)
            .await
    }

    async fn page_print_to_pdf(&self, params: &PrintToPDFParams) -> Result<PrintToPDFReturn> {
        self.call("Page.printToPDF", params).await
    }

    async fn page_reload(&self, params: &ReloadParams) -> Result<()> {
        self.call_no_response("Page.reload", params).await
    }

    async fn page_remove_script_to_evaluate_on_new_document(
        &self,
        identifier: &ScriptIdentifier,
    ) -> Result<()> {
        let params = RemoveScriptToEvaluateOnNewDocumentInternalParams { identifier };
        self.call_no_response("Page.removeScriptToEvaluateOnNewDocument", &params)
            .await
    }

    async fn page_screencast_frame_ack(&self, session_id: i64) -> Result<()> {
        let params = ScreencastFrameAckInternalParams { session_id };
        self.call_no_response("Page.screencastFrameAck", &params)
            .await
    }

    async fn page_search_in_resource(
        &self,
        params: &SearchInResourceParams,
    ) -> Result<SearchInResourceReturn> {
        self.call("Page.searchInResource", params).await
    }

    async fn page_set_ad_blocking_enabled(&self, enabled: bool) -> Result<()> {
        let params = SetAdBlockingEnabledInternalParams { enabled };
        self.call_no_response("Page.setAdBlockingEnabled", &params)
            .await
    }

    async fn page_set_bypass_csp(&self, enabled: bool) -> Result<()> {
        let params = SetBypassCSPInternalParams { enabled };
        self.call_no_response("Page.setBypassCSP", &params).await
    }

    async fn page_get_permissions_policy_state(
        &self,
        frame_id: &FrameId,
    ) -> Result<GetPermissionsPolicyStateReturn> {
        let params = GetPermissionsPolicyStateInternalParams { frame_id };
        self.call("Page.getPermissionsPolicyState", &params).await
    }

    async fn page_get_origin_trials(&self, frame_id: &FrameId) -> Result<GetOriginTrialsReturn> {
        let params = GetOriginTrialsInternalParams { frame_id };
        self.call("Page.getOriginTrials", &params).await
    }

    async fn page_set_font_families(&self, params: &SetFontFamiliesParams) -> Result<()> {
        self.call_no_response("Page.setFontFamilies", params).await
    }

    async fn page_set_font_sizes(&self, font_sizes: &FontSizes) -> Result<()> {
        let params = SetFontSizesInternalParams { font_sizes };
        self.call_no_response("Page.setFontSizes", &params).await
    }

    async fn page_set_document_content(&self, frame_id: &FrameId, html: &str) -> Result<()> {
        let params = SetDocumentContentInternalParams { frame_id, html };
        self.call_no_response("Page.setDocumentContent", &params)
            .await
    }

    async fn page_set_lifecycle_events_enabled(&self, enabled: bool) -> Result<()> {
        let params = SetLifecycleEventsEnabledInternalParams { enabled };
        self.call_no_response("Page.setLifecycleEventsEnabled", &params)
            .await
    }

    async fn page_start_screencast(&self, params: &StartScreencastParams) -> Result<()> {
        self.call_no_response("Page.startScreencast", params).await
    }

    async fn page_stop_loading(&self) -> Result<()> {
        self.call_no_response("Page.stopLoading", &serde_json::json!({}))
            .await
    }

    async fn page_crash(&self) -> Result<()> {
        self.call_no_response("Page.crash", &serde_json::json!({}))
            .await
    }

    async fn page_close(&self) -> Result<()> {
        self.call_no_response("Page.close", &serde_json::json!({}))
            .await
    }

    async fn page_set_web_lifecycle_state(
        &self,
        params: &SetWebLifecycleStateParams,
    ) -> Result<()> {
        self.call_no_response("Page.setWebLifecycleState", params)
            .await
    }

    async fn page_stop_screencast(&self) -> Result<()> {
        self.call_no_response("Page.stopScreencast", &serde_json::json!({}))
            .await
    }

    async fn page_produce_compilation_cache(
        &self,
        scripts: &[CompilationCacheParams],
    ) -> Result<()> {
        let params = ProduceCompilationCacheInternalParams { scripts };
        self.call_no_response("Page.produceCompilationCache", &params)
            .await
    }

    async fn page_add_compilation_cache(&self, url: &str, data: &str) -> Result<()> {
        let params = AddCompilationCacheInternalParams { url, data };
        self.call_no_response("Page.addCompilationCache", &params)
            .await
    }

    async fn page_clear_compilation_cache(&self) -> Result<()> {
        self.call_no_response("Page.clearCompilationCache", &serde_json::json!({}))
            .await
    }

    async fn page_set_spc_transaction_mode(
        &self,
        params: &SetSPCTransactionModeParams,
    ) -> Result<()> {
        self.call_no_response("Page.setSPCTransactionMode", params)
            .await
    }

    async fn page_set_rph_registration_mode(
        &self,
        params: &SetRPHRegistrationModeParams,
    ) -> Result<()> {
        self.call_no_response("Page.setRPHRegistrationMode", params)
            .await
    }

    async fn page_generate_test_report(&self, params: &GenerateTestReportParams) -> Result<()> {
        self.call_no_response("Page.generateTestReport", params)
            .await
    }

    async fn page_wait_for_debugger(&self) -> Result<()> {
        self.call_no_response("Page.waitForDebugger", &serde_json::json!({}))
            .await
    }

    async fn page_set_intercept_file_chooser_dialog(
        &self,
        params: &SetInterceptFileChooserDialogParams,
    ) -> Result<()> {
        self.call_no_response("Page.setInterceptFileChooserDialog", params)
            .await
    }

    async fn page_set_prerendering_allowed(&self, is_allowed: bool) -> Result<()> {
        let params = SetPrerenderingAllowedInternalParams { is_allowed };
        self.call_no_response("Page.setPrerenderingAllowed", &params)
            .await
    }

    async fn page_get_annotated_page_content(
        &self,
        params: &GetAnnotatedPageContentParams,
    ) -> Result<GetAnnotatedPageContentReturn> {
        self.call("Page.getAnnotatedPageContent", params).await
    }
}
