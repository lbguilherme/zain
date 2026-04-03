use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::session::CdpSession;
use crate::types::{Frame, FrameId, MonotonicTime, NavigationEntry};

// ── Types ──────────────────────────────────────────────────────────────────

/// Indicates whether a frame has been identified as an ad.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AdFrameType {
    None,
    Child,
    Root,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdFrameExplanation {
    ParentIsAd,
    CreatedByAdScript,
    MatchedBlockingRule,
}

/// Indicates whether a frame has been identified as an ad and why.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdFrameStatus {
    pub ad_frame_type: AdFrameType,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub explanations: Option<Vec<AdFrameExplanation>>,
}

/// Indicates whether the frame is a secure context and why it is the case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecureContextType {
    Secure,
    SecureLocalhost,
    InsecureScheme,
    InsecureAncestor,
}

/// Indicates whether the frame is cross-origin isolated and why it is the case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CrossOriginIsolatedContextType {
    Isolated,
    NotIsolated,
    NotIsolatedFeatureDisabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GatedAPIFeatures {
    SharedArrayBuffers,
    SharedArrayBuffersTransferAllowed,
    PerformanceMeasureMemory,
    PerformanceProfile,
}

/// All Permissions Policy features.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PermissionsPolicyFeature {
    Accelerometer,
    AllScreensCapture,
    AmbientLightSensor,
    AriaNotify,
    AttributionReporting,
    Autofill,
    Autoplay,
    Bluetooth,
    BrowsingTopics,
    Camera,
    CapturedSurfaceControl,
    ChDpr,
    ChDeviceMemory,
    ChDownlink,
    ChEct,
    ChPrefersColorScheme,
    ChPrefersReducedMotion,
    ChPrefersReducedTransparency,
    ChRtt,
    ChSaveData,
    ChUa,
    ChUaArch,
    ChUaBitness,
    ChUaHighEntropyValues,
    ChUaPlatform,
    ChUaModel,
    ChUaMobile,
    ChUaFormFactors,
    ChUaFullVersion,
    ChUaFullVersionList,
    ChUaPlatformVersion,
    ChUaWow64,
    ChViewportHeight,
    ChViewportWidth,
    ChWidth,
    ClipboardRead,
    ClipboardWrite,
    ComputePressure,
    ControlledFrame,
    CrossOriginIsolated,
    DeferredFetch,
    DeferredFetchMinimal,
    DeviceAttributes,
    DigitalCredentialsCreate,
    DigitalCredentialsGet,
    DirectSockets,
    DirectSocketsMulticast,
    DirectSocketsPrivate,
    DisplayCapture,
    DocumentDomain,
    EncryptedMedia,
    ExecutionWhileOutOfViewport,
    ExecutionWhileNotRendered,
    FencedUnpartitionedStorageRead,
    FocusWithoutUserActivation,
    Fullscreen,
    Frobulate,
    Gamepad,
    Geolocation,
    Gyroscope,
    Hid,
    IdentityCredentialsGet,
    IdleDetection,
    InterestCohort,
    JoinAdInterestGroup,
    KeyboardMap,
    LanguageDetector,
    LanguageModel,
    LocalFonts,
    LocalNetwork,
    LocalNetworkAccess,
    LoopbackNetwork,
    Magnetometer,
    ManualText,
    MediaPlaybackWhileNotVisible,
    Microphone,
    Midi,
    OnDeviceSpeechRecognition,
    OtpCredentials,
    Payment,
    PictureInPicture,
    PrivateAggregation,
    PrivateStateTokenIssuance,
    PrivateStateTokenRedemption,
    PublickeyCredentialsCreate,
    PublickeyCredentialsGet,
    RecordAdAuctionEvents,
    Rewriter,
    RunAdAuction,
    ScreenWakeLock,
    Serial,
    SharedStorage,
    SharedStorageSelectUrl,
    SmartCard,
    SpeakerSelection,
    StorageAccess,
    SubApps,
    Summarizer,
    SyncXhr,
    Translator,
    Unload,
    Usb,
    UsbUnrestricted,
    VerticalScroll,
    WebAppInstallation,
    WebPrinting,
    WebShare,
    WindowManagement,
    Writer,
    XrSpatialTracking,
}

/// Reason for a permissions policy feature to be disabled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionsPolicyBlockReason {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionsPolicyFeatureState {
    pub feature: PermissionsPolicyFeature,
    pub allowed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub locator: Option<PermissionsPolicyBlockLocator>,
}

/// Origin Trial token status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OriginTrialTokenStatus {
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OriginTrialStatus {
    Enabled,
    ValidTokenNotProvided,
    OSNotSupported,
    TrialNotAllowed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OriginTrialUsageRestriction {
    None,
    Subset,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OriginTrialToken {
    pub origin: String,
    pub match_sub_domains: bool,
    pub trial_name: String,
    /// Network.TimeSinceEpoch
    pub expiry_time: f64,
    pub is_third_party: bool,
    pub usage_restriction: OriginTrialUsageRestriction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OriginTrialTokenWithStatus {
    pub raw_token_text: String,
    /// `parsedToken` is present only when the token is extractable and parsable.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub parsed_token: Option<OriginTrialToken>,
    pub status: OriginTrialTokenStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OriginTrial {
    pub trial_name: String,
    pub status: OriginTrialStatus,
    pub tokens_with_status: Vec<OriginTrialTokenWithStatus>,
}

/// Additional information about the frame document's security origin.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityOriginDetails {
    /// Indicates whether the frame document's security origin is one
    /// of the local hostnames (e.g. "localhost") or IP addresses (IPv4
    /// 127.0.0.0/8 or IPv6 ::1).
    pub is_localhost: bool,
}

/// Information about the Resource on the page.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameResource {
    /// Resource URL.
    pub url: String,
    /// Type of this resource (Network.ResourceType).
    #[serde(rename = "type")]
    pub resource_type: String,
    /// Resource mimeType as determined by the browser.
    pub mime_type: String,
    /// last-modified timestamp as reported by server (Network.TimeSinceEpoch).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub last_modified: Option<f64>,
    /// Resource content size.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub content_size: Option<f64>,
    /// True if the resource failed to load.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub failed: Option<bool>,
    /// True if the resource was canceled during loading.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub canceled: Option<bool>,
}

/// Information about the Frame hierarchy along with their cached resources.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameResourceTree {
    /// Frame information for this tree item.
    pub frame: Frame,
    /// Child frames.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub child_frames: Option<Vec<FrameResourceTree>>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub child_frames: Option<Vec<FrameTree>>,
}

/// Unique script identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ScriptIdentifier(pub String);

/// Transition type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransitionType {
    Link,
    Typed,
    AddressBar,
    AutoBookmark,
    AutoSubframe,
    ManualSubframe,
    Generated,
    AutoToplevel,
    FormSubmit,
    Reload,
    Keyword,
    KeywordGenerated,
    Other,
}

/// Screencast frame metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// Frame swap timestamp (Network.TimeSinceEpoch).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub timestamp: Option<f64>,
}

/// Javascript dialog type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DialogType {
    Alert,
    Confirm,
    Prompt,
    Beforeunload,
}

/// Error while paring app manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppManifestParsedProperties {
    /// Computed scope value
    pub scope: String,
}

/// Layout viewport position and dimensions.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub standard: Option<String>,
    /// The fixed font-family.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub fixed: Option<String>,
    /// The serif font-family.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub serif: Option<String>,
    /// The sansSerif font-family.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub sans_serif: Option<String>,
    /// The cursive font-family.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub cursive: Option<String>,
    /// The fantasy font-family.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub fantasy: Option<String>,
    /// The math font-family.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub math: Option<String>,
}

/// Font families collection for a script.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub standard: Option<i64>,
    /// Default fixed font size.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub fixed: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ClientNavigationReason {
    AnchorClick,
    FormSubmissionGet,
    FormSubmissionPost,
    HttpHeaderRefresh,
    InitialFrameNavigation,
    MetaTagRefresh,
    Other,
    PageBlockInterstitial,
    Reload,
    ScriptInitiated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ClientNavigationDisposition {
    CurrentTab,
    NewTab,
    NewWindow,
    Download,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallabilityErrorArgument {
    /// Argument name (e.g. name:'minimum-icon-size-in-pixels').
    pub name: String,
    /// Argument value (e.g. value:'64').
    pub value: String,
}

/// The installability error
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallabilityError {
    /// The error id (e.g. 'manifest-missing-suitable-icon').
    pub error_id: String,
    /// The list of error arguments.
    pub error_arguments: Vec<InstallabilityErrorArgument>,
}

/// The referring-policy used for the navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReferrerPolicy {
    NoReferrer,
    NoReferrerWhenDowngrade,
    Origin,
    OriginWhenCrossOrigin,
    SameOrigin,
    StrictOrigin,
    StrictOriginWhenCrossOrigin,
    UnsafeUrl,
}

/// Per-script compilation cache parameters for `Page.produceCompilationCache`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompilationCacheParams {
    /// The URL of the script to produce a compilation cache entry for.
    pub url: String,
    /// A hint to the backend whether eager compilation is recommended.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub eager: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileFilter {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub accepts: Option<Vec<String>>,
}

/// The image definition used in both icon and screenshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageResource {
    /// The src field in the definition, but changing to url in favor of consistency.
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub sizes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    #[serde(rename = "type")]
    pub resource_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileHandler {
    pub action: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub icons: Option<Vec<ImageResource>>,
    /// Mimic a map, name is the key, accepts is the value.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub accepts: Option<Vec<FileFilter>>,
    pub launch_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchHandler {
    pub client_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolHandler {
    pub protocol: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelatedApplication {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub id: Option<String>,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopeExtension {
    /// Instead of using tuple, this field always returns the serialized string
    /// for easy understanding and comparison.
    pub origin: String,
    pub has_origin_wildcard: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Screenshot {
    pub image: ImageResource,
    pub form_factor: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareTarget {
    pub action: String,
    pub method: String,
    pub enctype: String,
    /// Embed the ShareTargetParams
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub files: Option<Vec<FileFilter>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Shortcut {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebAppManifest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub background_color: Option<String>,
    /// The extra description provided by the manifest.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub display: Option<String>,
    /// The overrided display mode controlled by the user.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub display_overrides: Option<Vec<String>>,
    /// The handlers to open files.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub file_handlers: Option<Vec<FileHandler>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub icons: Option<Vec<ImageResource>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub lang: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub launch_handler: Option<LaunchHandler>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub orientation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub prefer_related_applications: Option<bool>,
    /// The handlers to open protocols.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub protocol_handlers: Option<Vec<ProtocolHandler>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub related_applications: Option<Vec<RelatedApplication>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub scope_extensions: Option<Vec<ScopeExtension>>,
    /// The screenshots used by chromium.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub screenshots: Option<Vec<Screenshot>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub share_target: Option<ShareTarget>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub short_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub shortcuts: Option<Vec<Shortcut>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub start_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub theme_color: Option<String>,
}

/// The type of a frameNavigated event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NavigationType {
    Navigation,
    BackForwardCacheRestore,
}

/// List of not restored reasons for back-forward cache.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackForwardCacheNotRestoredReason {
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
    RequestedByWebViewClient,
    PostMessageByWebViewClient,
    CacheControlNoStoreDeviceBoundSessionTerminated,
    CacheLimitPrunedOnModerateMemoryPressure,
    CacheLimitPrunedOnCriticalMemoryPressure,
}

/// Types of not restored reasons for back-forward cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackForwardCacheNotRestoredReasonType {
    SupportPending,
    PageSupportNeeded,
    Circumstantial,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackForwardCacheBlockingDetails {
    /// Url of the file where blockage happened. Optional because of tests.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub url: Option<String>,
    /// Function name where blockage happened. Optional because of anonymous functions and tests.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub function: Option<String>,
    /// Line number in the script (0-based).
    pub line_number: i64,
    /// Column number in the script (0-based).
    pub column_number: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackForwardCacheNotRestoredExplanation {
    /// Type of the reason
    #[serde(rename = "type")]
    pub reason_type: BackForwardCacheNotRestoredReasonType,
    /// Not restored reason
    pub reason: BackForwardCacheNotRestoredReason,
    /// Context associated with the reason.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub context: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub details: Option<Vec<BackForwardCacheBlockingDetails>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackForwardCacheNotRestoredExplanationTree {
    /// URL of each frame
    pub url: String,
    /// Not restored reasons of each frame
    pub explanations: Vec<BackForwardCacheNotRestoredExplanation>,
    /// Array of children frame
    pub children: Vec<BackForwardCacheNotRestoredExplanationTree>,
}

// ── Param types ────────────────────────────────────────────────────────────

/// Parameters for [`PageCommands::page_add_script_to_evaluate_on_new_document`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddScriptToEvaluateOnNewDocumentParams {
    pub source: String,
    /// If specified, creates an isolated world with the given name and evaluates given script in it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub world_name: Option<String>,
    /// Specifies whether command line API should be available to the script, defaults to false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_command_line_api: Option<bool>,
    /// If true, runs the script immediately on existing execution contexts or worlds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_immediately: Option<bool>,
}

/// Parameters for [`PageCommands::page_capture_screenshot`].
#[derive(Debug, Default, Clone, Serialize)]
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

/// Image compression format for captureScreenshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CaptureScreenshotFormat {
    Jpeg,
    Png,
    Webp,
}

/// Parameters for [`PageCommands::page_capture_snapshot`].
#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureSnapshotParams {
    /// Format (defaults to mhtml).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
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
    /// Whether or not universal access should be granted to the isolated world.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grant_univeral_access: Option<bool>,
}

/// Parameters for [`PageCommands::page_enable`].
#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnableParams {
    /// If true, the `Page.fileChooserOpened` event will be emitted regardless of the
    /// state set by `Page.setInterceptFileChooserDialog` command (default: false).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_file_chooser_opened_event: Option<bool>,
}

/// Parameters for [`PageCommands::page_get_app_manifest`].
#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetAppManifestParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manifest_id: Option<String>,
}

/// Parameters for [`PageCommands::page_get_resource_content`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetResourceContentParams {
    /// Frame id to get resource for.
    pub frame_id: FrameId,
    /// URL of the resource to get content for.
    pub url: String,
}

/// Parameters for [`PageCommands::page_handle_javascript_dialog`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HandleJavaScriptDialogParams {
    /// Whether to accept or dismiss the dialog.
    pub accept: bool,
    /// The text to enter into the dialog prompt before accepting. Used only if this is a prompt dialog.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_text: Option<String>,
}

/// Parameters for [`PageCommands::page_navigate`].
#[derive(Debug, Default, Clone, Serialize)]
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
#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrintToPdfParams {
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
    /// Paper ranges to print, one based, e.g., '1-5, 8, 11-13'.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_ranges: Option<String>,
    /// HTML template for the print header.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header_template: Option<String>,
    /// HTML template for the print footer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub footer_template: Option<String>,
    /// Whether or not to prefer page size as defined by css. Defaults to false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefer_css_page_size: Option<bool>,
    /// return as stream
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transfer_mode: Option<PrintToPdfTransferMode>,
    /// Whether or not to generate tagged (accessible) PDF. Defaults to embedder choice.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generate_tagged_pdf: Option<bool>,
    /// Whether or not to embed the document outline into the PDF.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generate_document_outline: Option<bool>,
}

/// Transfer mode for printToPDF.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrintToPdfTransferMode {
    ReturnAsBase64,
    ReturnAsStream,
}

/// Parameters for [`PageCommands::page_reload`].
#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReloadParams {
    /// If true, browser cache is ignored (as if the user pressed Shift+refresh).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore_cache: Option<bool>,
    /// If set, the script will be injected into all frames of the inspected page after reload.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script_to_evaluate_on_load: Option<String>,
    /// If set, an error will be thrown if the target page's main frame's
    /// loader id does not match the provided id (Network.LoaderId).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loader_id: Option<String>,
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

/// Parameters for [`PageCommands::page_set_document_content`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetDocumentContentParams {
    /// Frame id to set HTML for.
    pub frame_id: FrameId,
    /// HTML content to set.
    pub html: String,
}

/// Parameters for [`PageCommands::page_set_font_families`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetFontFamiliesParams {
    /// Specifies font families to set. If a font family is not specified, it won't be changed.
    pub font_families: FontFamilies,
    /// Specifies font families to set for individual scripts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub for_scripts: Option<Vec<ScriptFontFamilies>>,
}

/// Parameters for [`PageCommands::page_start_screencast`].
#[derive(Debug, Default, Clone, Serialize)]
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

/// Image compression format for startScreencast.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StartScreencastFormat {
    Jpeg,
    Png,
}

/// Web lifecycle state for setWebLifecycleState.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WebLifecycleState {
    Frozen,
    Active,
}

/// SPC transaction mode for setSPCTransactionMode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SpcTransactionMode {
    None,
    AutoAccept,
    AutoChooseToAuthAnotherWay,
    AutoReject,
    AutoOptOut,
}

/// RPH registration mode for setRPHRegistrationMode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RphRegistrationMode {
    None,
    AutoAccept,
    AutoReject,
}

/// Parameters for [`PageCommands::page_set_intercept_file_chooser_dialog`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetInterceptFileChooserDialogParams {
    pub enabled: bool,
    /// If true, cancels the dialog by emitting relevant events (if any)
    /// in addition to not showing it if the interception is enabled (default: false).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancel: Option<bool>,
}

/// Parameters for [`PageCommands::page_generate_test_report`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateTestReportParams {
    /// Message to be displayed in the report.
    pub message: String,
    /// Specifies the endpoint group to deliver the report to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
}

/// Parameters for [`PageCommands::page_get_annotated_page_content`].
#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetAnnotatedPageContentParams {
    /// Whether to include actionable information. Defaults to true.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_actionable_information: Option<bool>,
}

// ── Return types ───────────────────────────────────────────────────────────

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
    /// Base64-encoded image data.
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
    /// Execution context of the isolated world (Runtime.ExecutionContextId).
    pub execution_context_id: i64,
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
    /// App id, either from manifest's id attribute or computed from start_url
    #[serde(default)]
    pub app_id: Option<String>,
    /// Recommendation for manifest's id attribute to match current id computed from start_url
    #[serde(default)]
    pub recommended_id: Option<String>,
}

/// Return type for [`PageCommands::page_get_ad_script_ancestry`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetAdScriptAncestryReturn {
    /// The ancestry chain of ad script identifiers (Network.AdAncestry).
    #[serde(default)]
    pub ad_script_ancestry: Option<serde_json::Value>,
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
    /// Deprecated metrics relating to the layout viewport. Is in device pixels.
    pub layout_viewport: LayoutViewport,
    /// Deprecated metrics relating to the visual viewport. Is in device pixels.
    pub visual_viewport: VisualViewport,
    /// Deprecated size of scrollable area. Is in DP (DOM.Rect).
    pub content_size: serde_json::Value,
    /// Metrics relating to the layout viewport in CSS pixels.
    pub css_layout_viewport: LayoutViewport,
    /// Metrics relating to the visual viewport in CSS pixels.
    pub css_visual_viewport: VisualViewport,
    /// Size of scrollable area in CSS pixels (DOM.Rect).
    pub css_content_size: serde_json::Value,
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
    /// Loader identifier (Network.LoaderId). This is omitted in case of same-document navigation.
    #[serde(default)]
    pub loader_id: Option<String>,
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
pub struct PrintToPdfReturn {
    /// Base64-encoded pdf data. Empty if |returnAsStream| is specified.
    pub data: String,
    /// A handle of the stream that holds resulting PDF data (IO.StreamHandle).
    #[serde(default)]
    pub stream: Option<String>,
}

/// Return type for [`PageCommands::page_search_in_resource`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchInResourceReturn {
    /// List of search matches (Debugger.SearchMatch).
    pub result: Vec<serde_json::Value>,
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
    pub content: String,
}

// ── Events ─────────────────────────────────────────────────────────────────

/// Fired when `DOMContentLoaded` event fires.
///
/// CDP: `Page.domContentEventFired`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DomContentEventFiredEvent {
    pub timestamp: MonotonicTime,
}

/// Emitted only when `page.interceptFileChooser` is enabled.
///
/// CDP: `Page.fileChooserOpened`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileChooserOpenedEvent {
    /// Id of the frame containing input node.
    pub frame_id: FrameId,
    /// Input mode.
    pub mode: FileChooserOpenedMode,
    /// Input node id. Only present for file choosers opened via an `<input type="file">` element (DOM.BackendNodeId).
    #[serde(default)]
    pub backend_node_id: Option<i64>,
}

/// Input mode for fileChooserOpened event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FileChooserOpenedMode {
    SelectSingle,
    SelectMultiple,
}

/// Fired when frame has been attached to its parent.
///
/// CDP: `Page.frameAttached`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameAttachedEvent {
    /// Id of the frame that has been attached.
    pub frame_id: FrameId,
    /// Parent frame identifier.
    pub parent_frame_id: FrameId,
    /// JavaScript stack trace of when frame was attached, only set if frame initiated from script (Runtime.StackTrace).
    #[serde(default)]
    pub stack: Option<serde_json::Value>,
}

/// Fired when frame has been detached from its parent.
///
/// CDP: `Page.frameDetached`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameDetachedEvent {
    /// Id of the frame that has been detached.
    pub frame_id: FrameId,
    pub reason: FrameDetachedReason,
}

/// Reason for frame detachment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FrameDetachedReason {
    Remove,
    Swap,
}

/// Fired before frame subtree is detached. Emitted before any frame of the
/// subtree is actually detached.
///
/// CDP: `Page.frameSubtreeWillBeDetached`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameSubtreeWillBeDetachedEvent {
    /// Id of the frame that is the root of the subtree that will be detached.
    pub frame_id: FrameId,
}

/// Fired once navigation of the frame has completed. Frame is now associated with the new loader.
///
/// CDP: `Page.frameNavigated`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameNavigatedEvent {
    /// Frame object.
    pub frame: Frame,
    #[serde(rename = "type")]
    #[serde(default)]
    pub navigation_type: Option<NavigationType>,
}

/// Fired when opening document to write to.
///
/// CDP: `Page.documentOpened`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentOpenedEvent {
    /// Frame object.
    pub frame: Frame,
}

/// Fired when frame is resized.
///
/// CDP: `Page.frameResized`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameResizedEvent {}

/// Navigation type for frameStartedNavigating event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FrameStartedNavigatingNavigationType {
    Reload,
    ReloadBypassingCache,
    Restore,
    RestoreWithPost,
    HistorySameDocument,
    HistoryDifferentDocument,
    SameDocument,
    DifferentDocument,
}

/// Fired when a navigation starts.
///
/// CDP: `Page.frameStartedNavigating`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameStartedNavigatingEvent {
    /// ID of the frame that is being navigated.
    pub frame_id: FrameId,
    /// The URL the navigation started with. The final URL can be different.
    pub url: String,
    /// Loader identifier (Network.LoaderId).
    pub loader_id: String,
    pub navigation_type: FrameStartedNavigatingNavigationType,
}

/// Fired when a renderer-initiated navigation is requested.
///
/// CDP: `Page.frameRequestedNavigation`
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
///
/// CDP: `Page.frameStartedLoading`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameStartedLoadingEvent {
    /// Id of the frame that has started loading.
    pub frame_id: FrameId,
}

/// Fired when frame has stopped loading.
///
/// CDP: `Page.frameStoppedLoading`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameStoppedLoadingEvent {
    /// Id of the frame that has stopped loading.
    pub frame_id: FrameId,
}

/// Fired when interstitial page was hidden
///
/// CDP: `Page.interstitialHidden`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InterstitialHiddenEvent {}

/// Fired when interstitial page was shown
///
/// CDP: `Page.interstitialShown`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InterstitialShownEvent {}

/// Fired when a JavaScript initiated dialog (alert, confirm, prompt, or onbeforeunload) has been closed.
///
/// CDP: `Page.javascriptDialogClosed`
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

/// Fired when a JavaScript initiated dialog (alert, confirm, prompt, or onbeforeunload) is about to open.
///
/// CDP: `Page.javascriptDialogOpening`
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
    #[serde(rename = "type")]
    pub dialog_type: DialogType,
    /// True iff browser is capable showing or acting on the given dialog.
    pub has_browser_handler: bool,
    /// Default dialog prompt.
    #[serde(default)]
    pub default_prompt: Option<String>,
}

/// Fired for lifecycle events (navigation, load, paint, etc) in the current
/// target (including local frames).
///
/// CDP: `Page.lifecycleEvent`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LifecycleEventEvent {
    /// Id of the frame.
    pub frame_id: FrameId,
    /// Loader identifier (Network.LoaderId). Empty string if the request is fetched from worker.
    pub loader_id: String,
    pub name: String,
    pub timestamp: MonotonicTime,
}

/// Fired for failed bfcache history navigations if BackForwardCache feature is enabled.
///
/// CDP: `Page.backForwardCacheNotUsed`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackForwardCacheNotUsedEvent {
    /// The loader id for the associated navigation (Network.LoaderId).
    pub loader_id: String,
    /// The frame id of the associated frame.
    pub frame_id: FrameId,
    /// Array of reasons why the page could not be cached. This must not be empty.
    pub not_restored_explanations: Vec<BackForwardCacheNotRestoredExplanation>,
    /// Tree structure of reasons why the page could not be cached for each frame.
    #[serde(default)]
    pub not_restored_explanations_tree: Option<BackForwardCacheNotRestoredExplanationTree>,
}

/// Fired when the page's `load` event fires.
///
/// CDP: `Page.loadEventFired`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadEventFiredEvent {
    pub timestamp: MonotonicTime,
}

/// Navigation type for navigatedWithinDocument event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NavigatedWithinDocumentNavigationType {
    Fragment,
    HistoryApi,
    Other,
}

/// Fired when same-document navigation happens, e.g. due to history API usage or anchor navigation.
///
/// CDP: `Page.navigatedWithinDocument`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NavigatedWithinDocumentEvent {
    /// Id of the frame.
    pub frame_id: FrameId,
    /// Frame's new url.
    pub url: String,
    /// Navigation type
    pub navigation_type: NavigatedWithinDocumentNavigationType,
}

/// Compressed image data requested by the `startScreencast`.
///
/// CDP: `Page.screencastFrame`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreencastFrameEvent {
    /// Base64-encoded compressed image.
    pub data: String,
    /// Screencast frame metadata.
    pub metadata: ScreencastFrameMetadata,
    /// Frame number.
    pub session_id: i64,
}

/// Fired when the page with currently enabled screencast was shown or hidden.
///
/// CDP: `Page.screencastVisibilityChanged`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreencastVisibilityChangedEvent {
    /// True if the page is visible.
    pub visible: bool,
}

/// Fired when a new window is going to be opened, via window.open(), link click, form submission, etc.
///
/// CDP: `Page.windowOpen`
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
///
/// CDP: `Page.compilationCacheProduced`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompilationCacheProducedEvent {
    pub url: String,
    /// Base64-encoded data
    pub data: String,
}

// ── Domain trait ────────────────────────────────────────────────────────────

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
    ///
    /// CDP: `Page.getAppManifest`
    async fn page_get_app_manifest(
        &self,
        params: &GetAppManifestParams,
    ) -> Result<GetAppManifestReturn>;

    /// Returns installability errors.
    ///
    /// CDP: `Page.getInstallabilityErrors`
    async fn page_get_installability_errors(&self) -> Result<GetInstallabilityErrorsReturn>;

    /// Returns the unique (PWA) app id.
    ///
    /// CDP: `Page.getAppId`
    async fn page_get_app_id(&self) -> Result<GetAppIdReturn>;

    /// Returns ad script ancestry for a given frame.
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
        params: &GetResourceContentParams,
    ) -> Result<GetResourceContentReturn>;

    /// Returns present frame / resource tree structure.
    ///
    /// CDP: `Page.getResourceTree`
    async fn page_get_resource_tree(&self) -> Result<GetResourceTreeReturn>;

    /// Accepts or dismisses a JavaScript initiated dialog (alert, confirm, prompt, or onbeforeunload).
    ///
    /// CDP: `Page.handleJavaScriptDialog`
    async fn page_handle_javascript_dialog(
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
    async fn page_print_to_pdf(&self, params: &PrintToPdfParams) -> Result<PrintToPdfReturn>;

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
    async fn page_set_document_content(&self, params: &SetDocumentContentParams) -> Result<()>;

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
    ///
    /// CDP: `Page.setWebLifecycleState`
    async fn page_set_web_lifecycle_state(&self, state: WebLifecycleState) -> Result<()>;

    /// Stops sending each frame in the `screencastFrame`.
    ///
    /// CDP: `Page.stopScreencast`
    async fn page_stop_screencast(&self) -> Result<()>;

    /// Requests backend to produce compilation cache for the specified scripts.
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
    ///
    /// CDP: `Page.setSPCTransactionMode`
    async fn page_set_spc_transaction_mode(&self, mode: SpcTransactionMode) -> Result<()>;

    /// Extensions for Custom Handlers API.
    ///
    /// CDP: `Page.setRPHRegistrationMode`
    async fn page_set_rph_registration_mode(&self, mode: RphRegistrationMode) -> Result<()>;

    /// Generates a report for testing.
    ///
    /// CDP: `Page.generateTestReport`
    async fn page_generate_test_report(&self, params: &GenerateTestReportParams) -> Result<()>;

    /// Pauses page execution. Can be resumed using generic Runtime.runIfWaitingForDebugger.
    ///
    /// CDP: `Page.waitForDebugger`
    async fn page_wait_for_debugger(&self) -> Result<()>;

    /// Intercept file chooser requests and transfer control to protocol clients.
    ///
    /// CDP: `Page.setInterceptFileChooserDialog`
    async fn page_set_intercept_file_chooser_dialog(
        &self,
        params: &SetInterceptFileChooserDialogParams,
    ) -> Result<()>;

    /// Enable/disable prerendering manually.
    ///
    /// CDP: `Page.setPrerenderingAllowed`
    async fn page_set_prerendering_allowed(&self, is_allowed: bool) -> Result<()>;

    /// Get the annotated page content for the main frame.
    ///
    /// CDP: `Page.getAnnotatedPageContent`
    async fn page_get_annotated_page_content(
        &self,
        params: &GetAnnotatedPageContentParams,
    ) -> Result<GetAnnotatedPageContentReturn>;
}

// ── Impl ───────────────────────────────────────────────────────────────────

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
struct SetBypassCspInternalParams {
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
struct GetAdScriptAncestryInternalParams<'a> {
    frame_id: &'a FrameId,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetFontSizesInternalParams<'a> {
    font_sizes: &'a FontSizes,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetLifecycleEventsEnabledInternalParams {
    enabled: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetWebLifecycleStateInternalParams {
    state: WebLifecycleState,
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
struct SetSpcTransactionModeInternalParams {
    mode: SpcTransactionMode,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetRphRegistrationModeInternalParams {
    mode: RphRegistrationMode,
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
        params: &GetResourceContentParams,
    ) -> Result<GetResourceContentReturn> {
        self.call("Page.getResourceContent", params).await
    }

    async fn page_get_resource_tree(&self) -> Result<GetResourceTreeReturn> {
        self.call("Page.getResourceTree", &serde_json::json!({}))
            .await
    }

    async fn page_handle_javascript_dialog(
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

    async fn page_print_to_pdf(&self, params: &PrintToPdfParams) -> Result<PrintToPdfReturn> {
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
        let params = SetBypassCspInternalParams { enabled };
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

    async fn page_set_document_content(&self, params: &SetDocumentContentParams) -> Result<()> {
        self.call_no_response("Page.setDocumentContent", params)
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

    async fn page_set_web_lifecycle_state(&self, state: WebLifecycleState) -> Result<()> {
        let params = SetWebLifecycleStateInternalParams { state };
        self.call_no_response("Page.setWebLifecycleState", &params)
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

    async fn page_set_spc_transaction_mode(&self, mode: SpcTransactionMode) -> Result<()> {
        let params = SetSpcTransactionModeInternalParams { mode };
        self.call_no_response("Page.setSPCTransactionMode", &params)
            .await
    }

    async fn page_set_rph_registration_mode(&self, mode: RphRegistrationMode) -> Result<()> {
        let params = SetRphRegistrationModeInternalParams { mode };
        self.call_no_response("Page.setRPHRegistrationMode", &params)
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
