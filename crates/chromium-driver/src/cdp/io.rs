use serde::{Deserialize, Serialize};

use crate::cdp::runtime::RemoteObjectId;
use crate::error::Result;
use crate::session::CdpSession;

// ── Types ───────────────────────────────────────────────────────────────────

/// This is either obtained from another method or specified as `blob:<uuid>` where
/// `<uuid>` is an UUID of a Blob.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StreamHandle(pub String);

// ── Param types ─────────────────────────────────────────────────────────────

/// Parameters for [`IoCommands::io_read`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadParams {
    /// Handle of the stream to read.
    pub handle: StreamHandle,
    /// Seek to the specified offset before reading (if not specified, proceed with offset
    /// following the last read). Some types of streams may only support sequential reads.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<i64>,
    /// Maximum number of bytes to read (left upon the agent discretion if not specified).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<i64>,
}

// ── Return types ────────────────────────────────────────────────────────────

/// Return type for [`IoCommands::io_read`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadReturn {
    /// Set if the data is base64-encoded.
    #[serde(default)]
    pub base64_encoded: Option<bool>,
    /// Data that were read.
    pub data: String,
    /// Set if the end-of-file condition occurred while reading.
    pub eof: bool,
}

/// Return type for [`IoCommands::io_resolve_blob`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveBlobReturn {
    /// UUID of the specified Blob.
    pub uuid: String,
}

// ── Domain trait ─────────────────────────────────────────────────────────────

/// `IO` domain CDP methods.
///
/// Input/Output operations for streams produced by DevTools.
///
/// Reference: <https://chromedevtools.github.io/devtools-protocol/tot/IO/>
pub trait IoCommands {
    /// Close the stream, discard any temporary backing storage.
    ///
    /// CDP: `IO.close`
    async fn io_close(&self, handle: &StreamHandle) -> Result<()>;

    /// Read a chunk of the stream.
    ///
    /// CDP: `IO.read`
    async fn io_read(&self, params: &ReadParams) -> Result<ReadReturn>;

    /// Return UUID of Blob object specified by a remote object id.
    ///
    /// CDP: `IO.resolveBlob`
    async fn io_resolve_blob(&self, object_id: &RemoteObjectId) -> Result<ResolveBlobReturn>;
}

// ── Impl ────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CloseInternalParams<'a> {
    handle: &'a StreamHandle,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ResolveBlobInternalParams<'a> {
    object_id: &'a RemoteObjectId,
}

impl IoCommands for CdpSession {
    async fn io_close(&self, handle: &StreamHandle) -> Result<()> {
        let params = CloseInternalParams { handle };
        self.call_no_response("IO.close", &params).await
    }

    async fn io_read(&self, params: &ReadParams) -> Result<ReadReturn> {
        self.call("IO.read", params).await
    }

    async fn io_resolve_blob(&self, object_id: &RemoteObjectId) -> Result<ResolveBlobReturn> {
        let params = ResolveBlobInternalParams { object_id };
        self.call("IO.resolveBlob", &params).await
    }
}
