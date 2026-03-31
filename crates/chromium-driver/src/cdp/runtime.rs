use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::session::CdpSession;

// --- Core types ---

/// Mirror of CDP `Runtime.RemoteObject`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteObject {
    /// Object type: "object", "function", "undefined", "string", "number", "boolean", "symbol", "bigint".
    #[serde(rename = "type")]
    pub object_type: String,
    /// Object subtype hint: "array", "null", "node", "regexp", "date", "map", "set", "promise", etc.
    #[serde(default)]
    pub subtype: Option<String>,
    /// Object class (constructor) name.
    #[serde(default)]
    pub class_name: Option<String>,
    /// Primitive value or JSON-serializable object (if `returnByValue` was set).
    #[serde(default)]
    pub value: Option<serde_json::Value>,
    /// String representation of the object.
    #[serde(default)]
    pub description: Option<String>,
    /// Unique object identifier (for non-primitive values).
    #[serde(default)]
    pub object_id: Option<String>,
}

/// Exception details returned by Runtime.evaluate on error.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExceptionDetails {
    pub exception_id: i64,
    pub text: String,
    pub line_number: i64,
    pub column_number: i64,
    #[serde(default)]
    pub exception: Option<RemoteObject>,
}

// --- Param types ---

/// Parameters for `Runtime.evaluate`.
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluateParams {
    pub expression: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_group: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_by_value: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub await_promise: Option<bool>,
}

/// Parameters for `Runtime.callFunctionOn`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CallFunctionOnParams {
    pub function_declaration: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<CallArgument>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_by_value: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub await_promise: Option<bool>,
}

/// Argument for `Runtime.callFunctionOn`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CallArgument {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_id: Option<String>,
}

// --- Return types ---

/// Return type for `Runtime.evaluate` and `Runtime.callFunctionOn`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluateReturn {
    pub result: RemoteObject,
    #[serde(default)]
    pub exception_details: Option<ExceptionDetails>,
}

// --- Domain trait ---

/// `Runtime` domain CDP methods.
///
/// Reference: <https://chromedevtools.github.io/devtools-protocol/tot/Runtime/>
pub trait RuntimeCommands {
    /// Evaluates a JavaScript expression in the global scope.
    ///
    /// CDP: `Runtime.evaluate`
    async fn runtime_evaluate(&self, params: &EvaluateParams) -> Result<EvaluateReturn>;

    /// Calls a function with a given `this` object (identified by `objectId`).
    ///
    /// CDP: `Runtime.callFunctionOn`
    async fn runtime_call_function_on(
        &self,
        params: &CallFunctionOnParams,
    ) -> Result<EvaluateReturn>;

    /// Releases a remote object by its ID, allowing it to be garbage-collected.
    ///
    /// CDP: `Runtime.releaseObject`
    async fn runtime_release_object(&self, object_id: &str) -> Result<()>;

    /// Releases all remote objects in the given group.
    ///
    /// CDP: `Runtime.releaseObjectGroup`
    async fn runtime_release_object_group(&self, object_group: &str) -> Result<()>;
}

impl RuntimeCommands for CdpSession {
    async fn runtime_evaluate(&self, params: &EvaluateParams) -> Result<EvaluateReturn> {
        self.call("Runtime.evaluate", params).await
    }

    async fn runtime_call_function_on(
        &self,
        params: &CallFunctionOnParams,
    ) -> Result<EvaluateReturn> {
        self.call("Runtime.callFunctionOn", params).await
    }

    async fn runtime_release_object(&self, object_id: &str) -> Result<()> {
        self.call_no_response(
            "Runtime.releaseObject",
            &serde_json::json!({"objectId": object_id}),
        )
        .await
    }

    async fn runtime_release_object_group(&self, object_group: &str) -> Result<()> {
        self.call_no_response(
            "Runtime.releaseObjectGroup",
            &serde_json::json!({"objectGroup": object_group}),
        )
        .await
    }
}
