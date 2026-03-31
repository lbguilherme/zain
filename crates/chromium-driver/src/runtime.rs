use std::sync::Arc;

use crate::cdp::runtime::{
    CallArgument, CallFunctionOnParams, EvaluateParams, ExceptionDetails, RemoteObject,
    RuntimeCommands,
};
use crate::error::{CdpError, Result};
use crate::session::CdpSession;

struct JsObjectInner {
    object_id: String,
    cdp: CdpSession,
}

impl Drop for JsObjectInner {
    fn drop(&mut self) {
        let cdp = self.cdp.clone();
        let object_id = self.object_id.clone();
        tokio::spawn(async move {
            let _ = cdp.runtime_release_object(&object_id).await;
        });
    }
}

/// A reference to a JavaScript object in the browser, backed by a CDP `RemoteObject`.
///
/// Shares ownership via `Arc` — the remote object is released (garbage-collectible)
/// when the last `JsObject` clone is dropped.
///
/// Use [`eval`](Self::eval) to call a function with this object as `this`.
#[derive(Clone)]
pub struct JsObject {
    inner: Arc<JsObjectInner>,
    remote: Arc<RemoteObject>,
}

impl JsObject {
    /// Wraps a `RemoteObject` that has an `objectId`.
    ///
    /// Returns `None` if the remote object has no `objectId` (primitives, null, undefined).
    pub fn new(cdp: CdpSession, remote: RemoteObject) -> Option<Self> {
        let object_id = remote.object_id.clone()?;
        Some(Self {
            inner: Arc::new(JsObjectInner { object_id, cdp }),
            remote: Arc::new(remote),
        })
    }

    /// The CDP `objectId` for this remote object.
    pub fn object_id(&self) -> &str {
        &self.inner.object_id
    }

    /// The underlying `RemoteObject` metadata.
    pub fn remote(&self) -> &RemoteObject {
        &self.remote
    }

    /// Calls a JavaScript function with this object as `this`.
    ///
    /// The `function_declaration` should be a function body, e.g.
    /// `"function() { return this.textContent; }"`.
    ///
    /// Returns the result as a new `JsObject` if it has an `objectId`,
    /// or the raw `RemoteObject` via `Err(RemoteObject)` for primitives.
    pub async fn eval(&self, function_declaration: &str) -> Result<EvalResult> {
        let ret = self
            .inner
            .cdp
            .runtime_call_function_on(&CallFunctionOnParams {
                function_declaration: function_declaration.to_owned(),
                object_id: Some(self.inner.object_id.clone()),
                arguments: None,
                return_by_value: None,
                await_promise: None,
            })
            .await?;
        check_exception(ret.exception_details)?;
        Ok(EvalResult::from_remote(self.inner.cdp.clone(), ret.result))
    }

    /// Calls a JavaScript function with this object as `this` and additional arguments.
    pub async fn eval_with(
        &self,
        function_declaration: &str,
        args: Vec<CallArgument>,
    ) -> Result<EvalResult> {
        let ret = self
            .inner
            .cdp
            .runtime_call_function_on(&CallFunctionOnParams {
                function_declaration: function_declaration.to_owned(),
                object_id: Some(self.inner.object_id.clone()),
                arguments: Some(args),
                return_by_value: None,
                await_promise: None,
            })
            .await?;
        check_exception(ret.exception_details)?;
        Ok(EvalResult::from_remote(self.inner.cdp.clone(), ret.result))
    }

    /// Calls a function and returns the result by value (JSON-serialized).
    pub async fn eval_value(&self, function_declaration: &str) -> Result<serde_json::Value> {
        let ret = self
            .inner
            .cdp
            .runtime_call_function_on(&CallFunctionOnParams {
                function_declaration: function_declaration.to_owned(),
                object_id: Some(self.inner.object_id.clone()),
                arguments: None,
                return_by_value: Some(true),
                await_promise: None,
            })
            .await?;
        check_exception(ret.exception_details)?;
        Ok(ret.result.value.unwrap_or(serde_json::Value::Null))
    }
}

/// Result of a JS evaluation — either a managed object or a primitive value.
pub enum EvalResult {
    /// The result is a JS object with a remote reference.
    Object(JsObject),
    /// The result is a primitive (or null/undefined) with no remote reference.
    Value(RemoteObject),
}

impl EvalResult {
    fn from_remote(cdp: CdpSession, remote: RemoteObject) -> Self {
        match JsObject::new(cdp, remote.clone()) {
            Some(obj) => Self::Object(obj),
            None => Self::Value(remote),
        }
    }

    /// Returns the `JsObject` if this is an object result.
    pub fn into_object(self) -> Option<JsObject> {
        match self {
            Self::Object(obj) => Some(obj),
            Self::Value(_) => None,
        }
    }

    /// Returns the underlying `RemoteObject`.
    pub fn remote(&self) -> &RemoteObject {
        match self {
            Self::Object(obj) => obj.remote(),
            Self::Value(r) => r,
        }
    }
}

/// Evaluates a JavaScript expression in the global scope.
///
/// Returns a managed `EvalResult`.
pub async fn evaluate(cdp: &CdpSession, expression: &str) -> Result<EvalResult> {
    let ret = cdp
        .runtime_evaluate(&EvaluateParams {
            expression: expression.to_owned(),
            ..Default::default()
        })
        .await?;
    check_exception(ret.exception_details)?;
    Ok(EvalResult::from_remote(cdp.clone(), ret.result))
}

/// Evaluates a JavaScript expression and returns the result by value.
pub async fn evaluate_value(
    cdp: &CdpSession,
    expression: &str,
) -> Result<serde_json::Value> {
    let ret = cdp
        .runtime_evaluate(&EvaluateParams {
            expression: expression.to_owned(),
            return_by_value: Some(true),
            ..Default::default()
        })
        .await?;
    check_exception(ret.exception_details)?;
    Ok(ret.result.value.unwrap_or(serde_json::Value::Null))
}

fn check_exception(details: Option<ExceptionDetails>) -> Result<()> {
    if let Some(ex) = details {
        let msg = ex
            .exception
            .and_then(|e| e.description)
            .unwrap_or(ex.text);
        return Err(CdpError::JsException(msg));
    }
    Ok(())
}
