use std::sync::Arc;

use serde::de::DeserializeOwned;
use serde::Serialize;
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
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
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
                Err(broadcast::error::TryRecvError::Lagged(_)) => continue,
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
