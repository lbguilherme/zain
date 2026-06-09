use serde::{Deserialize, Serialize};

use crate::cdp::browser::BrowserContextId;
use crate::cdp::common::Cookie;
use crate::cdp::common::CookieParam;
use crate::cdp::common::RequestId;
use crate::cdp::common::TimeSinceEpoch;
use crate::cdp::page::FrameId;
use crate::cdp::target::TargetId;
use crate::error::Result;
use crate::session::CdpSession;

// ── Types ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SerializedStorageKey(pub String);

/// Enum of possible storage types.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageType {
    #[default]
    #[serde(rename = "cookies")]
    Cookies,
    #[serde(rename = "file_systems")]
    FileSystems,
    #[serde(rename = "indexeddb")]
    Indexeddb,
    #[serde(rename = "local_storage")]
    LocalStorage,
    #[serde(rename = "shader_cache")]
    ShaderCache,
    #[serde(rename = "websql")]
    Websql,
    #[serde(rename = "service_workers")]
    ServiceWorkers,
    #[serde(rename = "cache_storage")]
    CacheStorage,
    #[serde(rename = "interest_groups")]
    InterestGroups,
    #[serde(rename = "shared_storage")]
    SharedStorage,
    #[serde(rename = "storage_buckets")]
    StorageBuckets,
    #[serde(rename = "all")]
    All,
    #[serde(rename = "other")]
    Other,
}

/// Usage for a storage type.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageForType {
    /// Name of storage type.
    pub storage_type: StorageType,
    /// Storage usage (bytes).
    pub usage: f64,
}

/// Pair of issuer origin and number of available (signed, but not used) Trust
/// Tokens from that issuer.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustTokens {
    pub issuer_origin: String,
    pub count: f64,
}

/// Protected audience interest group auction identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InterestGroupAuctionId(pub String);

/// Enum of interest group access types.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum InterestGroupAccessType {
    #[default]
    #[serde(rename = "join")]
    Join,
    #[serde(rename = "leave")]
    Leave,
    #[serde(rename = "update")]
    Update,
    #[serde(rename = "loaded")]
    Loaded,
    #[serde(rename = "bid")]
    Bid,
    #[serde(rename = "win")]
    Win,
    #[serde(rename = "additionalBid")]
    AdditionalBid,
    #[serde(rename = "additionalBidWin")]
    AdditionalBidWin,
    #[serde(rename = "topLevelBid")]
    TopLevelBid,
    #[serde(rename = "topLevelAdditionalBid")]
    TopLevelAdditionalBid,
    #[serde(rename = "clear")]
    Clear,
}

/// Enum of auction events.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum InterestGroupAuctionEventType {
    #[default]
    #[serde(rename = "started")]
    Started,
    #[serde(rename = "configResolved")]
    ConfigResolved,
}

/// Enum of network fetches auctions can do.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum InterestGroupAuctionFetchType {
    #[default]
    #[serde(rename = "bidderJs")]
    BidderJs,
    #[serde(rename = "bidderWasm")]
    BidderWasm,
    #[serde(rename = "sellerJs")]
    SellerJs,
    #[serde(rename = "bidderTrustedSignals")]
    BidderTrustedSignals,
    #[serde(rename = "sellerTrustedSignals")]
    SellerTrustedSignals,
}

/// Enum of shared storage access scopes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SharedStorageAccessScope {
    #[default]
    #[serde(rename = "window")]
    Window,
    #[serde(rename = "sharedStorageWorklet")]
    SharedStorageWorklet,
    #[serde(rename = "protectedAudienceWorklet")]
    ProtectedAudienceWorklet,
    #[serde(rename = "header")]
    Header,
}

/// Enum of shared storage access methods.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SharedStorageAccessMethod {
    #[default]
    #[serde(rename = "addModule")]
    AddModule,
    #[serde(rename = "createWorklet")]
    CreateWorklet,
    #[serde(rename = "selectURL")]
    SelectURL,
    #[serde(rename = "run")]
    Run,
    #[serde(rename = "batchUpdate")]
    BatchUpdate,
    #[serde(rename = "set")]
    Set,
    #[serde(rename = "append")]
    Append,
    #[serde(rename = "delete")]
    Delete,
    #[serde(rename = "clear")]
    Clear,
    #[serde(rename = "get")]
    Get,
    #[serde(rename = "keys")]
    Keys,
    #[serde(rename = "values")]
    Values,
    #[serde(rename = "entries")]
    Entries,
    #[serde(rename = "length")]
    Length,
    #[serde(rename = "remainingBudget")]
    RemainingBudget,
}

/// Struct for a single key-value pair in an origin's shared storage.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SharedStorageEntry {
    pub key: String,
    pub value: String,
}

/// Details for an origin's shared storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SharedStorageMetadata {
    /// Time when the origin's shared storage was last created.
    pub creation_time: TimeSinceEpoch,
    /// Number of key-value pairs stored in origin's shared storage.
    pub length: i64,
    /// Current amount of bits of entropy remaining in the navigation budget.
    pub remaining_budget: f64,
    /// Total number of bytes stored as key-value pairs in origin's shared
    /// storage.
    pub bytes_used: i64,
}

/// Represents a dictionary object passed in as privateAggregationConfig to
/// run or selectURL.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SharedStoragePrivateAggregationConfig {
    /// The chosen aggregation service deployment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aggregation_coordinator_origin: Option<String>,
    /// The context ID provided.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_id: Option<String>,
    /// Configures the maximum size allowed for filtering IDs.
    pub filtering_id_max_bytes: i64,
    /// The limit on the number of contributions in the final report.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_contributions: Option<i64>,
}

/// Pair of reporting metadata details for a candidate URL for `selectURL()`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SharedStorageReportingMetadata {
    pub event_type: String,
    pub reporting_url: String,
}

/// Bundles a candidate URL with its reporting metadata.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SharedStorageUrlWithMetadata {
    /// Spec of candidate URL.
    pub url: String,
    /// Any associated reporting metadata.
    pub reporting_metadata: Vec<SharedStorageReportingMetadata>,
}

/// Bundles the parameters for shared storage access events whose
/// presence/absence can vary according to SharedStorageAccessType.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SharedStorageAccessParams {
    /// Spec of the module script URL.
    /// Present only for SharedStorageAccessMethods: addModule and
    /// createWorklet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script_source_url: Option<String>,
    /// String denoting "context-origin", "script-origin", or a custom
    /// origin to be used as the worklet's data origin.
    /// Present only for SharedStorageAccessMethod: createWorklet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_origin: Option<String>,
    /// Name of the registered operation to be run.
    /// Present only for SharedStorageAccessMethods: run and selectURL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_name: Option<String>,
    /// ID of the operation call.
    /// Present only for SharedStorageAccessMethods: run and selectURL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
    /// Whether or not to keep the worket alive for future run or selectURL
    /// calls.
    /// Present only for SharedStorageAccessMethods: run and selectURL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keep_alive: Option<bool>,
    /// Configures the private aggregation options.
    /// Present only for SharedStorageAccessMethods: run and selectURL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub private_aggregation_config: Option<SharedStoragePrivateAggregationConfig>,
    /// The operation's serialized data in bytes (converted to a string).
    /// Present only for SharedStorageAccessMethods: run and selectURL.
    /// TODO(crbug.com/401011862): Consider updating this parameter to binary.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub serialized_data: Option<String>,
    /// Array of candidate URLs' specs, along with any associated metadata.
    /// Present only for SharedStorageAccessMethod: selectURL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub urls_with_metadata: Option<Vec<SharedStorageUrlWithMetadata>>,
    /// Spec of the URN:UUID generated for a selectURL call.
    /// Present only for SharedStorageAccessMethod: selectURL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub urn_uuid: Option<String>,
    /// Key for a specific entry in an origin's shared storage.
    /// Present only for SharedStorageAccessMethods: set, append, delete, and
    /// get.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// Value for a specific entry in an origin's shared storage.
    /// Present only for SharedStorageAccessMethods: set and append.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// Whether or not to set an entry for a key if that key is already present.
    /// Present only for SharedStorageAccessMethod: set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ignore_if_present: Option<bool>,
    /// A number denoting the (0-based) order of the worklet's
    /// creation relative to all other shared storage worklets created by
    /// documents using the current storage partition.
    /// Present only for SharedStorageAccessMethods: addModule, createWorklet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worklet_ordinal: Option<i64>,
    /// Hex representation of the DevTools token used as the TargetID for the
    /// associated shared storage worklet.
    /// Present only for SharedStorageAccessMethods: addModule, createWorklet,
    /// run, selectURL, and any other SharedStorageAccessMethod when the
    /// SharedStorageAccessScope is sharedStorageWorklet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worklet_target_id: Option<TargetId>,
    /// Name of the lock to be acquired, if present.
    /// Optionally present only for SharedStorageAccessMethods: batchUpdate,
    /// set, append, delete, and clear.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub with_lock: Option<String>,
    /// If the method has been called as part of a batchUpdate, then this
    /// number identifies the batch to which it belongs.
    /// Optionally present only for SharedStorageAccessMethods:
    /// batchUpdate (required), set, append, delete, and clear.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batch_update_id: Option<String>,
    /// Number of modifier methods sent in batch.
    /// Present only for SharedStorageAccessMethod: batchUpdate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batch_size: Option<i64>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageBucketsDurability {
    #[default]
    #[serde(rename = "relaxed")]
    Relaxed,
    #[serde(rename = "strict")]
    Strict,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageBucket {
    pub storage_key: SerializedStorageKey,
    /// If not specified, it is the default bucket of the storageKey.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageBucketInfo {
    pub bucket: StorageBucket,
    pub id: String,
    pub expiration: TimeSinceEpoch,
    /// Storage quota (bytes).
    pub quota: f64,
    pub persistent: bool,
    pub durability: StorageBucketsDurability,
}

/// A single Related Website Set object.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelatedWebsiteSet {
    /// The primary site of this set, along with the ccTLDs if there is any.
    pub primary_sites: Vec<String>,
    /// The associated sites of this set, along with the ccTLDs if there is any.
    pub associated_sites: Vec<String>,
    /// The service sites of this set, along with the ccTLDs if there is any.
    pub service_sites: Vec<String>,
}

// ── Param types ──────────────────────────────────────────────────────────────

/// Parameters for [`StorageCommands::storage_get_storage_key`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetStorageKeyParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame_id: Option<FrameId>,
}

/// Parameters for [`StorageCommands::storage_get_cookies`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetCookiesParams {
    /// Browser context to use when called on the browser endpoint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser_context_id: Option<BrowserContextId>,
}

/// Parameters for [`StorageCommands::storage_set_cookies`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetCookiesParams {
    /// Cookies to be set.
    pub cookies: Vec<CookieParam>,
    /// Browser context to use when called on the browser endpoint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser_context_id: Option<BrowserContextId>,
}

/// Parameters for [`StorageCommands::storage_clear_cookies`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClearCookiesParams {
    /// Browser context to use when called on the browser endpoint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser_context_id: Option<BrowserContextId>,
}

/// Parameters for [`StorageCommands::storage_override_quota_for_origin`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverrideQuotaForOriginParams {
    /// Security origin.
    pub origin: String,
    /// The quota size (in bytes) to override the original quota with.
    /// If this is called multiple times, the overridden quota will be equal to
    /// the quotaSize provided in the final call. If this is called without
    /// specifying a quotaSize, the quota will be reset to the default value for
    /// the specified origin. If this is called multiple times with different
    /// origins, the override will be maintained for each origin until it is
    /// disabled (called without a quotaSize).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quota_size: Option<f64>,
}

/// Parameters for [`StorageCommands::storage_set_shared_storage_entry`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetSharedStorageEntryParams {
    pub owner_origin: String,
    pub key: String,
    pub value: String,
    /// If `ignoreIfPresent` is included and true, then only sets the entry if
    /// `key` doesn't already exist.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore_if_present: Option<bool>,
}

/// Parameters for [`StorageCommands::storage_set_protected_audience_k_anonymity`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetProtectedAudienceKAnonymityParams {
    pub owner: String,
    pub name: String,
    pub hashes: Vec<String>,
}

// ── Return types ─────────────────────────────────────────────────────────────

/// Return type for [`StorageCommands::storage_get_storage_key`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetStorageKeyReturn {
    pub storage_key: SerializedStorageKey,
}

/// Return type for [`StorageCommands::storage_get_cookies`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetCookiesReturn {
    /// Array of cookie objects.
    pub cookies: Vec<Cookie>,
}

/// Return type for [`StorageCommands::storage_get_usage_and_quota`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetUsageAndQuotaReturn {
    /// Storage usage (bytes).
    pub usage: f64,
    /// Storage quota (bytes).
    pub quota: f64,
    /// Whether or not the origin has an active storage quota override.
    pub override_active: bool,
    /// Storage usage per type (bytes).
    pub usage_breakdown: Vec<UsageForType>,
}

/// Return type for [`StorageCommands::storage_get_trust_tokens`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTrustTokensReturn {
    pub tokens: Vec<TrustTokens>,
}

/// Return type for [`StorageCommands::storage_clear_trust_tokens`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClearTrustTokensReturn {
    /// True if any tokens were deleted, false otherwise.
    pub did_delete_tokens: bool,
}

/// Return type for [`StorageCommands::storage_get_interest_group_details`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetInterestGroupDetailsReturn {
    /// This largely corresponds to:
    /// https://wicg.github.io/turtledove/#dictdef-generatebidinterestgroup
    /// but has absolute expirationTime instead of relative lifetimeMs and
    /// also adds joiningOrigin.
    pub details: serde_json::Value,
}

/// Return type for [`StorageCommands::storage_get_shared_storage_metadata`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSharedStorageMetadataReturn {
    pub metadata: SharedStorageMetadata,
}

/// Return type for [`StorageCommands::storage_get_shared_storage_entries`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSharedStorageEntriesReturn {
    pub entries: Vec<SharedStorageEntry>,
}

/// Return type for [`StorageCommands::storage_run_bounce_tracking_mitigations`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunBounceTrackingMitigationsReturn {
    pub deleted_sites: Vec<String>,
}

/// Return type for [`StorageCommands::storage_get_related_website_sets`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetRelatedWebsiteSetsReturn {
    pub sets: Vec<RelatedWebsiteSet>,
}

// ── Events ───────────────────────────────────────────────────────────────────

/// A cache's contents have been modified.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheStorageContentUpdatedEvent {
    /// Origin to update.
    pub origin: String,
    /// Storage key to update.
    pub storage_key: String,
    /// Storage bucket to update.
    pub bucket_id: String,
    /// Name of cache in origin.
    pub cache_name: String,
}

/// A cache has been added/deleted.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheStorageListUpdatedEvent {
    /// Origin to update.
    pub origin: String,
    /// Storage key to update.
    pub storage_key: String,
    /// Storage bucket to update.
    pub bucket_id: String,
}

/// The origin's IndexedDB object store has been modified.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexedDBContentUpdatedEvent {
    /// Origin to update.
    pub origin: String,
    /// Storage key to update.
    pub storage_key: String,
    /// Storage bucket to update.
    pub bucket_id: String,
    /// Database to update.
    pub database_name: String,
    /// ObjectStore to update.
    pub object_store_name: String,
}

/// The origin's IndexedDB database list has been modified.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexedDBListUpdatedEvent {
    /// Origin to update.
    pub origin: String,
    /// Storage key to update.
    pub storage_key: String,
    /// Storage bucket to update.
    pub bucket_id: String,
}

/// One of the interest groups was accessed. Note that these events are global
/// to all targets sharing an interest group store.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InterestGroupAccessedEvent {
    pub access_time: TimeSinceEpoch,
    pub r#type: InterestGroupAccessType,
    pub owner_origin: String,
    pub name: String,
    /// For topLevelBid/topLevelAdditionalBid, and when appropriate,
    /// win and additionalBidWin.
    #[serde(default)]
    pub component_seller_origin: Option<String>,
    /// For bid or somethingBid event, if done locally and not on a server.
    #[serde(default)]
    pub bid: Option<f64>,
    #[serde(default)]
    pub bid_currency: Option<String>,
    /// For non-global events --- links to interestGroupAuctionEvent.
    #[serde(default)]
    pub unique_auction_id: Option<InterestGroupAuctionId>,
}

/// An auction involving interest groups is taking place. These events are
/// target-specific.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InterestGroupAuctionEventOccurredEvent {
    pub event_time: TimeSinceEpoch,
    pub r#type: InterestGroupAuctionEventType,
    pub unique_auction_id: InterestGroupAuctionId,
    /// Set for child auctions.
    #[serde(default)]
    pub parent_auction_id: Option<InterestGroupAuctionId>,
    /// Set for started and configResolved.
    #[serde(default)]
    pub auction_config: Option<serde_json::Value>,
}

/// Specifies which auctions a particular network fetch may be related to, and
/// in what role. Note that it is not ordered with respect to
/// Network.requestWillBeSent (but will happen before loadingFinished
/// loadingFailed).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InterestGroupAuctionNetworkRequestCreatedEvent {
    pub r#type: InterestGroupAuctionFetchType,
    pub request_id: RequestId,
    /// This is the set of the auctions using the worklet that issued this
    /// request.  In the case of trusted signals, it's possible that only some of
    /// them actually care about the keys being queried.
    pub auctions: Vec<InterestGroupAuctionId>,
}

/// Shared storage was accessed by the associated page.
/// The following parameters are included in all events.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SharedStorageAccessedEvent {
    /// Time of the access.
    pub access_time: TimeSinceEpoch,
    /// Enum value indicating the access scope.
    pub scope: SharedStorageAccessScope,
    /// Enum value indicating the Shared Storage API method invoked.
    pub method: SharedStorageAccessMethod,
    /// DevTools Frame Token for the primary frame tree's root.
    pub main_frame_id: FrameId,
    /// Serialization of the origin owning the Shared Storage data.
    pub owner_origin: String,
    /// Serialization of the site owning the Shared Storage data.
    pub owner_site: String,
    /// The sub-parameters wrapped by `params` are all optional and their
    /// presence/absence depends on `type`.
    pub params: SharedStorageAccessParams,
}

/// A shared storage run or selectURL operation finished its execution.
/// The following parameters are included in all events.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SharedStorageWorkletOperationExecutionFinishedEvent {
    /// Time that the operation finished.
    pub finished_time: TimeSinceEpoch,
    /// Time, in microseconds, from start of shared storage JS API call until
    /// end of operation execution in the worklet.
    pub execution_time: i64,
    /// Enum value indicating the Shared Storage API method invoked.
    pub method: SharedStorageAccessMethod,
    /// ID of the operation call.
    pub operation_id: String,
    /// Hex representation of the DevTools token used as the TargetID for the
    /// associated shared storage worklet.
    pub worklet_target_id: TargetId,
    /// DevTools Frame Token for the primary frame tree's root.
    pub main_frame_id: FrameId,
    /// Serialization of the origin owning the Shared Storage data.
    pub owner_origin: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageBucketCreatedOrUpdatedEvent {
    pub bucket_info: StorageBucketInfo,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageBucketDeletedEvent {
    pub bucket_id: String,
}

// ── Domain trait ─────────────────────────────────────────────────────────────

/// `Storage` domain CDP methods.
///
/// Reference: <https://chromedevtools.github.io/devtools-protocol/tot/Storage/>
pub trait StorageCommands {
    /// Returns storage key for the given frame. If no frame ID is provided,
    /// the storage key of the target executing this command is returned.
    ///
    /// CDP: `Storage.getStorageKey`
    async fn storage_get_storage_key(
        &self,
        params: &GetStorageKeyParams,
    ) -> Result<GetStorageKeyReturn>;

    /// Clears storage for origin.
    ///
    /// CDP: `Storage.clearDataForOrigin`
    async fn storage_clear_data_for_origin(&self, origin: &str, storage_types: &str) -> Result<()>;

    /// Clears storage for storage key.
    ///
    /// CDP: `Storage.clearDataForStorageKey`
    async fn storage_clear_data_for_storage_key(
        &self,
        storage_key: &str,
        storage_types: &str,
    ) -> Result<()>;

    /// Returns all browser cookies.
    ///
    /// CDP: `Storage.getCookies`
    async fn storage_get_cookies(&self, params: &GetCookiesParams) -> Result<GetCookiesReturn>;

    /// Sets given cookies.
    ///
    /// CDP: `Storage.setCookies`
    async fn storage_set_cookies(&self, params: &SetCookiesParams) -> Result<()>;

    /// Clears cookies.
    ///
    /// CDP: `Storage.clearCookies`
    async fn storage_clear_cookies(&self, params: &ClearCookiesParams) -> Result<()>;

    /// Returns usage and quota in bytes.
    ///
    /// CDP: `Storage.getUsageAndQuota`
    async fn storage_get_usage_and_quota(&self, origin: &str) -> Result<GetUsageAndQuotaReturn>;

    /// Override quota for the specified origin.
    ///
    /// CDP: `Storage.overrideQuotaForOrigin`
    async fn storage_override_quota_for_origin(
        &self,
        params: &OverrideQuotaForOriginParams,
    ) -> Result<()>;

    /// Registers origin to be notified when an update occurs to its cache storage list.
    ///
    /// CDP: `Storage.trackCacheStorageForOrigin`
    async fn storage_track_cache_storage_for_origin(&self, origin: &str) -> Result<()>;

    /// Registers storage key to be notified when an update occurs to its cache storage list.
    ///
    /// CDP: `Storage.trackCacheStorageForStorageKey`
    async fn storage_track_cache_storage_for_storage_key(&self, storage_key: &str) -> Result<()>;

    /// Registers origin to be notified when an update occurs to its IndexedDB.
    ///
    /// CDP: `Storage.trackIndexedDBForOrigin`
    async fn storage_track_indexed_db_for_origin(&self, origin: &str) -> Result<()>;

    /// Registers storage key to be notified when an update occurs to its IndexedDB.
    ///
    /// CDP: `Storage.trackIndexedDBForStorageKey`
    async fn storage_track_indexed_db_for_storage_key(&self, storage_key: &str) -> Result<()>;

    /// Unregisters origin from receiving notifications for cache storage.
    ///
    /// CDP: `Storage.untrackCacheStorageForOrigin`
    async fn storage_untrack_cache_storage_for_origin(&self, origin: &str) -> Result<()>;

    /// Unregisters storage key from receiving notifications for cache storage.
    ///
    /// CDP: `Storage.untrackCacheStorageForStorageKey`
    async fn storage_untrack_cache_storage_for_storage_key(&self, storage_key: &str) -> Result<()>;

    /// Unregisters origin from receiving notifications for IndexedDB.
    ///
    /// CDP: `Storage.untrackIndexedDBForOrigin`
    async fn storage_untrack_indexed_db_for_origin(&self, origin: &str) -> Result<()>;

    /// Unregisters storage key from receiving notifications for IndexedDB.
    ///
    /// CDP: `Storage.untrackIndexedDBForStorageKey`
    async fn storage_untrack_indexed_db_for_storage_key(&self, storage_key: &str) -> Result<()>;

    /// Returns the number of stored Trust Tokens per issuer for the
    /// current browsing context.
    ///
    /// CDP: `Storage.getTrustTokens`
    async fn storage_get_trust_tokens(&self) -> Result<GetTrustTokensReturn>;

    /// Removes all Trust Tokens issued by the provided issuerOrigin.
    /// Leaves other stored data, including the issuer's Redemption Records, intact.
    ///
    /// CDP: `Storage.clearTrustTokens`
    async fn storage_clear_trust_tokens(
        &self,
        issuer_origin: &str,
    ) -> Result<ClearTrustTokensReturn>;

    /// Gets details for a named interest group.
    ///
    /// CDP: `Storage.getInterestGroupDetails`
    async fn storage_get_interest_group_details(
        &self,
        owner_origin: &str,
        name: &str,
    ) -> Result<GetInterestGroupDetailsReturn>;

    /// Enables/Disables issuing of interestGroupAccessed events.
    ///
    /// CDP: `Storage.setInterestGroupTracking`
    async fn storage_set_interest_group_tracking(&self, enable: bool) -> Result<()>;

    /// Enables/Disables issuing of interestGroupAuctionEventOccurred and
    /// interestGroupAuctionNetworkRequestCreated.
    ///
    /// CDP: `Storage.setInterestGroupAuctionTracking`
    async fn storage_set_interest_group_auction_tracking(&self, enable: bool) -> Result<()>;

    /// Gets metadata for an origin's shared storage.
    ///
    /// CDP: `Storage.getSharedStorageMetadata`
    async fn storage_get_shared_storage_metadata(
        &self,
        owner_origin: &str,
    ) -> Result<GetSharedStorageMetadataReturn>;

    /// Gets the entries in an given origin's shared storage.
    ///
    /// CDP: `Storage.getSharedStorageEntries`
    async fn storage_get_shared_storage_entries(
        &self,
        owner_origin: &str,
    ) -> Result<GetSharedStorageEntriesReturn>;

    /// Sets entry with `key` and `value` for a given origin's shared storage.
    ///
    /// CDP: `Storage.setSharedStorageEntry`
    async fn storage_set_shared_storage_entry(
        &self,
        params: &SetSharedStorageEntryParams,
    ) -> Result<()>;

    /// Deletes entry for `key` (if it exists) for a given origin's shared storage.
    ///
    /// CDP: `Storage.deleteSharedStorageEntry`
    async fn storage_delete_shared_storage_entry(
        &self,
        owner_origin: &str,
        key: &str,
    ) -> Result<()>;

    /// Clears all entries for a given origin's shared storage.
    ///
    /// CDP: `Storage.clearSharedStorageEntries`
    async fn storage_clear_shared_storage_entries(&self, owner_origin: &str) -> Result<()>;

    /// Resets the budget for `ownerOrigin` by clearing all budget withdrawals.
    ///
    /// CDP: `Storage.resetSharedStorageBudget`
    async fn storage_reset_shared_storage_budget(&self, owner_origin: &str) -> Result<()>;

    /// Enables/disables issuing of sharedStorageAccessed events.
    ///
    /// CDP: `Storage.setSharedStorageTracking`
    async fn storage_set_shared_storage_tracking(&self, enable: bool) -> Result<()>;

    /// Set tracking for a storage key's buckets.
    ///
    /// CDP: `Storage.setStorageBucketTracking`
    async fn storage_set_storage_bucket_tracking(
        &self,
        storage_key: &str,
        enable: bool,
    ) -> Result<()>;

    /// Deletes the Storage Bucket with the given storage key and bucket name.
    ///
    /// CDP: `Storage.deleteStorageBucket`
    async fn storage_delete_storage_bucket(&self, bucket: &StorageBucket) -> Result<()>;

    /// Deletes state for sites identified as potential bounce trackers, immediately.
    ///
    /// CDP: `Storage.runBounceTrackingMitigations`
    async fn storage_run_bounce_tracking_mitigations(
        &self,
    ) -> Result<RunBounceTrackingMitigationsReturn>;

    /// Returns the effective Related Website Sets in use by this profile for the browser
    /// session. The effective Related Website Sets will not change during a browser session.
    ///
    /// CDP: `Storage.getRelatedWebsiteSets`
    async fn storage_get_related_website_sets(&self) -> Result<GetRelatedWebsiteSetsReturn>;

    ///
    /// CDP: `Storage.setProtectedAudienceKAnonymity`
    async fn storage_set_protected_audience_k_anonymity(
        &self,
        params: &SetProtectedAudienceKAnonymityParams,
    ) -> Result<()>;
}

// ── Impl ─────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ClearDataForOriginInternalParams<'a> {
    origin: &'a str,
    storage_types: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ClearDataForStorageKeyInternalParams<'a> {
    storage_key: &'a str,
    storage_types: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GetUsageAndQuotaInternalParams<'a> {
    origin: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TrackCacheStorageForOriginInternalParams<'a> {
    origin: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TrackCacheStorageForStorageKeyInternalParams<'a> {
    storage_key: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TrackIndexedDBForOriginInternalParams<'a> {
    origin: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TrackIndexedDBForStorageKeyInternalParams<'a> {
    storage_key: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UntrackCacheStorageForOriginInternalParams<'a> {
    origin: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UntrackCacheStorageForStorageKeyInternalParams<'a> {
    storage_key: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UntrackIndexedDBForOriginInternalParams<'a> {
    origin: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UntrackIndexedDBForStorageKeyInternalParams<'a> {
    storage_key: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ClearTrustTokensInternalParams<'a> {
    issuer_origin: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GetInterestGroupDetailsInternalParams<'a> {
    owner_origin: &'a str,
    name: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetInterestGroupTrackingInternalParams {
    enable: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetInterestGroupAuctionTrackingInternalParams {
    enable: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GetSharedStorageMetadataInternalParams<'a> {
    owner_origin: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GetSharedStorageEntriesInternalParams<'a> {
    owner_origin: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DeleteSharedStorageEntryInternalParams<'a> {
    owner_origin: &'a str,
    key: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ClearSharedStorageEntriesInternalParams<'a> {
    owner_origin: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ResetSharedStorageBudgetInternalParams<'a> {
    owner_origin: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetSharedStorageTrackingInternalParams {
    enable: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetStorageBucketTrackingInternalParams<'a> {
    storage_key: &'a str,
    enable: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DeleteStorageBucketInternalParams<'a> {
    bucket: &'a StorageBucket,
}

impl StorageCommands for CdpSession {
    async fn storage_get_storage_key(
        &self,
        params: &GetStorageKeyParams,
    ) -> Result<GetStorageKeyReturn> {
        self.call("Storage.getStorageKey", params).await
    }

    async fn storage_clear_data_for_origin(&self, origin: &str, storage_types: &str) -> Result<()> {
        let params = ClearDataForOriginInternalParams {
            origin,
            storage_types,
        };
        self.call_no_response("Storage.clearDataForOrigin", &params)
            .await
    }

    async fn storage_clear_data_for_storage_key(
        &self,
        storage_key: &str,
        storage_types: &str,
    ) -> Result<()> {
        let params = ClearDataForStorageKeyInternalParams {
            storage_key,
            storage_types,
        };
        self.call_no_response("Storage.clearDataForStorageKey", &params)
            .await
    }

    async fn storage_get_cookies(&self, params: &GetCookiesParams) -> Result<GetCookiesReturn> {
        self.call("Storage.getCookies", params).await
    }

    async fn storage_set_cookies(&self, params: &SetCookiesParams) -> Result<()> {
        self.call_no_response("Storage.setCookies", params).await
    }

    async fn storage_clear_cookies(&self, params: &ClearCookiesParams) -> Result<()> {
        self.call_no_response("Storage.clearCookies", params).await
    }

    async fn storage_get_usage_and_quota(&self, origin: &str) -> Result<GetUsageAndQuotaReturn> {
        let params = GetUsageAndQuotaInternalParams { origin };
        self.call("Storage.getUsageAndQuota", &params).await
    }

    async fn storage_override_quota_for_origin(
        &self,
        params: &OverrideQuotaForOriginParams,
    ) -> Result<()> {
        self.call_no_response("Storage.overrideQuotaForOrigin", params)
            .await
    }

    async fn storage_track_cache_storage_for_origin(&self, origin: &str) -> Result<()> {
        let params = TrackCacheStorageForOriginInternalParams { origin };
        self.call_no_response("Storage.trackCacheStorageForOrigin", &params)
            .await
    }

    async fn storage_track_cache_storage_for_storage_key(&self, storage_key: &str) -> Result<()> {
        let params = TrackCacheStorageForStorageKeyInternalParams { storage_key };
        self.call_no_response("Storage.trackCacheStorageForStorageKey", &params)
            .await
    }

    async fn storage_track_indexed_db_for_origin(&self, origin: &str) -> Result<()> {
        let params = TrackIndexedDBForOriginInternalParams { origin };
        self.call_no_response("Storage.trackIndexedDBForOrigin", &params)
            .await
    }

    async fn storage_track_indexed_db_for_storage_key(&self, storage_key: &str) -> Result<()> {
        let params = TrackIndexedDBForStorageKeyInternalParams { storage_key };
        self.call_no_response("Storage.trackIndexedDBForStorageKey", &params)
            .await
    }

    async fn storage_untrack_cache_storage_for_origin(&self, origin: &str) -> Result<()> {
        let params = UntrackCacheStorageForOriginInternalParams { origin };
        self.call_no_response("Storage.untrackCacheStorageForOrigin", &params)
            .await
    }

    async fn storage_untrack_cache_storage_for_storage_key(&self, storage_key: &str) -> Result<()> {
        let params = UntrackCacheStorageForStorageKeyInternalParams { storage_key };
        self.call_no_response("Storage.untrackCacheStorageForStorageKey", &params)
            .await
    }

    async fn storage_untrack_indexed_db_for_origin(&self, origin: &str) -> Result<()> {
        let params = UntrackIndexedDBForOriginInternalParams { origin };
        self.call_no_response("Storage.untrackIndexedDBForOrigin", &params)
            .await
    }

    async fn storage_untrack_indexed_db_for_storage_key(&self, storage_key: &str) -> Result<()> {
        let params = UntrackIndexedDBForStorageKeyInternalParams { storage_key };
        self.call_no_response("Storage.untrackIndexedDBForStorageKey", &params)
            .await
    }

    async fn storage_get_trust_tokens(&self) -> Result<GetTrustTokensReturn> {
        self.call("Storage.getTrustTokens", &serde_json::json!({}))
            .await
    }

    async fn storage_clear_trust_tokens(
        &self,
        issuer_origin: &str,
    ) -> Result<ClearTrustTokensReturn> {
        let params = ClearTrustTokensInternalParams { issuer_origin };
        self.call("Storage.clearTrustTokens", &params).await
    }

    async fn storage_get_interest_group_details(
        &self,
        owner_origin: &str,
        name: &str,
    ) -> Result<GetInterestGroupDetailsReturn> {
        let params = GetInterestGroupDetailsInternalParams { owner_origin, name };
        self.call("Storage.getInterestGroupDetails", &params).await
    }

    async fn storage_set_interest_group_tracking(&self, enable: bool) -> Result<()> {
        let params = SetInterestGroupTrackingInternalParams { enable };
        self.call_no_response("Storage.setInterestGroupTracking", &params)
            .await
    }

    async fn storage_set_interest_group_auction_tracking(&self, enable: bool) -> Result<()> {
        let params = SetInterestGroupAuctionTrackingInternalParams { enable };
        self.call_no_response("Storage.setInterestGroupAuctionTracking", &params)
            .await
    }

    async fn storage_get_shared_storage_metadata(
        &self,
        owner_origin: &str,
    ) -> Result<GetSharedStorageMetadataReturn> {
        let params = GetSharedStorageMetadataInternalParams { owner_origin };
        self.call("Storage.getSharedStorageMetadata", &params).await
    }

    async fn storage_get_shared_storage_entries(
        &self,
        owner_origin: &str,
    ) -> Result<GetSharedStorageEntriesReturn> {
        let params = GetSharedStorageEntriesInternalParams { owner_origin };
        self.call("Storage.getSharedStorageEntries", &params).await
    }

    async fn storage_set_shared_storage_entry(
        &self,
        params: &SetSharedStorageEntryParams,
    ) -> Result<()> {
        self.call_no_response("Storage.setSharedStorageEntry", params)
            .await
    }

    async fn storage_delete_shared_storage_entry(
        &self,
        owner_origin: &str,
        key: &str,
    ) -> Result<()> {
        let params = DeleteSharedStorageEntryInternalParams { owner_origin, key };
        self.call_no_response("Storage.deleteSharedStorageEntry", &params)
            .await
    }

    async fn storage_clear_shared_storage_entries(&self, owner_origin: &str) -> Result<()> {
        let params = ClearSharedStorageEntriesInternalParams { owner_origin };
        self.call_no_response("Storage.clearSharedStorageEntries", &params)
            .await
    }

    async fn storage_reset_shared_storage_budget(&self, owner_origin: &str) -> Result<()> {
        let params = ResetSharedStorageBudgetInternalParams { owner_origin };
        self.call_no_response("Storage.resetSharedStorageBudget", &params)
            .await
    }

    async fn storage_set_shared_storage_tracking(&self, enable: bool) -> Result<()> {
        let params = SetSharedStorageTrackingInternalParams { enable };
        self.call_no_response("Storage.setSharedStorageTracking", &params)
            .await
    }

    async fn storage_set_storage_bucket_tracking(
        &self,
        storage_key: &str,
        enable: bool,
    ) -> Result<()> {
        let params = SetStorageBucketTrackingInternalParams {
            storage_key,
            enable,
        };
        self.call_no_response("Storage.setStorageBucketTracking", &params)
            .await
    }

    async fn storage_delete_storage_bucket(&self, bucket: &StorageBucket) -> Result<()> {
        let params = DeleteStorageBucketInternalParams { bucket };
        self.call_no_response("Storage.deleteStorageBucket", &params)
            .await
    }

    async fn storage_run_bounce_tracking_mitigations(
        &self,
    ) -> Result<RunBounceTrackingMitigationsReturn> {
        self.call(
            "Storage.runBounceTrackingMitigations",
            &serde_json::json!({}),
        )
        .await
    }

    async fn storage_get_related_website_sets(&self) -> Result<GetRelatedWebsiteSetsReturn> {
        self.call("Storage.getRelatedWebsiteSets", &serde_json::json!({}))
            .await
    }

    async fn storage_set_protected_audience_k_anonymity(
        &self,
        params: &SetProtectedAudienceKAnonymityParams,
    ) -> Result<()> {
        self.call_no_response("Storage.setProtectedAudienceKAnonymity", params)
            .await
    }
}
