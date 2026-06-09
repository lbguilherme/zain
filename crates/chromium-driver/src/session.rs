use std::sync::Arc;

use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio::sync::broadcast;

use crate::error::Result;
use crate::transport::{CdpEvent, Transport};

#[derive(Clone)]
pub struct CdpSession {
    pub(crate) transport: Arc<Transport>,
    pub(crate) session_id: Option<String>,
}

/// Filtered event receiver scoped to a specific CDP session.
///
/// Only delivers events whose `sessionId` matches the session that created it.
/// Browser-level sessions (no `sessionId`) only receive events without a `sessionId`.
pub struct CdpEventStream {
    inner: broadcast::Receiver<CdpEvent>,
    session_id: Option<String>,
}

impl CdpEventStream {
    /// Receives the next event for this session.
    ///
    /// Blocks until a matching event arrives. Returns `None` if the channel is closed.
    pub async fn recv(&mut self) -> Option<CdpEvent> {
        loop {
            match self.inner.recv().await {
                Ok(evt) => {
                    if evt.session_id == self.session_id {
                        return Some(evt);
                    }
                }
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    tracing::warn!(skipped, "CDP event stream lagged; events dropped");
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => return None,
            }
        }
    }

    /// Non-blocking attempt to receive the next event for this session.
    ///
    /// Returns `None` if no matching event is available or the channel is closed.
    pub fn try_recv(&mut self) -> Option<CdpEvent> {
        loop {
            match self.inner.try_recv() {
                Ok(evt) => {
                    if evt.session_id == self.session_id {
                        return Some(evt);
                    }
                }
                Err(broadcast::error::TryRecvError::Lagged(skipped)) => {
                    tracing::warn!(skipped, "CDP event stream lagged; events dropped");
                    continue;
                }
                Err(_) => return None,
            }
        }
    }
}

impl CdpSession {
    pub(crate) fn new(transport: Arc<Transport>) -> Self {
        Self {
            transport,
            session_id: None,
        }
    }

    pub fn for_session(&self, session_id: crate::types::SessionId) -> Self {
        Self {
            transport: self.transport.clone(),
            session_id: Some(session_id.0),
        }
    }

    pub async fn call<P: Serialize, R: DeserializeOwned>(
        &self,
        method: &str,
        params: &P,
    ) -> Result<R> {
        let params_value = serde_json::to_value(params)?;
        let result = self
            .transport
            .send(method, params_value, self.session_id.as_deref())
            .await?;
        Ok(serde_json::from_value(result)?)
    }

    pub async fn call_no_response<P: Serialize>(&self, method: &str, params: &P) -> Result<()> {
        let params_value = serde_json::to_value(params)?;
        self.transport
            .send(method, params_value, self.session_id.as_deref())
            .await?;
        Ok(())
    }

    /// Like [`call`](Self::call) but fails with [`CdpError::Timeout`](crate::CdpError::Timeout)
    /// if no response arrives within `timeout`. Use for commands that may run
    /// long (downloads, captcha waits) or that should fail fast.
    pub async fn call_with_timeout<P: Serialize, R: DeserializeOwned>(
        &self,
        method: &str,
        params: &P,
        timeout: std::time::Duration,
    ) -> Result<R> {
        let params_value = serde_json::to_value(params)?;
        let result = self
            .transport
            .send_with_timeout(method, params_value, self.session_id.as_deref(), timeout)
            .await?;
        Ok(serde_json::from_value(result)?)
    }

    /// Sets the default response timeout for all commands on this connection
    /// (shared across every [`CdpSession`] over the same transport).
    pub fn set_default_timeout(&self, timeout: std::time::Duration) {
        self.transport.set_default_timeout(timeout);
    }

    /// Returns a filtered event stream scoped to this session.
    ///
    /// Events from other sessions are automatically filtered out.
    pub fn events(&self) -> CdpEventStream {
        CdpEventStream {
            inner: self.transport.events(),
            session_id: self.session_id.clone(),
        }
    }
}
