use std::sync::Arc;

use crate::cdp::runtime::{
    CallArgument, CallFunctionOnParams, EvaluateParams, ExceptionDetails, ExecutionContextId,
    GetPropertiesParams, GetPropertiesReturn, RemoteObject, RemoteObjectId, RemoteObjectType,
    RuntimeCommands,
};
use crate::error::{CdpError, Result};
use crate::session::CdpSession;

// ── JsObject ───────────────────────────────────────────────────────────────

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
        let object_id = remote.object_id.as_ref()?.0.clone();
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

    /// Returns a `CallArgument` referencing this object, for use in `eval_with`.
    pub fn as_arg(&self) -> CallArgument {
        CallArgument {
            object_id: Some(RemoteObjectId(self.inner.object_id.clone())),
            value: None,
            unserializable_value: None,
        }
    }

    // ── Eval ───────────────────────────────────────────────────────────

    /// Calls a JavaScript function with this object as `this`.
    ///
    /// The `function_declaration` should be a function body, e.g.
    /// `"function() { return this.textContent; }"`.
    pub async fn eval(&self, function_declaration: &str) -> Result<EvalResult> {
        let ret = self
            .call_on(function_declaration, None, false, false)
            .await?;
        Ok(EvalResult::from_remote(self.inner.cdp.clone(), ret))
    }

    /// Calls a JavaScript function with this object as `this` and additional arguments.
    pub async fn eval_with(
        &self,
        function_declaration: &str,
        args: Vec<CallArgument>,
    ) -> Result<EvalResult> {
        let ret = self
            .call_on(function_declaration, Some(args), false, false)
            .await?;
        Ok(EvalResult::from_remote(self.inner.cdp.clone(), ret))
    }

    /// Calls a function and returns the result by value (JSON-serialized).
    pub async fn eval_value(&self, function_declaration: &str) -> Result<serde_json::Value> {
        let ret = self
            .call_on(function_declaration, None, true, false)
            .await?;
        Ok(ret.value.unwrap_or(serde_json::Value::Null))
    }

    /// Calls an async function (or one returning a Promise) with this object as `this`.
    ///
    /// Uses `awaitPromise: true` — waits for the Promise to resolve.
    pub async fn eval_async(&self, function_declaration: &str) -> Result<EvalResult> {
        let ret = self
            .call_on(function_declaration, None, false, true)
            .await?;
        Ok(EvalResult::from_remote(self.inner.cdp.clone(), ret))
    }

    /// Calls an async function and returns the resolved result by value.
    pub async fn eval_value_async(&self, function_declaration: &str) -> Result<serde_json::Value> {
        let ret = self.call_on(function_declaration, None, true, true).await?;
        Ok(ret.value.unwrap_or(serde_json::Value::Null))
    }

    // ── Properties ─────────────────────────────────────────────────────

    /// Returns the own properties of this object.
    pub async fn get_properties(&self) -> Result<GetPropertiesReturn> {
        self.inner
            .cdp
            .runtime_get_properties(&GetPropertiesParams {
                object_id: RemoteObjectId(self.inner.object_id.clone()),
                own_properties: Some(true),
                accessor_properties_only: None,
                generate_preview: None,
                non_indexed_properties_only: None,
            })
            .await
    }

    // ── Internal ───────────────────────────────────────────────────────

    async fn call_on(
        &self,
        function_declaration: &str,
        arguments: Option<Vec<CallArgument>>,
        return_by_value: bool,
        await_promise: bool,
    ) -> Result<RemoteObject> {
        let ret = self
            .inner
            .cdp
            .runtime_call_function_on(&CallFunctionOnParams {
                function_declaration: function_declaration.to_owned(),
                object_id: Some(RemoteObjectId(self.inner.object_id.clone())),
                arguments,
                return_by_value: if return_by_value { Some(true) } else { None },
                await_promise: if await_promise { Some(true) } else { None },
                silent: None,
                generate_preview: None,
                user_gesture: None,
                execution_context_id: None,
                object_group: None,
                throw_on_side_effect: None,
                unique_context_id: None,
                serialization_options: None,
            })
            .await?;
        check_exception(ret.exception_details)?;
        Ok(ret.result)
    }
}

impl std::fmt::Debug for JsObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JsObject")
            .field("object_id", &self.inner.object_id)
            .field("type", &self.remote.object_type)
            .field("subtype", &self.remote.subtype)
            .field("class_name", &self.remote.class_name)
            .field("description", &self.remote.description)
            .finish()
    }
}

// ── EvalResult ─────────────────────────────────────────────────────────────

/// Result of a JS evaluation — either a managed object or a primitive value.
pub enum EvalResult {
    /// The result is a JS object with a remote reference.
    Object(JsObject),
    /// The result is a primitive (or null/undefined) with no remote reference.
    Value(Box<RemoteObject>),
}

impl EvalResult {
    fn from_remote(cdp: CdpSession, remote: RemoteObject) -> Self {
        match JsObject::new(cdp, remote.clone()) {
            Some(obj) => Self::Object(obj),
            None => Self::Value(Box::new(remote)),
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

    /// Extracts the JSON value (for primitives returned by value).
    pub fn into_value(self) -> Option<serde_json::Value> {
        match self {
            Self::Value(r) => r.value,
            Self::Object(_) => None,
        }
    }

    /// Returns true if the result is null or undefined.
    pub fn is_null(&self) -> bool {
        match self {
            Self::Value(r) => {
                r.object_type == RemoteObjectType::Undefined
                    || r.subtype
                        .as_ref()
                        .is_some_and(|s| *s == crate::cdp::runtime::RemoteObjectSubtype::Null)
            }
            Self::Object(_) => false,
        }
    }

    /// Returns the value as a string, if it is one.
    pub fn as_str(&self) -> Option<&str> {
        self.remote().value.as_ref()?.as_str()
    }

    /// Returns the value as f64, if it is one.
    pub fn as_f64(&self) -> Option<f64> {
        self.remote().value.as_ref()?.as_f64()
    }

    /// Returns the value as bool, if it is one.
    pub fn as_bool(&self) -> Option<bool> {
        self.remote().value.as_ref()?.as_bool()
    }
}

// ── Module-level helpers (crate-internal) ──────────────────────────────────

pub(crate) async fn evaluate(cdp: &CdpSession, expression: &str) -> Result<EvalResult> {
    let ret = cdp
        .runtime_evaluate(&EvaluateParams {
            expression: expression.to_owned(),
            ..Default::default()
        })
        .await?;
    check_exception(ret.exception_details)?;
    Ok(EvalResult::from_remote(cdp.clone(), ret.result))
}

pub(crate) async fn evaluate_value(
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

pub(crate) async fn evaluate_value_async(
    cdp: &CdpSession,
    expression: &str,
) -> Result<serde_json::Value> {
    let ret = cdp
        .runtime_evaluate(&EvaluateParams {
            expression: expression.to_owned(),
            return_by_value: Some(true),
            await_promise: Some(true),
            ..Default::default()
        })
        .await?;
    check_exception(ret.exception_details)?;
    Ok(ret.result.value.unwrap_or(serde_json::Value::Null))
}

pub(crate) async fn evaluate_in_context(
    cdp: &CdpSession,
    expression: &str,
    context_id: ExecutionContextId,
) -> Result<EvalResult> {
    let ret = cdp
        .runtime_evaluate(&EvaluateParams {
            expression: expression.to_owned(),
            context_id: Some(context_id),
            ..Default::default()
        })
        .await?;
    check_exception(ret.exception_details)?;
    Ok(EvalResult::from_remote(cdp.clone(), ret.result))
}

pub(crate) async fn evaluate_value_in_context(
    cdp: &CdpSession,
    expression: &str,
    context_id: ExecutionContextId,
) -> Result<serde_json::Value> {
    let ret = cdp
        .runtime_evaluate(&EvaluateParams {
            expression: expression.to_owned(),
            context_id: Some(context_id),
            return_by_value: Some(true),
            ..Default::default()
        })
        .await?;
    check_exception(ret.exception_details)?;
    Ok(ret.result.value.unwrap_or(serde_json::Value::Null))
}

fn check_exception(details: Option<ExceptionDetails>) -> Result<()> {
    if let Some(ex) = details {
        let msg = ex.exception.and_then(|e| e.description).unwrap_or(ex.text);
        return Err(CdpError::JsException(msg));
    }
    Ok(())
}
