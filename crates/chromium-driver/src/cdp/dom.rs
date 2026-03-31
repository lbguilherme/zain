use serde::{Deserialize, Serialize};

use crate::cdp::runtime::RemoteObject;
use crate::error::Result;
use crate::session::CdpSession;

// --- Return types ---

/// Return type for [`DomCommands::dom_get_document`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetDocumentReturn {
    /// The root DOM node.
    pub root: Node,
}

/// A DOM node as returned by CDP.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    /// Node identifier used in further DOM operations.
    pub node_id: i64,
    /// Node's `nodeName`.
    #[serde(default)]
    pub node_name: String,
    /// Node's `localName`.
    #[serde(default)]
    pub local_name: String,
    /// Node's `nodeValue`.
    #[serde(default)]
    pub node_value: String,
    /// Child node count (if requested).
    #[serde(default)]
    pub child_node_count: Option<i64>,
    /// Child nodes (if depth > 0).
    #[serde(default)]
    pub children: Option<Vec<Node>>,
    /// Attributes as flat array: [name1, value1, name2, value2, ...].
    #[serde(default)]
    pub attributes: Option<Vec<String>>,
}

/// Return type for [`DomCommands::dom_query_selector`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuerySelectorReturn {
    /// Node ID of the matched element. `0` means no match.
    pub node_id: i64,
}

/// Return type for [`DomCommands::dom_query_selector_all`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuerySelectorAllReturn {
    /// Node IDs of all matched elements.
    pub node_ids: Vec<i64>,
}

/// Return type for [`DomCommands::dom_get_box_model`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBoxModelReturn {
    /// Box model data for the node.
    pub model: BoxModel,
}

/// CSS box model for a DOM node.
///
/// Each quad is an array of 8 floats: [x1,y1, x2,y2, x3,y3, x4,y4]
/// representing the four corners of the box.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoxModel {
    /// Content box quad.
    pub content: Vec<f64>,
    /// Padding box quad.
    pub padding: Vec<f64>,
    /// Border box quad.
    pub border: Vec<f64>,
    /// Margin box quad.
    pub margin: Vec<f64>,
    /// Node width in CSS pixels.
    pub width: i64,
    /// Node height in CSS pixels.
    pub height: i64,
}

/// Return type for [`DomCommands::dom_get_outer_html`].
#[derive(Debug, Deserialize)]
pub struct GetOuterHtmlReturn {
    /// Outer HTML of the node.
    #[serde(rename = "outerHTML")]
    pub outer_html: String,
}

/// Return type for [`DomCommands::dom_get_attributes`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetAttributesReturn {
    /// Flat array of attribute name/value pairs: [name1, value1, name2, value2, ...].
    pub attributes: Vec<String>,
}

/// Return type for [`DomCommands::dom_describe_node`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DescribeNodeReturn {
    /// Node description.
    pub node: Node,
}

/// Return type for [`DomCommands::dom_resolve_node`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveNodeReturn {
    /// JavaScript object corresponding to the DOM node.
    pub object: RemoteObject,
}

// --- Param types ---

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct QuerySelectorParams {
    pub node_id: i64,
    pub selector: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GetBoxModelParams {
    pub node_id: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GetOuterHtmlParams {
    pub node_id: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GetAttributesParams {
    pub node_id: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DescribeNodeParams {
    pub node_id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResolveNodeParams {
    pub node_id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_group: Option<String>,
}

// --- Domain trait ---

/// `DOM` domain CDP methods.
///
/// Reference: <https://chromedevtools.github.io/devtools-protocol/tot/DOM/>
pub trait DomCommands {
    /// Enables the DOM domain. Required before any DOM operations.
    ///
    /// CDP: `DOM.enable`
    async fn dom_enable(&self) -> Result<()>;

    /// Disables the DOM domain.
    ///
    /// CDP: `DOM.disable`
    async fn dom_disable(&self) -> Result<()>;

    /// Returns the root DOM node of the document.
    ///
    /// - `depth`: maximum depth of children to return. `0` = root only.
    ///
    /// CDP: `DOM.getDocument`
    async fn dom_get_document(&self, depth: i64) -> Result<GetDocumentReturn>;

    /// Finds the first element matching a CSS selector within the given node's subtree.
    ///
    /// Returns `node_id = 0` if no element matches.
    ///
    /// CDP: `DOM.querySelector`
    async fn dom_query_selector(&self, node_id: i64, selector: &str)
        -> Result<QuerySelectorReturn>;

    /// Finds all elements matching a CSS selector within the given node's subtree.
    ///
    /// CDP: `DOM.querySelectorAll`
    async fn dom_query_selector_all(
        &self,
        node_id: i64,
        selector: &str,
    ) -> Result<QuerySelectorAllReturn>;

    /// Returns the CSS box model for the given node.
    ///
    /// CDP: `DOM.getBoxModel`
    async fn dom_get_box_model(&self, node_id: i64) -> Result<GetBoxModelReturn>;

    /// Returns the outer HTML of the given node.
    ///
    /// CDP: `DOM.getOuterHTML`
    async fn dom_get_outer_html(&self, node_id: i64) -> Result<GetOuterHtmlReturn>;

    /// Returns the attributes of the given node as name/value pairs.
    ///
    /// CDP: `DOM.getAttributes`
    async fn dom_get_attributes(&self, node_id: i64) -> Result<GetAttributesReturn>;

    /// Describes the given node. Can return children up to the specified depth.
    ///
    /// CDP: `DOM.describeNode`
    async fn dom_describe_node(&self, node_id: i64, depth: Option<i64>)
        -> Result<DescribeNodeReturn>;

    /// Resolves a DOM node to a JavaScript `RemoteObject`.
    ///
    /// CDP: `DOM.resolveNode`
    async fn dom_resolve_node(
        &self,
        node_id: i64,
        object_group: Option<&str>,
    ) -> Result<ResolveNodeReturn>;

    /// Scrolls the given node into the viewport if it is not already visible.
    ///
    /// CDP: `DOM.scrollIntoViewIfNeeded`
    async fn dom_scroll_into_view_if_needed(&self, node_id: i64) -> Result<()>;
}

impl DomCommands for CdpSession {
    async fn dom_enable(&self) -> Result<()> {
        self.call_no_response("DOM.enable", &serde_json::json!({}))
            .await
    }

    async fn dom_disable(&self) -> Result<()> {
        self.call_no_response("DOM.disable", &serde_json::json!({}))
            .await
    }

    async fn dom_get_document(&self, depth: i64) -> Result<GetDocumentReturn> {
        self.call("DOM.getDocument", &serde_json::json!({"depth": depth}))
            .await
    }

    async fn dom_query_selector(
        &self,
        node_id: i64,
        selector: &str,
    ) -> Result<QuerySelectorReturn> {
        let params = QuerySelectorParams {
            node_id,
            selector: selector.to_owned(),
        };
        self.call("DOM.querySelector", &params).await
    }

    async fn dom_query_selector_all(
        &self,
        node_id: i64,
        selector: &str,
    ) -> Result<QuerySelectorAllReturn> {
        let params = QuerySelectorParams {
            node_id,
            selector: selector.to_owned(),
        };
        self.call("DOM.querySelectorAll", &params).await
    }

    async fn dom_get_box_model(&self, node_id: i64) -> Result<GetBoxModelReturn> {
        self.call("DOM.getBoxModel", &GetBoxModelParams { node_id })
            .await
    }

    async fn dom_get_outer_html(&self, node_id: i64) -> Result<GetOuterHtmlReturn> {
        self.call("DOM.getOuterHTML", &GetOuterHtmlParams { node_id })
            .await
    }

    async fn dom_get_attributes(&self, node_id: i64) -> Result<GetAttributesReturn> {
        self.call("DOM.getAttributes", &GetAttributesParams { node_id })
            .await
    }

    async fn dom_describe_node(
        &self,
        node_id: i64,
        depth: Option<i64>,
    ) -> Result<DescribeNodeReturn> {
        self.call(
            "DOM.describeNode",
            &DescribeNodeParams { node_id, depth },
        )
        .await
    }

    async fn dom_resolve_node(
        &self,
        node_id: i64,
        object_group: Option<&str>,
    ) -> Result<ResolveNodeReturn> {
        self.call(
            "DOM.resolveNode",
            &ResolveNodeParams {
                node_id,
                object_group: object_group.map(String::from),
            },
        )
        .await
    }

    async fn dom_scroll_into_view_if_needed(&self, node_id: i64) -> Result<()> {
        self.call_no_response(
            "DOM.scrollIntoViewIfNeeded",
            &serde_json::json!({"nodeId": node_id}),
        )
        .await
    }
}
