use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::session::CdpSession;

// ── Types ────────────────────────────────────────────────────────────────────

/// Unique script identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ScriptId(pub String);

/// Represents options for serialization. Overrides `generatePreview` and `returnByValue`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializationOptions {
    pub serialization: SerializationOptionsSerialization,
    /// Deep serialization depth. Default is full depth. Respected only in `deep` serialization mode.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_depth: Option<i64>,
    /// Embedder-specific parameters. For example if connected to V8 in Chrome these control DOM
    /// serialization via `maxNodeDepth: integer` and `includeShadowTree: "none" | "open" | "all"`.
    /// Values can be only of type string or integer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub additional_parameters: Option<serde_json::Value>,
}

/// Represents deep serialized value.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeepSerializedValue {
    pub r#type: DeepSerializedValueType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object_id: Option<String>,
    /// Set if value reference met more then once during serialization. In such
    /// case, value is provided only to one of the serialized values. Unique
    /// per value in the scope of one CDP call.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weak_local_object_reference: Option<i64>,
}

/// Unique object identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RemoteObjectId(pub String);

/// Primitive value which cannot be JSON-stringified. Includes values `-0`, `NaN`, `Infinity`,
/// `-Infinity`, and bigint literals.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UnserializableValue(pub String);

/// Mirror object referencing original JavaScript object.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteObject {
    /// Object type.
    pub r#type: RemoteObjectType,
    /// Object subtype hint. Specified for `object` type values only.
    /// NOTE: If you change anything here, make sure to also update
    /// `subtype` in `ObjectPreview` and `PropertyPreview` below.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtype: Option<RemoteObjectSubtype>,
    /// Object class (constructor) name. Specified for `object` type values only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class_name: Option<String>,
    /// Remote object value in case of primitive values or JSON values (if it was requested).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
    /// Primitive value which can not be JSON-stringified does not have `value`, but gets this
    /// property.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unserializable_value: Option<UnserializableValue>,
    /// String representation of the object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Deep serialized value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deep_serialized_value: Option<DeepSerializedValue>,
    /// Unique object identifier (for non-primitive values).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object_id: Option<RemoteObjectId>,
    /// Preview containing abbreviated property values. Specified for `object` type values only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview: Option<ObjectPreview>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_preview: Option<CustomPreview>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomPreview {
    /// The JSON-stringified result of formatter.header(object, config) call.
    /// It contains json ML array that represents RemoteObject.
    pub header: String,
    /// If formatter returns true as a result of formatter.hasBody call then bodyGetterId will
    /// contain RemoteObjectId for the function that returns result of formatter.body(object, config) call.
    /// The result value is json ML array.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_getter_id: Option<RemoteObjectId>,
}

/// Object containing abbreviated remote object value.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectPreview {
    /// Object type.
    pub r#type: ObjectPreviewType,
    /// Object subtype hint. Specified for `object` type values only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtype: Option<ObjectPreviewSubtype>,
    /// String representation of the object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// True iff some of the properties or entries of the original object did not fit.
    pub overflow: bool,
    /// List of the properties.
    pub properties: Vec<PropertyPreview>,
    /// List of the entries. Specified for `map` and `set` subtype values only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entries: Option<Vec<EntryPreview>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PropertyPreview {
    /// Property name.
    pub name: String,
    /// Object type. Accessor means that the property itself is an accessor property.
    pub r#type: PropertyPreviewType,
    /// User-friendly property value string.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// Nested value preview.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_preview: Option<ObjectPreview>,
    /// Object subtype hint. Specified for `object` type values only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtype: Option<PropertyPreviewSubtype>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryPreview {
    /// Preview of the key. Specified for map-like collection entries.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<ObjectPreview>,
    /// Preview of the value.
    pub value: ObjectPreview,
}

/// Object property descriptor.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PropertyDescriptor {
    /// Property name or symbol description.
    pub name: String,
    /// The value associated with the property.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<RemoteObject>,
    /// True if the value associated with the property may be changed (data descriptors only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub writable: Option<bool>,
    /// A function which serves as a getter for the property, or `undefined` if there is no getter
    /// (accessor descriptors only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub get: Option<RemoteObject>,
    /// A function which serves as a setter for the property, or `undefined` if there is no setter
    /// (accessor descriptors only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub set: Option<RemoteObject>,
    /// True if the type of this property descriptor may be changed and if the property may be
    /// deleted from the corresponding object.
    pub configurable: bool,
    /// True if this property shows up during enumeration of the properties on the corresponding
    /// object.
    pub enumerable: bool,
    /// True if the result was thrown during the evaluation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub was_thrown: Option<bool>,
    /// True if the property is owned for the object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_own: Option<bool>,
    /// Property symbol object, if the property is of the `symbol` type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol: Option<RemoteObject>,
}

/// Object internal property descriptor. This property isn't normally visible in JavaScript code.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InternalPropertyDescriptor {
    /// Conventional property name.
    pub name: String,
    /// The value associated with the property.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<RemoteObject>,
}

/// Object private field descriptor.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivatePropertyDescriptor {
    /// Private property name.
    pub name: String,
    /// The value associated with the private property.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<RemoteObject>,
    /// A function which serves as a getter for the private property,
    /// or `undefined` if there is no getter (accessor descriptors only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub get: Option<RemoteObject>,
    /// A function which serves as a setter for the private property,
    /// or `undefined` if there is no setter (accessor descriptors only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub set: Option<RemoteObject>,
}

/// Represents function call argument. Either remote object id `objectId`, primitive `value`,
/// unserializable primitive value or neither of (for undefined) them should be specified.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallArgument {
    /// Primitive value or serializable javascript object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
    /// Primitive value which can not be JSON-stringified.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unserializable_value: Option<UnserializableValue>,
    /// Remote object handle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object_id: Option<RemoteObjectId>,
}

/// Id of an execution context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExecutionContextId(pub i64);

/// Description of an isolated world.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionContextDescription {
    /// Unique id of the execution context. It can be used to specify in which execution context
    /// script evaluation should be performed.
    pub id: ExecutionContextId,
    /// Execution context origin.
    pub origin: String,
    /// Human readable name describing given context.
    pub name: String,
    /// A system-unique execution context identifier. Unlike the id, this is unique across
    /// multiple processes, so can be reliably used to identify specific context while backend
    /// performs a cross-process navigation.
    pub unique_id: String,
    /// Embedder-specific auxiliary data likely matching {isDefault: boolean, type: 'default'|'isolated'|'worker', frameId: string}.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aux_data: Option<serde_json::Value>,
}

/// Detailed information about exception (or error) that was thrown during script compilation or
/// execution.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExceptionDetails {
    /// Exception id.
    pub exception_id: i64,
    /// Exception text, which should be used together with exception object when available.
    pub text: String,
    /// Line number of the exception location (0-based).
    pub line_number: i64,
    /// Column number of the exception location (0-based).
    pub column_number: i64,
    /// Script ID of the exception location.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script_id: Option<ScriptId>,
    /// URL of the exception location, to be used when the script was not reported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// JavaScript stack trace if available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stack_trace: Option<StackTrace>,
    /// Exception object if available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exception: Option<RemoteObject>,
    /// Identifier of the context where exception happened.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_context_id: Option<ExecutionContextId>,
    /// Dictionary with entries of meta data that the client associated
    /// with this exception, such as information about associated network
    /// requests, etc.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exception_meta_data: Option<serde_json::Value>,
}

/// Number of milliseconds since epoch.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Timestamp(pub f64);

/// Number of milliseconds.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TimeDelta(pub f64);

/// Stack entry for runtime errors and assertions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallFrame {
    /// JavaScript function name.
    pub function_name: String,
    /// JavaScript script id.
    pub script_id: ScriptId,
    /// JavaScript script name or url.
    pub url: String,
    /// JavaScript script line number (0-based).
    pub line_number: i64,
    /// JavaScript script column number (0-based).
    pub column_number: i64,
}

/// Call frames for assertions or error messages.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackTrace {
    /// String label of this stack trace. For async traces this may be a name of the function that
    /// initiated the async call.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// JavaScript function name.
    pub call_frames: Vec<CallFrame>,
    /// Asynchronous JavaScript stack trace that preceded this stack, if available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<Box<StackTrace>>,
    /// Asynchronous JavaScript stack trace that preceded this stack, if available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<StackTraceId>,
}

/// Unique identifier of current debugger.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UniqueDebuggerId(pub String);

/// If `debuggerId` is set stack trace comes from another debugger and can be resolved there. This
/// allows to track cross-debugger calls. See `Runtime.StackTrace` and `Debugger.paused` for usages.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackTraceId {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub debugger_id: Option<UniqueDebuggerId>,
}

// ── Inline enums ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SerializationOptionsSerialization {
    #[default]
    #[serde(rename = "deep")]
    Deep,
    #[serde(rename = "json")]
    Json,
    #[serde(rename = "idOnly")]
    IdOnly,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeepSerializedValueType {
    #[default]
    #[serde(rename = "undefined")]
    Undefined,
    #[serde(rename = "null")]
    Null,
    #[serde(rename = "string")]
    String,
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "boolean")]
    Boolean,
    #[serde(rename = "bigint")]
    Bigint,
    #[serde(rename = "regexp")]
    Regexp,
    #[serde(rename = "date")]
    Date,
    #[serde(rename = "symbol")]
    Symbol,
    #[serde(rename = "array")]
    Array,
    #[serde(rename = "object")]
    Object,
    #[serde(rename = "function")]
    Function,
    #[serde(rename = "map")]
    Map,
    #[serde(rename = "set")]
    Set,
    #[serde(rename = "weakmap")]
    Weakmap,
    #[serde(rename = "weakset")]
    Weakset,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "proxy")]
    Proxy,
    #[serde(rename = "promise")]
    Promise,
    #[serde(rename = "typedarray")]
    Typedarray,
    #[serde(rename = "arraybuffer")]
    Arraybuffer,
    #[serde(rename = "node")]
    Node,
    #[serde(rename = "window")]
    Window,
    #[serde(rename = "generator")]
    Generator,
}

/// Object type.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RemoteObjectType {
    #[default]
    #[serde(rename = "object")]
    Object,
    #[serde(rename = "function")]
    Function,
    #[serde(rename = "undefined")]
    Undefined,
    #[serde(rename = "string")]
    String,
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "boolean")]
    Boolean,
    #[serde(rename = "symbol")]
    Symbol,
    #[serde(rename = "bigint")]
    Bigint,
}

/// Object subtype hint. Specified for `object` type values only.
/// NOTE: If you change anything here, make sure to also update
/// `subtype` in `ObjectPreview` and `PropertyPreview` below.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RemoteObjectSubtype {
    #[default]
    #[serde(rename = "array")]
    Array,
    #[serde(rename = "null")]
    Null,
    #[serde(rename = "node")]
    Node,
    #[serde(rename = "regexp")]
    Regexp,
    #[serde(rename = "date")]
    Date,
    #[serde(rename = "map")]
    Map,
    #[serde(rename = "set")]
    Set,
    #[serde(rename = "weakmap")]
    Weakmap,
    #[serde(rename = "weakset")]
    Weakset,
    #[serde(rename = "iterator")]
    Iterator,
    #[serde(rename = "generator")]
    Generator,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "proxy")]
    Proxy,
    #[serde(rename = "promise")]
    Promise,
    #[serde(rename = "typedarray")]
    Typedarray,
    #[serde(rename = "arraybuffer")]
    Arraybuffer,
    #[serde(rename = "dataview")]
    Dataview,
    #[serde(rename = "webassemblymemory")]
    Webassemblymemory,
    #[serde(rename = "wasmvalue")]
    Wasmvalue,
    #[serde(rename = "trustedtype")]
    Trustedtype,
}

/// Object type.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObjectPreviewType {
    #[default]
    #[serde(rename = "object")]
    Object,
    #[serde(rename = "function")]
    Function,
    #[serde(rename = "undefined")]
    Undefined,
    #[serde(rename = "string")]
    String,
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "boolean")]
    Boolean,
    #[serde(rename = "symbol")]
    Symbol,
    #[serde(rename = "bigint")]
    Bigint,
}

/// Object subtype hint. Specified for `object` type values only.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObjectPreviewSubtype {
    #[default]
    #[serde(rename = "array")]
    Array,
    #[serde(rename = "null")]
    Null,
    #[serde(rename = "node")]
    Node,
    #[serde(rename = "regexp")]
    Regexp,
    #[serde(rename = "date")]
    Date,
    #[serde(rename = "map")]
    Map,
    #[serde(rename = "set")]
    Set,
    #[serde(rename = "weakmap")]
    Weakmap,
    #[serde(rename = "weakset")]
    Weakset,
    #[serde(rename = "iterator")]
    Iterator,
    #[serde(rename = "generator")]
    Generator,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "proxy")]
    Proxy,
    #[serde(rename = "promise")]
    Promise,
    #[serde(rename = "typedarray")]
    Typedarray,
    #[serde(rename = "arraybuffer")]
    Arraybuffer,
    #[serde(rename = "dataview")]
    Dataview,
    #[serde(rename = "webassemblymemory")]
    Webassemblymemory,
    #[serde(rename = "wasmvalue")]
    Wasmvalue,
    #[serde(rename = "trustedtype")]
    Trustedtype,
}

/// Object type. Accessor means that the property itself is an accessor property.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PropertyPreviewType {
    #[default]
    #[serde(rename = "object")]
    Object,
    #[serde(rename = "function")]
    Function,
    #[serde(rename = "undefined")]
    Undefined,
    #[serde(rename = "string")]
    String,
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "boolean")]
    Boolean,
    #[serde(rename = "symbol")]
    Symbol,
    #[serde(rename = "accessor")]
    Accessor,
    #[serde(rename = "bigint")]
    Bigint,
}

/// Object subtype hint. Specified for `object` type values only.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PropertyPreviewSubtype {
    #[default]
    #[serde(rename = "array")]
    Array,
    #[serde(rename = "null")]
    Null,
    #[serde(rename = "node")]
    Node,
    #[serde(rename = "regexp")]
    Regexp,
    #[serde(rename = "date")]
    Date,
    #[serde(rename = "map")]
    Map,
    #[serde(rename = "set")]
    Set,
    #[serde(rename = "weakmap")]
    Weakmap,
    #[serde(rename = "weakset")]
    Weakset,
    #[serde(rename = "iterator")]
    Iterator,
    #[serde(rename = "generator")]
    Generator,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "proxy")]
    Proxy,
    #[serde(rename = "promise")]
    Promise,
    #[serde(rename = "typedarray")]
    Typedarray,
    #[serde(rename = "arraybuffer")]
    Arraybuffer,
    #[serde(rename = "dataview")]
    Dataview,
    #[serde(rename = "webassemblymemory")]
    Webassemblymemory,
    #[serde(rename = "wasmvalue")]
    Wasmvalue,
    #[serde(rename = "trustedtype")]
    Trustedtype,
}

/// Type of the call.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsoleAPICalledType {
    #[default]
    #[serde(rename = "log")]
    Log,
    #[serde(rename = "debug")]
    Debug,
    #[serde(rename = "info")]
    Info,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "warning")]
    Warning,
    #[serde(rename = "dir")]
    Dir,
    #[serde(rename = "dirxml")]
    Dirxml,
    #[serde(rename = "table")]
    Table,
    #[serde(rename = "trace")]
    Trace,
    #[serde(rename = "clear")]
    Clear,
    #[serde(rename = "startGroup")]
    StartGroup,
    #[serde(rename = "startGroupCollapsed")]
    StartGroupCollapsed,
    #[serde(rename = "endGroup")]
    EndGroup,
    #[serde(rename = "assert")]
    Assert,
    #[serde(rename = "profile")]
    Profile,
    #[serde(rename = "profileEnd")]
    ProfileEnd,
    #[serde(rename = "count")]
    Count,
    #[serde(rename = "timeEnd")]
    TimeEnd,
}

// ── Param types ──────────────────────────────────────────────────────────────

/// Parameters for [`RuntimeCommands::runtime_await_promise`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AwaitPromiseParams {
    /// Identifier of the promise.
    pub promise_object_id: RemoteObjectId,
    /// Whether the result is expected to be a JSON object that should be sent by value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_by_value: Option<bool>,
    /// Whether preview should be generated for the result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generate_preview: Option<bool>,
}

/// Parameters for [`RuntimeCommands::runtime_call_function_on`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CallFunctionOnParams {
    /// Declaration of the function to call.
    pub function_declaration: String,
    /// Identifier of the object to call function on. Either objectId or executionContextId should
    /// be specified.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_id: Option<RemoteObjectId>,
    /// Call arguments. All call arguments must belong to the same JavaScript world as the target
    /// object.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<CallArgument>>,
    /// In silent mode exceptions thrown during evaluation are not reported and do not pause
    /// execution. Overrides `setPauseOnException` state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub silent: Option<bool>,
    /// Whether the result is expected to be a JSON object which should be sent by value.
    /// Can be overriden by `serializationOptions`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_by_value: Option<bool>,
    /// Whether preview should be generated for the result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generate_preview: Option<bool>,
    /// Whether execution should be treated as initiated by user in the UI.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_gesture: Option<bool>,
    /// Whether execution should `await` for resulting value and return once awaited promise is
    /// resolved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub await_promise: Option<bool>,
    /// Specifies execution context which global object will be used to call function on. Either
    /// executionContextId or objectId should be specified.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_context_id: Option<ExecutionContextId>,
    /// Symbolic group name that can be used to release multiple objects. If objectGroup is not
    /// specified and objectId is, objectGroup will be inherited from object.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_group: Option<String>,
    /// Whether to throw an exception if side effect cannot be ruled out during evaluation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub throw_on_side_effect: Option<bool>,
    /// An alternative way to specify the execution context to call function on.
    /// Compared to contextId that may be reused across processes, this is guaranteed to be
    /// system-unique, so it can be used to prevent accidental function call
    /// in context different than intended (e.g. as a result of navigation across process
    /// boundaries).
    /// This is mutually exclusive with `executionContextId`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unique_context_id: Option<String>,
    /// Specifies the result serialization. If provided, overrides
    /// `generatePreview` and `returnByValue`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serialization_options: Option<SerializationOptions>,
}

/// Parameters for [`RuntimeCommands::runtime_compile_script`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompileScriptParams {
    /// Expression to compile.
    pub expression: String,
    /// Source url to be set for the script.
    #[serde(rename = "sourceURL")]
    pub source_url: String,
    /// Specifies whether the compiled script should be persisted.
    pub persist_script: bool,
    /// Specifies in which execution context to perform script run. If the parameter is omitted the
    /// evaluation will be performed in the context of the inspected page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_context_id: Option<ExecutionContextId>,
}

/// Parameters for [`RuntimeCommands::runtime_evaluate`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluateParams {
    /// Expression to evaluate.
    pub expression: String,
    /// Symbolic group name that can be used to release multiple objects.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_group: Option<String>,
    /// Determines whether Command Line API should be available during the evaluation.
    #[serde(rename = "includeCommandLineAPI")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_command_line_api: Option<bool>,
    /// In silent mode exceptions thrown during evaluation are not reported and do not pause
    /// execution. Overrides `setPauseOnException` state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub silent: Option<bool>,
    /// Specifies in which execution context to perform evaluation. If the parameter is omitted the
    /// evaluation will be performed in the context of the inspected page.
    /// This is mutually exclusive with `uniqueContextId`, which offers an
    /// alternative way to identify the execution context that is more reliable
    /// in a multi-process environment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_id: Option<ExecutionContextId>,
    /// Whether the result is expected to be a JSON object that should be sent by value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_by_value: Option<bool>,
    /// Whether preview should be generated for the result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generate_preview: Option<bool>,
    /// Whether execution should be treated as initiated by user in the UI.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_gesture: Option<bool>,
    /// Whether execution should `await` for resulting value and return once awaited promise is
    /// resolved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub await_promise: Option<bool>,
    /// Whether to throw an exception if side effect cannot be ruled out during evaluation.
    /// This implies `disableBreaks` below.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub throw_on_side_effect: Option<bool>,
    /// Terminate execution after timing out (number of milliseconds).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<TimeDelta>,
    /// Disable breakpoints during execution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_breaks: Option<bool>,
    /// Setting this flag to true enables `let` re-declaration and top-level `await`.
    /// Note that `let` variables can only be re-declared if they originate from
    /// `replMode` themselves.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repl_mode: Option<bool>,
    /// The Content Security Policy (CSP) for the target might block 'unsafe-eval'
    /// which includes eval(), Function(), setTimeout() and setInterval()
    /// when called with non-callable arguments. This flag bypasses CSP for this
    /// evaluation and allows unsafe-eval. Defaults to true.
    #[serde(rename = "allowUnsafeEvalBlockedByCSP")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_unsafe_eval_blocked_by_csp: Option<bool>,
    /// An alternative way to specify the execution context to evaluate in.
    /// Compared to contextId that may be reused across processes, this is guaranteed to be
    /// system-unique, so it can be used to prevent accidental evaluation of the expression
    /// in context different than intended (e.g. as a result of navigation across process
    /// boundaries).
    /// This is mutually exclusive with `contextId`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unique_context_id: Option<String>,
    /// Specifies the result serialization. If provided, overrides
    /// `generatePreview` and `returnByValue`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serialization_options: Option<SerializationOptions>,
}

/// Parameters for [`RuntimeCommands::runtime_get_properties`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPropertiesParams {
    /// Identifier of the object to return properties for.
    pub object_id: RemoteObjectId,
    /// If true, returns properties belonging only to the element itself, not to its prototype
    /// chain.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub own_properties: Option<bool>,
    /// If true, returns accessor properties (with getter/setter) only; internal properties are not
    /// returned either.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessor_properties_only: Option<bool>,
    /// Whether preview should be generated for the results.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generate_preview: Option<bool>,
    /// If true, returns non-indexed properties only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub non_indexed_properties_only: Option<bool>,
}

/// Parameters for [`RuntimeCommands::runtime_global_lexical_scope_names`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalLexicalScopeNamesParams {
    /// Specifies in which execution context to lookup global scope variables.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_context_id: Option<ExecutionContextId>,
}

/// Parameters for [`RuntimeCommands::runtime_query_objects`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryObjectsParams {
    /// Identifier of the prototype to return objects for.
    pub prototype_object_id: RemoteObjectId,
    /// Symbolic group name that can be used to release the results.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_group: Option<String>,
}

/// Parameters for [`RuntimeCommands::runtime_run_script`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunScriptParams {
    /// Id of the script to run.
    pub script_id: ScriptId,
    /// Specifies in which execution context to perform script run. If the parameter is omitted the
    /// evaluation will be performed in the context of the inspected page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_context_id: Option<ExecutionContextId>,
    /// Symbolic group name that can be used to release multiple objects.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_group: Option<String>,
    /// In silent mode exceptions thrown during evaluation are not reported and do not pause
    /// execution. Overrides `setPauseOnException` state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub silent: Option<bool>,
    /// Determines whether Command Line API should be available during the evaluation.
    #[serde(rename = "includeCommandLineAPI")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_command_line_api: Option<bool>,
    /// Whether the result is expected to be a JSON object which should be sent by value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_by_value: Option<bool>,
    /// Whether preview should be generated for the result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generate_preview: Option<bool>,
    /// Whether execution should `await` for resulting value and return once awaited promise is
    /// resolved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub await_promise: Option<bool>,
}

/// Parameters for [`RuntimeCommands::runtime_add_binding`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddBindingParams {
    pub name: String,
    /// If specified, the binding would only be exposed to the specified
    /// execution context. If omitted and `executionContextName` is not set,
    /// the binding is exposed to all execution contexts of the target.
    /// This parameter is mutually exclusive with `executionContextName`.
    /// Deprecated in favor of `executionContextName` due to an unclear use case
    /// and bugs in implementation (crbug.com/1169639). `executionContextId` will be
    /// removed in the future.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_context_id: Option<ExecutionContextId>,
    /// If specified, the binding is exposed to the executionContext with
    /// matching name, even for contexts created after the binding is added.
    /// See also `ExecutionContext.name` and `worldName` parameter to
    /// `Page.addScriptToEvaluateOnNewDocument`.
    /// This parameter is mutually exclusive with `executionContextId`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_context_name: Option<String>,
}

// ── Return types ─────────────────────────────────────────────────────────────

/// Return type for [`RuntimeCommands::runtime_await_promise`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AwaitPromiseReturn {
    /// Promise result. Will contain rejected value if promise was rejected.
    pub result: RemoteObject,
    /// Exception details if stack strace is available.
    #[serde(default)]
    pub exception_details: Option<ExceptionDetails>,
}

/// Return type for [`RuntimeCommands::runtime_call_function_on`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallFunctionOnReturn {
    /// Call result.
    pub result: RemoteObject,
    /// Exception details.
    #[serde(default)]
    pub exception_details: Option<ExceptionDetails>,
}

/// Return type for [`RuntimeCommands::runtime_compile_script`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompileScriptReturn {
    /// Id of the script.
    #[serde(default)]
    pub script_id: Option<ScriptId>,
    /// Exception details.
    #[serde(default)]
    pub exception_details: Option<ExceptionDetails>,
}

/// Return type for [`RuntimeCommands::runtime_evaluate`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluateReturn {
    /// Evaluation result.
    pub result: RemoteObject,
    /// Exception details.
    #[serde(default)]
    pub exception_details: Option<ExceptionDetails>,
}

/// Return type for [`RuntimeCommands::runtime_get_isolate_id`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetIsolateIdReturn {
    /// The isolate id.
    pub id: String,
}

/// Return type for [`RuntimeCommands::runtime_get_heap_usage`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetHeapUsageReturn {
    /// Used JavaScript heap size in bytes.
    pub used_size: f64,
    /// Allocated JavaScript heap size in bytes.
    pub total_size: f64,
    /// Used size in bytes in the embedder's garbage-collected heap.
    pub embedder_heap_used_size: f64,
    /// Size in bytes of backing storage for array buffers and external strings.
    pub backing_storage_size: f64,
}

/// Return type for [`RuntimeCommands::runtime_get_properties`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPropertiesReturn {
    /// Object properties.
    pub result: Vec<PropertyDescriptor>,
    /// Internal object properties (only of the element itself).
    #[serde(default)]
    pub internal_properties: Option<Vec<InternalPropertyDescriptor>>,
    /// Object private properties.
    #[serde(default)]
    pub private_properties: Option<Vec<PrivatePropertyDescriptor>>,
    /// Exception details.
    #[serde(default)]
    pub exception_details: Option<ExceptionDetails>,
}

/// Return type for [`RuntimeCommands::runtime_global_lexical_scope_names`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalLexicalScopeNamesReturn {
    pub names: Vec<String>,
}

/// Return type for [`RuntimeCommands::runtime_query_objects`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryObjectsReturn {
    /// Array with objects.
    pub objects: RemoteObject,
}

/// Return type for [`RuntimeCommands::runtime_run_script`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunScriptReturn {
    /// Run result.
    pub result: RemoteObject,
    /// Exception details.
    #[serde(default)]
    pub exception_details: Option<ExceptionDetails>,
}

/// Return type for [`RuntimeCommands::runtime_get_exception_details`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetExceptionDetailsReturn {
    #[serde(default)]
    pub exception_details: Option<ExceptionDetails>,
}

// ── Events ───────────────────────────────────────────────────────────────────

/// Notification is issued every time when binding is called.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BindingCalledEvent {
    pub name: String,
    pub payload: String,
    /// Identifier of the context where the call was made.
    pub execution_context_id: ExecutionContextId,
}

/// Issued when console API was called.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsoleAPICalledEvent {
    /// Type of the call.
    pub r#type: ConsoleAPICalledType,
    /// Call arguments.
    pub args: Vec<RemoteObject>,
    /// Identifier of the context where the call was made.
    pub execution_context_id: ExecutionContextId,
    /// Call timestamp.
    pub timestamp: Timestamp,
    /// Stack trace captured when the call was made. The async stack chain is automatically reported for
    /// the following call types: `assert`, `error`, `trace`, `warning`. For other types the async call
    /// chain can be retrieved using `Debugger.getStackTrace` and `stackTrace.parentId` field.
    #[serde(default)]
    pub stack_trace: Option<StackTrace>,
    /// Console context descriptor for calls on non-default console context (not console.*):
    /// 'anonymous#unique-logger-id' for call on unnamed context, 'name#unique-logger-id' for call
    /// on named context.
    #[serde(default)]
    pub context: Option<String>,
}

/// Issued when unhandled exception was revoked.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExceptionRevokedEvent {
    /// Reason describing why exception was revoked.
    pub reason: String,
    /// The id of revoked exception, as reported in `exceptionThrown`.
    pub exception_id: i64,
}

/// Issued when exception was thrown and unhandled.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExceptionThrownEvent {
    /// Timestamp of the exception.
    pub timestamp: Timestamp,
    pub exception_details: ExceptionDetails,
}

/// Issued when new execution context is created.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionContextCreatedEvent {
    /// A newly created execution context.
    pub context: ExecutionContextDescription,
}

/// Issued when execution context is destroyed.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionContextDestroyedEvent {
    /// Id of the destroyed context.
    pub execution_context_id: ExecutionContextId,
    /// Unique Id of the destroyed context.
    pub execution_context_unique_id: String,
}

/// Issued when all executionContexts were cleared in browser.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionContextsClearedEvent {}

/// Issued when object should be inspected (for example, as a result of inspect() command line API
/// call).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectRequestedEvent {
    pub object: RemoteObject,
    pub hints: serde_json::Value,
    /// Identifier of the context where the call was made.
    #[serde(default)]
    pub execution_context_id: Option<ExecutionContextId>,
}

// ── Domain trait ─────────────────────────────────────────────────────────────

/// `Runtime` domain CDP methods.
///
/// Runtime domain exposes JavaScript runtime by means of remote evaluation and mirror objects.
/// Evaluation results are returned as mirror object that expose object type, string representation
/// and unique identifier that can be used for further object reference. Original objects are
/// maintained in memory unless they are either explicitly released or are released along with the
/// other objects in their object group.
///
/// Reference: <https://chromedevtools.github.io/devtools-protocol/tot/Runtime/>
pub trait RuntimeCommands {
    /// Add handler to promise with given promise object id.
    ///
    /// CDP: `Runtime.awaitPromise`
    async fn runtime_await_promise(
        &self,
        params: &AwaitPromiseParams,
    ) -> Result<AwaitPromiseReturn>;

    /// Calls function with given declaration on the given object. Object group of the result is
    /// inherited from the target object.
    ///
    /// CDP: `Runtime.callFunctionOn`
    async fn runtime_call_function_on(
        &self,
        params: &CallFunctionOnParams,
    ) -> Result<CallFunctionOnReturn>;

    /// Compiles expression.
    ///
    /// CDP: `Runtime.compileScript`
    async fn runtime_compile_script(
        &self,
        params: &CompileScriptParams,
    ) -> Result<CompileScriptReturn>;

    /// Disables reporting of execution contexts creation.
    ///
    /// CDP: `Runtime.disable`
    async fn runtime_disable(&self) -> Result<()>;

    /// Discards collected exceptions and console API calls.
    ///
    /// CDP: `Runtime.discardConsoleEntries`
    async fn runtime_discard_console_entries(&self) -> Result<()>;

    /// Enables reporting of execution contexts creation by means of `executionContextCreated` event.
    /// When the reporting gets enabled the event will be sent immediately for each existing execution
    /// context.
    ///
    /// CDP: `Runtime.enable`
    async fn runtime_enable(&self) -> Result<()>;

    /// Evaluates expression on global object.
    ///
    /// CDP: `Runtime.evaluate`
    async fn runtime_evaluate(&self, params: &EvaluateParams) -> Result<EvaluateReturn>;

    /// Returns the isolate id.
    ///
    /// CDP: `Runtime.getIsolateId`
    async fn runtime_get_isolate_id(&self) -> Result<GetIsolateIdReturn>;

    /// Returns the JavaScript heap usage.
    /// It is the total usage of the corresponding isolate not scoped to a particular Runtime.
    ///
    /// CDP: `Runtime.getHeapUsage`
    async fn runtime_get_heap_usage(&self) -> Result<GetHeapUsageReturn>;

    /// Returns properties of a given object. Object group of the result is inherited from the target
    /// object.
    ///
    /// CDP: `Runtime.getProperties`
    async fn runtime_get_properties(
        &self,
        params: &GetPropertiesParams,
    ) -> Result<GetPropertiesReturn>;

    /// Returns all let, const and class variables from global scope.
    ///
    /// CDP: `Runtime.globalLexicalScopeNames`
    async fn runtime_global_lexical_scope_names(
        &self,
        params: &GlobalLexicalScopeNamesParams,
    ) -> Result<GlobalLexicalScopeNamesReturn>;

    ///
    /// CDP: `Runtime.queryObjects`
    async fn runtime_query_objects(
        &self,
        params: &QueryObjectsParams,
    ) -> Result<QueryObjectsReturn>;

    /// Releases remote object with given id.
    ///
    /// CDP: `Runtime.releaseObject`
    async fn runtime_release_object(&self, object_id: &RemoteObjectId) -> Result<()>;

    /// Releases all remote objects that belong to a given group.
    ///
    /// CDP: `Runtime.releaseObjectGroup`
    async fn runtime_release_object_group(&self, object_group: &str) -> Result<()>;

    /// Tells inspected instance to run if it was waiting for debugger to attach.
    ///
    /// CDP: `Runtime.runIfWaitingForDebugger`
    async fn runtime_run_if_waiting_for_debugger(&self) -> Result<()>;

    /// Runs script with given id in a given context.
    ///
    /// CDP: `Runtime.runScript`
    async fn runtime_run_script(&self, params: &RunScriptParams) -> Result<RunScriptReturn>;

    /// Enables or disables async call stacks tracking.
    ///
    /// CDP: `Runtime.setAsyncCallStackDepth`
    async fn runtime_set_async_call_stack_depth(&self, max_depth: i64) -> Result<()>;

    ///
    /// CDP: `Runtime.setCustomObjectFormatterEnabled`
    async fn runtime_set_custom_object_formatter_enabled(&self, enabled: bool) -> Result<()>;

    ///
    /// CDP: `Runtime.setMaxCallStackSizeToCapture`
    async fn runtime_set_max_call_stack_size_to_capture(&self, size: i64) -> Result<()>;

    /// Terminate current or next JavaScript execution.
    /// Will cancel the termination when the outer-most script execution ends.
    ///
    /// CDP: `Runtime.terminateExecution`
    async fn runtime_terminate_execution(&self) -> Result<()>;

    /// If executionContextId is empty, adds binding with the given name on the
    /// global objects of all inspected contexts, including those created later,
    /// bindings survive reloads.
    /// Binding function takes exactly one argument, this argument should be string,
    /// in case of any other input, function throws an exception.
    /// Each binding function call produces Runtime.bindingCalled notification.
    ///
    /// CDP: `Runtime.addBinding`
    async fn runtime_add_binding(&self, params: &AddBindingParams) -> Result<()>;

    /// This method does not remove binding function from global object but
    /// unsubscribes current runtime agent from Runtime.bindingCalled notifications.
    ///
    /// CDP: `Runtime.removeBinding`
    async fn runtime_remove_binding(&self, name: &str) -> Result<()>;

    /// This method tries to lookup and populate exception details for a
    /// JavaScript Error object.
    /// Note that the stackTrace portion of the resulting exceptionDetails will
    /// only be populated if the Runtime domain was enabled at the time when the
    /// Error was thrown.
    ///
    /// CDP: `Runtime.getExceptionDetails`
    async fn runtime_get_exception_details(
        &self,
        error_object_id: &RemoteObjectId,
    ) -> Result<GetExceptionDetailsReturn>;
}

// ── Impl ─────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ReleaseObjectInternalParams<'a> {
    object_id: &'a RemoteObjectId,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ReleaseObjectGroupInternalParams<'a> {
    object_group: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetAsyncCallStackDepthInternalParams {
    max_depth: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetCustomObjectFormatterEnabledInternalParams {
    enabled: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetMaxCallStackSizeToCaptureInternalParams {
    size: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RemoveBindingInternalParams<'a> {
    name: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GetExceptionDetailsInternalParams<'a> {
    error_object_id: &'a RemoteObjectId,
}

impl RuntimeCommands for CdpSession {
    async fn runtime_await_promise(
        &self,
        params: &AwaitPromiseParams,
    ) -> Result<AwaitPromiseReturn> {
        self.call("Runtime.awaitPromise", params).await
    }

    async fn runtime_call_function_on(
        &self,
        params: &CallFunctionOnParams,
    ) -> Result<CallFunctionOnReturn> {
        self.call("Runtime.callFunctionOn", params).await
    }

    async fn runtime_compile_script(
        &self,
        params: &CompileScriptParams,
    ) -> Result<CompileScriptReturn> {
        self.call("Runtime.compileScript", params).await
    }

    async fn runtime_disable(&self) -> Result<()> {
        self.call_no_response("Runtime.disable", &serde_json::json!({}))
            .await
    }

    async fn runtime_discard_console_entries(&self) -> Result<()> {
        self.call_no_response("Runtime.discardConsoleEntries", &serde_json::json!({}))
            .await
    }

    async fn runtime_enable(&self) -> Result<()> {
        self.call_no_response("Runtime.enable", &serde_json::json!({}))
            .await
    }

    async fn runtime_evaluate(&self, params: &EvaluateParams) -> Result<EvaluateReturn> {
        self.call("Runtime.evaluate", params).await
    }

    async fn runtime_get_isolate_id(&self) -> Result<GetIsolateIdReturn> {
        self.call("Runtime.getIsolateId", &serde_json::json!({}))
            .await
    }

    async fn runtime_get_heap_usage(&self) -> Result<GetHeapUsageReturn> {
        self.call("Runtime.getHeapUsage", &serde_json::json!({}))
            .await
    }

    async fn runtime_get_properties(
        &self,
        params: &GetPropertiesParams,
    ) -> Result<GetPropertiesReturn> {
        self.call("Runtime.getProperties", params).await
    }

    async fn runtime_global_lexical_scope_names(
        &self,
        params: &GlobalLexicalScopeNamesParams,
    ) -> Result<GlobalLexicalScopeNamesReturn> {
        self.call("Runtime.globalLexicalScopeNames", params).await
    }

    async fn runtime_query_objects(
        &self,
        params: &QueryObjectsParams,
    ) -> Result<QueryObjectsReturn> {
        self.call("Runtime.queryObjects", params).await
    }

    async fn runtime_release_object(&self, object_id: &RemoteObjectId) -> Result<()> {
        let params = ReleaseObjectInternalParams { object_id };
        self.call_no_response("Runtime.releaseObject", &params)
            .await
    }

    async fn runtime_release_object_group(&self, object_group: &str) -> Result<()> {
        let params = ReleaseObjectGroupInternalParams { object_group };
        self.call_no_response("Runtime.releaseObjectGroup", &params)
            .await
    }

    async fn runtime_run_if_waiting_for_debugger(&self) -> Result<()> {
        self.call_no_response("Runtime.runIfWaitingForDebugger", &serde_json::json!({}))
            .await
    }

    async fn runtime_run_script(&self, params: &RunScriptParams) -> Result<RunScriptReturn> {
        self.call("Runtime.runScript", params).await
    }

    async fn runtime_set_async_call_stack_depth(&self, max_depth: i64) -> Result<()> {
        let params = SetAsyncCallStackDepthInternalParams { max_depth };
        self.call_no_response("Runtime.setAsyncCallStackDepth", &params)
            .await
    }

    async fn runtime_set_custom_object_formatter_enabled(&self, enabled: bool) -> Result<()> {
        let params = SetCustomObjectFormatterEnabledInternalParams { enabled };
        self.call_no_response("Runtime.setCustomObjectFormatterEnabled", &params)
            .await
    }

    async fn runtime_set_max_call_stack_size_to_capture(&self, size: i64) -> Result<()> {
        let params = SetMaxCallStackSizeToCaptureInternalParams { size };
        self.call_no_response("Runtime.setMaxCallStackSizeToCapture", &params)
            .await
    }

    async fn runtime_terminate_execution(&self) -> Result<()> {
        self.call_no_response("Runtime.terminateExecution", &serde_json::json!({}))
            .await
    }

    async fn runtime_add_binding(&self, params: &AddBindingParams) -> Result<()> {
        self.call_no_response("Runtime.addBinding", params).await
    }

    async fn runtime_remove_binding(&self, name: &str) -> Result<()> {
        let params = RemoveBindingInternalParams { name };
        self.call_no_response("Runtime.removeBinding", &params)
            .await
    }

    async fn runtime_get_exception_details(
        &self,
        error_object_id: &RemoteObjectId,
    ) -> Result<GetExceptionDetailsReturn> {
        let params = GetExceptionDetailsInternalParams { error_object_id };
        self.call("Runtime.getExceptionDetails", &params).await
    }
}
