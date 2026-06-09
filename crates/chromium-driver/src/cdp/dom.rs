use serde::{Deserialize, Serialize};

use crate::cdp::common::AdProvenance;
use crate::cdp::page::FrameId;
use crate::cdp::runtime::ExecutionContextId;
use crate::cdp::runtime::RemoteObject;
use crate::cdp::runtime::RemoteObjectId;
use crate::cdp::runtime::StackTrace;
use crate::error::Result;
use crate::session::CdpSession;

// ── Types ────────────────────────────────────────────────────────────────────

/// Unique DOM node identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub i64);

/// Unique DOM node identifier used to reference a node that may not have been pushed to the
/// front-end.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BackendNodeId(pub i64);

/// Unique identifier for a CSS stylesheet.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StyleSheetId(pub String);

/// Backend node with a friendly name.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackendNode {
    /// `Node`'s nodeType.
    pub node_type: i64,
    /// `Node`'s nodeName.
    pub node_name: String,
    pub backend_node_id: BackendNodeId,
}

/// Pseudo element type.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PseudoType {
    #[default]
    #[serde(rename = "first-line")]
    FirstLine,
    #[serde(rename = "first-letter")]
    FirstLetter,
    #[serde(rename = "checkmark")]
    Checkmark,
    #[serde(rename = "before")]
    Before,
    #[serde(rename = "after")]
    After,
    #[serde(rename = "expand-icon")]
    ExpandIcon,
    #[serde(rename = "picker-icon")]
    PickerIcon,
    #[serde(rename = "interest-button")]
    InterestButton,
    #[serde(rename = "marker")]
    Marker,
    #[serde(rename = "backdrop")]
    Backdrop,
    #[serde(rename = "column")]
    Column,
    #[serde(rename = "selection")]
    Selection,
    #[serde(rename = "search-text")]
    SearchText,
    #[serde(rename = "target-text")]
    TargetText,
    #[serde(rename = "spelling-error")]
    SpellingError,
    #[serde(rename = "grammar-error")]
    GrammarError,
    #[serde(rename = "highlight")]
    Highlight,
    #[serde(rename = "first-line-inherited")]
    FirstLineInherited,
    #[serde(rename = "scroll-marker")]
    ScrollMarker,
    #[serde(rename = "scroll-marker-group")]
    ScrollMarkerGroup,
    #[serde(rename = "scroll-button")]
    ScrollButton,
    #[serde(rename = "scrollbar")]
    Scrollbar,
    #[serde(rename = "scrollbar-thumb")]
    ScrollbarThumb,
    #[serde(rename = "scrollbar-button")]
    ScrollbarButton,
    #[serde(rename = "scrollbar-track")]
    ScrollbarTrack,
    #[serde(rename = "scrollbar-track-piece")]
    ScrollbarTrackPiece,
    #[serde(rename = "scrollbar-corner")]
    ScrollbarCorner,
    #[serde(rename = "resizer")]
    Resizer,
    #[serde(rename = "input-list-button")]
    InputListButton,
    #[serde(rename = "view-transition")]
    ViewTransition,
    #[serde(rename = "view-transition-group")]
    ViewTransitionGroup,
    #[serde(rename = "view-transition-image-pair")]
    ViewTransitionImagePair,
    #[serde(rename = "view-transition-group-children")]
    ViewTransitionGroupChildren,
    #[serde(rename = "view-transition-old")]
    ViewTransitionOld,
    #[serde(rename = "view-transition-new")]
    ViewTransitionNew,
    #[serde(rename = "placeholder")]
    Placeholder,
    #[serde(rename = "file-selector-button")]
    FileSelectorButton,
    #[serde(rename = "details-content")]
    DetailsContent,
    #[serde(rename = "picker")]
    Picker,
    #[serde(rename = "permission-icon")]
    PermissionIcon,
    #[serde(rename = "overscroll-area-parent")]
    OverscrollAreaParent,
    #[serde(rename = "skeleton")]
    Skeleton,
}

/// Shadow root type.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShadowRootType {
    #[default]
    #[serde(rename = "user-agent")]
    UserAgent,
    #[serde(rename = "open")]
    Open,
    #[serde(rename = "closed")]
    Closed,
}

/// Document compatibility mode.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompatibilityMode {
    #[default]
    QuirksMode,
    LimitedQuirksMode,
    NoQuirksMode,
}

/// ContainerSelector physical axes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PhysicalAxes {
    #[default]
    Horizontal,
    Vertical,
    Both,
}

/// ContainerSelector logical axes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogicalAxes {
    #[default]
    Inline,
    Block,
    Both,
}

/// Physical scroll orientation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScrollOrientation {
    #[default]
    #[serde(rename = "horizontal")]
    Horizontal,
    #[serde(rename = "vertical")]
    Vertical,
}

/// DOM interaction is implemented in terms of mirror objects that represent the actual DOM nodes.
/// DOMNode is a base node mirror type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    /// Node identifier that is passed into the rest of the DOM messages as the `nodeId`. Backend
    /// will only push node with given `id` once. It is aware of all requested nodes and will only
    /// fire DOM events for nodes known to the client.
    pub node_id: NodeId,
    /// The id of the parent node if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<NodeId>,
    /// The BackendNodeId for this node.
    pub backend_node_id: BackendNodeId,
    /// `Node`'s nodeType.
    pub node_type: i64,
    /// `Node`'s nodeName.
    pub node_name: String,
    /// `Node`'s localName.
    pub local_name: String,
    /// `Node`'s nodeValue.
    pub node_value: String,
    /// Child count for `Container` nodes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub child_node_count: Option<i64>,
    /// Child nodes of this node when requested with children.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<Box<Node>>>,
    /// Attributes of the `Element` node in the form of flat array `[name1, value1, name2, value2]`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attributes: Option<Vec<String>>,
    /// Document URL that `Document` or `FrameOwner` node points to.
    #[serde(rename = "documentURL")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document_url: Option<String>,
    /// Base URL that `Document` or `FrameOwner` node uses for URL completion.
    #[serde(rename = "baseURL")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    /// `DocumentType`'s publicId.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_id: Option<String>,
    /// `DocumentType`'s systemId.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_id: Option<String>,
    /// `DocumentType`'s internalSubset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub internal_subset: Option<String>,
    /// `Document`'s XML version in case of XML documents.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub xml_version: Option<String>,
    /// `Attr`'s name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// `Attr`'s value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// Pseudo element type for this node.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pseudo_type: Option<PseudoType>,
    /// Pseudo element identifier for this node. Only present if there is a
    /// valid pseudoType.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pseudo_identifier: Option<String>,
    /// Shadow root type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow_root_type: Option<ShadowRootType>,
    /// Frame ID for frame owner elements.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_id: Option<FrameId>,
    /// Content document for frame owner elements.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_document: Option<Box<Node>>,
    /// Shadow root list for given element host.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow_roots: Option<Vec<Box<Node>>>,
    /// Content document fragment for template elements.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_content: Option<Box<Node>>,
    /// Pseudo elements associated with this node.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pseudo_elements: Option<Vec<Box<Node>>>,
    /// Deprecated, as the HTML Imports API has been removed (crbug.com/937746).
    /// This property used to return the imported document for the HTMLImport links.
    /// The property is always undefined now.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub imported_document: Option<Box<Node>>,
    /// Distributed nodes for given insertion point.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub distributed_nodes: Option<Vec<BackendNode>>,
    /// Whether the node is SVG.
    #[serde(rename = "isSVG")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_svg: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compatibility_mode: Option<CompatibilityMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assigned_slot: Option<BackendNode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_scrollable: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub affected_by_starting_styles: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adopted_style_sheets: Option<Vec<StyleSheetId>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ad_provenance: Option<AdProvenance>,
}

/// A structure to hold the top-level node of a detached tree and an array of its retained descendants.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetachedElementInfo {
    pub tree_node: Node,
    pub retained_node_ids: Vec<NodeId>,
}

/// A structure holding an RGBA color.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RGBA {
    /// The red component, in the [0-255] range.
    pub r: i64,
    /// The green component, in the [0-255] range.
    pub g: i64,
    /// The blue component, in the [0-255] range.
    pub b: i64,
    /// The alpha component, in the [0-1] range (default: 1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub a: Option<f64>,
}

/// An array of quad vertices, x immediately followed by y for each point, points clock-wise.
pub type Quad = Vec<f64>;

/// Box model.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoxModel {
    /// Content box.
    pub content: Quad,
    /// Padding box.
    pub padding: Quad,
    /// Border box.
    pub border: Quad,
    /// Margin box.
    pub margin: Quad,
    /// Node width.
    pub width: i64,
    /// Node height.
    pub height: i64,
    /// Shape outside coordinates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape_outside: Option<ShapeOutsideInfo>,
}

/// CSS Shape Outside details.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShapeOutsideInfo {
    /// Shape bounds.
    pub bounds: Quad,
    /// Shape coordinate details.
    pub shape: Vec<serde_json::Value>,
    /// Margin shape bounds.
    pub margin_shape: Vec<serde_json::Value>,
}

/// Rectangle.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rect {
    /// X coordinate.
    pub x: f64,
    /// Y coordinate.
    pub y: f64,
    /// Rectangle width.
    pub width: f64,
    /// Rectangle height.
    pub height: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CSSComputedStyleProperty {
    /// Computed style property name.
    pub name: String,
    /// Computed style property value.
    pub value: String,
}

// ── Inline enums ─────────────────────────────────────────────────────────────

/// Whether to include whitespaces in the children array of returned Nodes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnableIncludeWhitespace {
    #[default]
    #[serde(rename = "none")]
    None,
    #[serde(rename = "all")]
    All,
}

/// Type of relation to get.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum GetElementByRelationRelation {
    #[default]
    PopoverTarget,
    InterestTarget,
    CommandFor,
}

// ── Param types ──────────────────────────────────────────────────────────────

/// Parameters for [`DomCommands::dom_copy_to`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyToParams {
    /// Id of the node to copy.
    pub node_id: NodeId,
    /// Id of the element to drop the copy into.
    pub target_node_id: NodeId,
    /// Drop the copy before this node (if absent, the copy becomes the last child of
    /// `targetNodeId`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insert_before_node_id: Option<NodeId>,
}

/// Parameters for [`DomCommands::dom_describe_node`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DescribeNodeParams {
    /// Identifier of the node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<NodeId>,
    /// Identifier of the backend node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend_node_id: Option<BackendNodeId>,
    /// JavaScript object id of the node wrapper.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_id: Option<RemoteObjectId>,
    /// The maximum depth at which children should be retrieved, defaults to 1. Use -1 for the
    /// entire subtree or provide an integer larger than 0.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth: Option<i64>,
    /// Whether or not iframes and shadow roots should be traversed when returning the subtree
    /// (default is false).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pierce: Option<bool>,
}

/// Parameters for [`DomCommands::dom_scroll_into_view_if_needed`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScrollIntoViewIfNeededParams {
    /// Identifier of the node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<NodeId>,
    /// Identifier of the backend node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend_node_id: Option<BackendNodeId>,
    /// JavaScript object id of the node wrapper.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_id: Option<RemoteObjectId>,
    /// The rect to be scrolled into view, relative to the node's border box, in CSS pixels.
    /// When omitted, center of the node will be used, similar to Element.scrollIntoView.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rect: Option<Rect>,
}

/// Parameters for [`DomCommands::dom_enable`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnableParams {
    /// Whether to include whitespaces in the children array of returned Nodes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_whitespace: Option<EnableIncludeWhitespace>,
}

/// Parameters for [`DomCommands::dom_focus`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FocusParams {
    /// Identifier of the node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<NodeId>,
    /// Identifier of the backend node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend_node_id: Option<BackendNodeId>,
    /// JavaScript object id of the node wrapper.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_id: Option<RemoteObjectId>,
}

/// Parameters for [`DomCommands::dom_get_box_model`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBoxModelParams {
    /// Identifier of the node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<NodeId>,
    /// Identifier of the backend node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend_node_id: Option<BackendNodeId>,
    /// JavaScript object id of the node wrapper.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_id: Option<RemoteObjectId>,
}

/// Parameters for [`DomCommands::dom_get_content_quads`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetContentQuadsParams {
    /// Identifier of the node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<NodeId>,
    /// Identifier of the backend node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend_node_id: Option<BackendNodeId>,
    /// JavaScript object id of the node wrapper.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_id: Option<RemoteObjectId>,
}

/// Parameters for [`DomCommands::dom_get_document`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetDocumentParams {
    /// The maximum depth at which children should be retrieved, defaults to 1. Use -1 for the
    /// entire subtree or provide an integer larger than 0.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth: Option<i64>,
    /// Whether or not iframes and shadow roots should be traversed when returning the subtree
    /// (default is false).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pierce: Option<bool>,
}

/// Parameters for [`DomCommands::dom_get_nodes_for_subtree_by_style`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetNodesForSubtreeByStyleParams {
    /// Node ID pointing to the root of a subtree.
    pub node_id: NodeId,
    /// The style to filter nodes by (includes nodes if any of properties matches).
    pub computed_styles: Vec<CSSComputedStyleProperty>,
    /// Whether or not iframes and shadow roots in the same target should be traversed when returning the
    /// results (default is false).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pierce: Option<bool>,
}

/// Parameters for [`DomCommands::dom_get_node_for_location`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetNodeForLocationParams {
    /// X coordinate.
    pub x: i64,
    /// Y coordinate.
    pub y: i64,
    /// False to skip to the nearest non-UA shadow root ancestor (default: false).
    #[serde(rename = "includeUserAgentShadowDOM")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_user_agent_shadow_dom: Option<bool>,
    /// Whether to ignore pointer-events: none on elements and hit test them.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore_pointer_events_none: Option<bool>,
}

/// Parameters for [`DomCommands::dom_get_outer_html`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetOuterHTMLParams {
    /// Identifier of the node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<NodeId>,
    /// Identifier of the backend node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend_node_id: Option<BackendNodeId>,
    /// JavaScript object id of the node wrapper.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_id: Option<RemoteObjectId>,
    /// Include all shadow roots. Equals to false if not specified.
    #[serde(rename = "includeShadowDOM")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_shadow_dom: Option<bool>,
}

/// Parameters for [`DomCommands::dom_get_search_results`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSearchResultsParams {
    /// Unique search session identifier.
    pub search_id: String,
    /// Start index of the search result to be returned.
    pub from_index: i64,
    /// End index of the search result to be returned.
    pub to_index: i64,
}

/// Parameters for [`DomCommands::dom_move_to`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveToParams {
    /// Id of the node to move.
    pub node_id: NodeId,
    /// Id of the element to drop the moved node into.
    pub target_node_id: NodeId,
    /// Drop node before this one (if absent, the moved node becomes the last child of
    /// `targetNodeId`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insert_before_node_id: Option<NodeId>,
}

/// Parameters for [`DomCommands::dom_perform_search`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformSearchParams {
    /// Plain text or query selector or XPath search query.
    pub query: String,
    /// True to search in user agent shadow DOM.
    #[serde(rename = "includeUserAgentShadowDOM")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_user_agent_shadow_dom: Option<bool>,
}

/// Parameters for [`DomCommands::dom_get_element_by_relation`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetElementByRelationParams {
    /// Id of the node from which to query the relation.
    pub node_id: NodeId,
    /// Type of relation to get.
    pub relation: GetElementByRelationRelation,
}

/// Parameters for [`DomCommands::dom_request_child_nodes`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestChildNodesParams {
    /// Id of the node to get children for.
    pub node_id: NodeId,
    /// The maximum depth at which children should be retrieved, defaults to 1. Use -1 for the
    /// entire subtree or provide an integer larger than 0.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth: Option<i64>,
    /// Whether or not iframes and shadow roots should be traversed when returning the sub-tree
    /// (default is false).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pierce: Option<bool>,
}

/// Parameters for [`DomCommands::dom_resolve_node`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveNodeParams {
    /// Id of the node to resolve.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<NodeId>,
    /// Backend identifier of the node to resolve.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend_node_id: Option<BackendNodeId>,
    /// Symbolic group name that can be used to release multiple objects.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_group: Option<String>,
    /// Execution context in which to resolve the node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_context_id: Option<ExecutionContextId>,
}

/// Parameters for [`DomCommands::dom_set_attribute_value`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetAttributeValueParams {
    /// Id of the element to set attribute for.
    pub node_id: NodeId,
    /// Attribute name.
    pub name: String,
    /// Attribute value.
    pub value: String,
}

/// Parameters for [`DomCommands::dom_set_attributes_as_text`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetAttributesAsTextParams {
    /// Id of the element to set attributes for.
    pub node_id: NodeId,
    /// Text with a number of attributes. Will parse this text using HTML parser.
    pub text: String,
    /// Attribute name to replace with new attributes derived from text in case text parsed
    /// successfully.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Parameters for [`DomCommands::dom_set_file_input_files`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetFileInputFilesParams {
    /// Array of file paths to set.
    pub files: Vec<String>,
    /// Identifier of the node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<NodeId>,
    /// Identifier of the backend node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend_node_id: Option<BackendNodeId>,
    /// JavaScript object id of the node wrapper.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_id: Option<RemoteObjectId>,
}

/// Parameters for [`DomCommands::dom_get_container_for_node`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetContainerForNodeParams {
    pub node_id: NodeId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub physical_axes: Option<PhysicalAxes>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logical_axes: Option<LogicalAxes>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queries_scroll_state: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queries_anchored: Option<bool>,
}

/// Parameters for [`DomCommands::dom_get_anchor_element`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetAnchorElementParams {
    /// Id of the positioned element from which to find the anchor.
    pub node_id: NodeId,
    /// An optional anchor specifier, as defined in
    /// https://www.w3.org/TR/css-anchor-position-1/#anchor-specifier.
    /// If not provided, it will return the implicit anchor element for
    /// the given positioned element.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor_specifier: Option<String>,
}

/// Parameters for [`DomCommands::dom_force_show_popover`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForceShowPopoverParams {
    /// Id of the popover HTMLElement.
    pub node_id: NodeId,
    /// If true, opens the popover and keeps it open. If false, closes the
    /// popover if it was previously force-opened.
    pub enable: bool,
    /// Optional ID of the element invoking this popover, used to establish the implicit anchor.
    /// If not provided, it will fall back to the first invoker in the document, preferring
    /// elements with a popovertarget attribute over those with a commandfor attribute. Note that
    /// if there are multiple invokers, this is just an estimate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoker_node_id: Option<BackendNodeId>,
}

// ── Return types ─────────────────────────────────────────────────────────────

/// Return type for [`DomCommands::dom_collect_class_names_from_subtree`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectClassNamesFromSubtreeReturn {
    /// Class name list.
    pub class_names: Vec<String>,
}

/// Return type for [`DomCommands::dom_copy_to`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyToReturn {
    /// Id of the node clone.
    pub node_id: NodeId,
}

/// Return type for [`DomCommands::dom_describe_node`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DescribeNodeReturn {
    /// Node description.
    pub node: Node,
}

/// Return type for [`DomCommands::dom_get_attributes`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetAttributesReturn {
    /// An interleaved array of node attribute names and values.
    pub attributes: Vec<String>,
}

/// Return type for [`DomCommands::dom_get_box_model`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBoxModelReturn {
    /// Box model for the node.
    pub model: BoxModel,
}

/// Return type for [`DomCommands::dom_get_content_quads`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetContentQuadsReturn {
    /// Quads that describe node layout relative to viewport.
    pub quads: Vec<Quad>,
}

/// Return type for [`DomCommands::dom_get_document`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetDocumentReturn {
    /// Resulting node.
    pub root: Node,
}

/// Return type for [`DomCommands::dom_get_nodes_for_subtree_by_style`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetNodesForSubtreeByStyleReturn {
    /// Resulting nodes.
    pub node_ids: Vec<NodeId>,
}

/// Return type for [`DomCommands::dom_get_node_for_location`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetNodeForLocationReturn {
    /// Resulting node.
    pub backend_node_id: BackendNodeId,
    /// Frame this node belongs to.
    pub frame_id: FrameId,
    /// Id of the node at given coordinates, only when enabled and requested document.
    #[serde(default)]
    pub node_id: Option<NodeId>,
}

/// Return type for [`DomCommands::dom_get_outer_html`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetOuterHTMLReturn {
    /// Outer HTML markup.
    #[serde(rename = "outerHTML")]
    pub outer_html: String,
}

/// Return type for [`DomCommands::dom_get_relayout_boundary`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetRelayoutBoundaryReturn {
    /// Relayout boundary node id for the given node.
    pub node_id: NodeId,
}

/// Return type for [`DomCommands::dom_get_search_results`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSearchResultsReturn {
    /// Ids of the search result nodes.
    pub node_ids: Vec<NodeId>,
}

/// Return type for [`DomCommands::dom_move_to`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveToReturn {
    /// New id of the moved node.
    pub node_id: NodeId,
}

/// Return type for [`DomCommands::dom_perform_search`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformSearchReturn {
    /// Unique search session identifier.
    pub search_id: String,
    /// Number of search results.
    pub result_count: i64,
}

/// Return type for [`DomCommands::dom_push_node_by_path_to_frontend`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PushNodeByPathToFrontendReturn {
    /// Id of the node for given path.
    pub node_id: NodeId,
}

/// Return type for [`DomCommands::dom_push_nodes_by_backend_ids_to_frontend`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PushNodesByBackendIdsToFrontendReturn {
    /// The array of ids of pushed nodes that correspond to the backend ids specified in
    /// backendNodeIds.
    pub node_ids: Vec<NodeId>,
}

/// Return type for [`DomCommands::dom_query_selector`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuerySelectorReturn {
    /// Query selector result.
    pub node_id: NodeId,
}

/// Return type for [`DomCommands::dom_query_selector_all`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuerySelectorAllReturn {
    /// Query selector result.
    pub node_ids: Vec<NodeId>,
}

/// Return type for [`DomCommands::dom_get_top_layer_elements`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTopLayerElementsReturn {
    /// NodeIds of top layer elements.
    pub node_ids: Vec<NodeId>,
}

/// Return type for [`DomCommands::dom_get_element_by_relation`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetElementByRelationReturn {
    /// NodeId of the element matching the queried relation.
    pub node_id: NodeId,
}

/// Return type for [`DomCommands::dom_request_node`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestNodeReturn {
    /// Node id for given object.
    pub node_id: NodeId,
}

/// Return type for [`DomCommands::dom_resolve_node`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveNodeReturn {
    /// JavaScript object wrapper for given node.
    pub object: RemoteObject,
}

/// Return type for [`DomCommands::dom_get_node_stack_traces`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetNodeStackTracesReturn {
    /// Creation stack trace, if available.
    #[serde(default)]
    pub creation: Option<StackTrace>,
}

/// Return type for [`DomCommands::dom_get_file_info`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetFileInfoReturn {
    pub path: String,
}

/// Return type for [`DomCommands::dom_get_detached_dom_nodes`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetDetachedDomNodesReturn {
    /// The list of detached nodes.
    pub detached_nodes: Vec<DetachedElementInfo>,
}

/// Return type for [`DomCommands::dom_set_node_name`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetNodeNameReturn {
    /// New node's id.
    pub node_id: NodeId,
}

/// Return type for [`DomCommands::dom_get_frame_owner`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetFrameOwnerReturn {
    /// Resulting node.
    pub backend_node_id: BackendNodeId,
    /// Id of the node at given coordinates, only when enabled and requested document.
    #[serde(default)]
    pub node_id: Option<NodeId>,
}

/// Return type for [`DomCommands::dom_get_container_for_node`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetContainerForNodeReturn {
    /// The container node for the given node, or null if not found.
    #[serde(default)]
    pub node_id: Option<NodeId>,
}

/// Return type for [`DomCommands::dom_get_querying_descendants_for_container`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetQueryingDescendantsForContainerReturn {
    /// Descendant nodes with container queries against the given container.
    pub node_ids: Vec<NodeId>,
}

/// Return type for [`DomCommands::dom_get_anchor_element`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetAnchorElementReturn {
    /// The anchor element of the given anchor query.
    pub node_id: NodeId,
}

/// Return type for [`DomCommands::dom_force_show_popover`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForceShowPopoverReturn {
    /// List of popovers that were closed in order to respect popover stacking order.
    pub node_ids: Vec<NodeId>,
}

// ── Events ───────────────────────────────────────────────────────────────────

/// Fired when `Element`'s attribute is modified.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttributeModifiedEvent {
    /// Id of the node that has changed.
    pub node_id: NodeId,
    /// Attribute name.
    pub name: String,
    /// Attribute value.
    pub value: String,
}

/// Fired when `Element`'s adoptedStyleSheets are modified.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdoptedStyleSheetsModifiedEvent {
    /// Id of the node that has changed.
    pub node_id: NodeId,
    /// New adoptedStyleSheets array.
    pub adopted_style_sheets: Vec<StyleSheetId>,
}

/// Fired when `Element`'s attribute is removed.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttributeRemovedEvent {
    /// Id of the node that has changed.
    pub node_id: NodeId,
    /// A ttribute name.
    pub name: String,
}

/// Mirrors `DOMCharacterDataModified` event.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterDataModifiedEvent {
    /// Id of the node that has changed.
    pub node_id: NodeId,
    /// New text value.
    pub character_data: String,
}

/// Fired when `Container`'s child node count has changed.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChildNodeCountUpdatedEvent {
    /// Id of the node that has changed.
    pub node_id: NodeId,
    /// New node count.
    pub child_node_count: i64,
}

/// Mirrors `DOMNodeInserted` event.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChildNodeInsertedEvent {
    /// Id of the node that has changed.
    pub parent_node_id: NodeId,
    /// Id of the previous sibling.
    pub previous_node_id: NodeId,
    /// Inserted node data.
    pub node: Node,
}

/// Mirrors `DOMNodeRemoved` event.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChildNodeRemovedEvent {
    /// Parent id.
    pub parent_node_id: NodeId,
    /// Id of the node that has been removed.
    pub node_id: NodeId,
}

/// Called when distribution is changed.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributedNodesUpdatedEvent {
    /// Insertion point where distributed nodes were updated.
    pub insertion_point_id: NodeId,
    /// Distributed nodes for given insertion point.
    pub distributed_nodes: Vec<BackendNode>,
}

/// Fired when `Document` has been totally updated. Node ids are no longer valid.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentUpdatedEvent {}

/// Fired when `Element`'s inline style is modified via a CSS property modification.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InlineStyleInvalidatedEvent {
    /// Ids of the nodes for which the inline styles have been invalidated.
    pub node_ids: Vec<NodeId>,
}

/// Called when a pseudo element is added to an element.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PseudoElementAddedEvent {
    /// Pseudo element's parent element id.
    pub parent_id: NodeId,
    /// The added pseudo element.
    pub pseudo_element: Node,
}

/// Called when top layer elements are changed.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TopLayerElementsUpdatedEvent {}

/// Fired when a node's scrollability state changes.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScrollableFlagUpdatedEvent {
    /// The id of the node.
    pub node_id: NodeId,
    /// If the node is scrollable.
    pub is_scrollable: bool,
}

/// Fired when a node's ad related state changes.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdRelatedStateUpdatedEvent {
    /// The id of the node.
    pub node_id: NodeId,
    /// The provenance of the ad related node, if it is ad related.
    #[serde(default)]
    pub ad_provenance: Option<AdProvenance>,
}

/// Fired when a node's starting styles changes.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AffectedByStartingStylesFlagUpdatedEvent {
    /// The id of the node.
    pub node_id: NodeId,
    /// If the node has starting styles.
    pub affected_by_starting_styles: bool,
}

/// Called when a pseudo element is removed from an element.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PseudoElementRemovedEvent {
    /// Pseudo element's parent element id.
    pub parent_id: NodeId,
    /// The removed pseudo element id.
    pub pseudo_element_id: NodeId,
}

/// Fired when backend wants to provide client with the missing DOM structure. This happens upon
/// most of the calls requesting node ids.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetChildNodesEvent {
    /// Parent node id to populate with children.
    pub parent_id: NodeId,
    /// Child nodes array.
    pub nodes: Vec<Node>,
}

/// Called when shadow root is popped from the element.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShadowRootPoppedEvent {
    /// Host element id.
    pub host_id: NodeId,
    /// Shadow root id.
    pub root_id: NodeId,
}

/// Called when shadow root is pushed into the element.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShadowRootPushedEvent {
    /// Host element id.
    pub host_id: NodeId,
    /// Shadow root.
    pub root: Node,
}

// ── Domain trait ─────────────────────────────────────────────────────────────

/// `DOM` domain CDP methods.
///
/// This domain exposes DOM read/write operations. Each DOM Node is represented with its mirror object
/// that has an `id`. This `id` can be used to get additional information on the Node, resolve it into
/// the JavaScript object wrapper, etc. It is important that client receives DOM events only for the
/// nodes that are known to the client. Backend keeps track of the nodes that were sent to the client
/// and never sends the same node twice. It is client's responsibility to collect information about
/// the nodes that were sent to the client. Note that `iframe` owner elements will return
/// corresponding document elements as their child nodes.
///
/// Reference: <https://chromedevtools.github.io/devtools-protocol/tot/DOM/>
pub trait DomCommands {
    /// Collects class names for the node with given id and all of it's child nodes.
    ///
    /// CDP: `DOM.collectClassNamesFromSubtree`
    async fn dom_collect_class_names_from_subtree(
        &self,
        node_id: &NodeId,
    ) -> Result<CollectClassNamesFromSubtreeReturn>;

    /// Creates a deep copy of the specified node and places it into the target container before the
    /// given anchor.
    ///
    /// CDP: `DOM.copyTo`
    async fn dom_copy_to(&self, params: &CopyToParams) -> Result<CopyToReturn>;

    /// Describes node given its id, does not require domain to be enabled. Does not start tracking any
    /// objects, can be used for automation.
    ///
    /// CDP: `DOM.describeNode`
    async fn dom_describe_node(&self, params: &DescribeNodeParams) -> Result<DescribeNodeReturn>;

    /// Scrolls the specified rect of the given node into view if not already visible.
    /// Note: exactly one between nodeId, backendNodeId and objectId should be passed
    /// to identify the node.
    ///
    /// CDP: `DOM.scrollIntoViewIfNeeded`
    async fn dom_scroll_into_view_if_needed(
        &self,
        params: &ScrollIntoViewIfNeededParams,
    ) -> Result<()>;

    /// Disables DOM agent for the given page.
    ///
    /// CDP: `DOM.disable`
    async fn dom_disable(&self) -> Result<()>;

    /// Discards search results from the session with the given id. `getSearchResults` should no longer
    /// be called for that search.
    ///
    /// CDP: `DOM.discardSearchResults`
    async fn dom_discard_search_results(&self, search_id: &str) -> Result<()>;

    /// Enables DOM agent for the given page.
    ///
    /// CDP: `DOM.enable`
    async fn dom_enable(&self, params: &EnableParams) -> Result<()>;

    /// Focuses the given element.
    ///
    /// CDP: `DOM.focus`
    async fn dom_focus(&self, params: &FocusParams) -> Result<()>;

    /// Returns attributes for the specified node.
    ///
    /// CDP: `DOM.getAttributes`
    async fn dom_get_attributes(&self, node_id: &NodeId) -> Result<GetAttributesReturn>;

    /// Returns boxes for the given node.
    ///
    /// CDP: `DOM.getBoxModel`
    async fn dom_get_box_model(&self, params: &GetBoxModelParams) -> Result<GetBoxModelReturn>;

    /// Returns quads that describe node position on the page. This method
    /// might return multiple quads for inline nodes.
    ///
    /// CDP: `DOM.getContentQuads`
    async fn dom_get_content_quads(
        &self,
        params: &GetContentQuadsParams,
    ) -> Result<GetContentQuadsReturn>;

    /// Returns the root DOM node (and optionally the subtree) to the caller.
    /// Implicitly enables the DOM domain events for the current target.
    ///
    /// CDP: `DOM.getDocument`
    async fn dom_get_document(&self, params: &GetDocumentParams) -> Result<GetDocumentReturn>;

    /// Finds nodes with a given computed style in a subtree.
    ///
    /// CDP: `DOM.getNodesForSubtreeByStyle`
    async fn dom_get_nodes_for_subtree_by_style(
        &self,
        params: &GetNodesForSubtreeByStyleParams,
    ) -> Result<GetNodesForSubtreeByStyleReturn>;

    /// Returns node id at given location. Depending on whether DOM domain is enabled, nodeId is
    /// either returned or not.
    ///
    /// CDP: `DOM.getNodeForLocation`
    async fn dom_get_node_for_location(
        &self,
        params: &GetNodeForLocationParams,
    ) -> Result<GetNodeForLocationReturn>;

    /// Returns node's HTML markup.
    ///
    /// CDP: `DOM.getOuterHTML`
    async fn dom_get_outer_html(&self, params: &GetOuterHTMLParams) -> Result<GetOuterHTMLReturn>;

    /// Returns the id of the nearest ancestor that is a relayout boundary.
    ///
    /// CDP: `DOM.getRelayoutBoundary`
    async fn dom_get_relayout_boundary(
        &self,
        node_id: &NodeId,
    ) -> Result<GetRelayoutBoundaryReturn>;

    /// Returns search results from given `fromIndex` to given `toIndex` from the search with the given
    /// identifier.
    ///
    /// CDP: `DOM.getSearchResults`
    async fn dom_get_search_results(
        &self,
        params: &GetSearchResultsParams,
    ) -> Result<GetSearchResultsReturn>;

    /// Hides any highlight.
    ///
    /// CDP: `DOM.hideHighlight`
    async fn dom_hide_highlight(&self) -> Result<()>;

    /// Highlights DOM node.
    ///
    /// CDP: `DOM.highlightNode`
    async fn dom_highlight_node(&self) -> Result<()>;

    /// Highlights given rectangle.
    ///
    /// CDP: `DOM.highlightRect`
    async fn dom_highlight_rect(&self) -> Result<()>;

    /// Marks last undoable state.
    ///
    /// CDP: `DOM.markUndoableState`
    async fn dom_mark_undoable_state(&self) -> Result<()>;

    /// Moves node into the new container, places it before the given anchor.
    ///
    /// CDP: `DOM.moveTo`
    async fn dom_move_to(&self, params: &MoveToParams) -> Result<MoveToReturn>;

    /// Searches for a given string in the DOM tree. Use `getSearchResults` to access search results or
    /// `cancelSearch` to end this search session.
    ///
    /// CDP: `DOM.performSearch`
    async fn dom_perform_search(&self, params: &PerformSearchParams)
    -> Result<PerformSearchReturn>;

    /// Requests that the node is sent to the caller given its path. // FIXME, use XPath.
    ///
    /// CDP: `DOM.pushNodeByPathToFrontend`
    async fn dom_push_node_by_path_to_frontend(
        &self,
        path: &str,
    ) -> Result<PushNodeByPathToFrontendReturn>;

    /// Requests that a batch of nodes is sent to the caller given their backend node ids.
    ///
    /// CDP: `DOM.pushNodesByBackendIdsToFrontend`
    async fn dom_push_nodes_by_backend_ids_to_frontend(
        &self,
        backend_node_ids: &[BackendNodeId],
    ) -> Result<PushNodesByBackendIdsToFrontendReturn>;

    /// Executes `querySelector` on a given node.
    ///
    /// CDP: `DOM.querySelector`
    async fn dom_query_selector(
        &self,
        node_id: &NodeId,
        selector: &str,
    ) -> Result<QuerySelectorReturn>;

    /// Executes `querySelectorAll` on a given node.
    ///
    /// CDP: `DOM.querySelectorAll`
    async fn dom_query_selector_all(
        &self,
        node_id: &NodeId,
        selector: &str,
    ) -> Result<QuerySelectorAllReturn>;

    /// Returns NodeIds of current top layer elements.
    /// Top layer is rendered closest to the user within a viewport, therefore its elements always
    /// appear on top of all other content.
    ///
    /// CDP: `DOM.getTopLayerElements`
    async fn dom_get_top_layer_elements(&self) -> Result<GetTopLayerElementsReturn>;

    /// Returns the NodeId of the matched element according to certain relations.
    ///
    /// CDP: `DOM.getElementByRelation`
    async fn dom_get_element_by_relation(
        &self,
        params: &GetElementByRelationParams,
    ) -> Result<GetElementByRelationReturn>;

    /// Re-does the last undone action.
    ///
    /// CDP: `DOM.redo`
    async fn dom_redo(&self) -> Result<()>;

    /// Removes attribute with given name from an element with given id.
    ///
    /// CDP: `DOM.removeAttribute`
    async fn dom_remove_attribute(&self, node_id: &NodeId, name: &str) -> Result<()>;

    /// Removes node with given id.
    ///
    /// CDP: `DOM.removeNode`
    async fn dom_remove_node(&self, node_id: &NodeId) -> Result<()>;

    /// Requests that children of the node with given id are returned to the caller in form of
    /// `setChildNodes` events where not only immediate children are retrieved, but all children down to
    /// the specified depth.
    ///
    /// CDP: `DOM.requestChildNodes`
    async fn dom_request_child_nodes(&self, params: &RequestChildNodesParams) -> Result<()>;

    /// Requests that the node is sent to the caller given the JavaScript node object reference. All
    /// nodes that form the path from the node to the root are also sent to the client as a series of
    /// `setChildNodes` notifications.
    ///
    /// CDP: `DOM.requestNode`
    async fn dom_request_node(&self, object_id: &RemoteObjectId) -> Result<RequestNodeReturn>;

    /// Resolves the JavaScript node object for a given NodeId or BackendNodeId.
    ///
    /// CDP: `DOM.resolveNode`
    async fn dom_resolve_node(&self, params: &ResolveNodeParams) -> Result<ResolveNodeReturn>;

    /// Sets attribute for an element with given id.
    ///
    /// CDP: `DOM.setAttributeValue`
    async fn dom_set_attribute_value(&self, params: &SetAttributeValueParams) -> Result<()>;

    /// Sets attributes on element with given id. This method is useful when user edits some existing
    /// attribute value and types in several attribute name/value pairs.
    ///
    /// CDP: `DOM.setAttributesAsText`
    async fn dom_set_attributes_as_text(&self, params: &SetAttributesAsTextParams) -> Result<()>;

    /// Sets files for the given file input element.
    ///
    /// CDP: `DOM.setFileInputFiles`
    async fn dom_set_file_input_files(&self, params: &SetFileInputFilesParams) -> Result<()>;

    /// Sets if stack traces should be captured for Nodes. See `Node.getNodeStackTraces`. Default is disabled.
    ///
    /// CDP: `DOM.setNodeStackTracesEnabled`
    async fn dom_set_node_stack_traces_enabled(&self, enable: bool) -> Result<()>;

    /// Gets stack traces associated with a Node. As of now, only provides stack trace for Node creation.
    ///
    /// CDP: `DOM.getNodeStackTraces`
    async fn dom_get_node_stack_traces(&self, node_id: &NodeId)
    -> Result<GetNodeStackTracesReturn>;

    /// Returns file information for the given
    /// File wrapper.
    ///
    /// CDP: `DOM.getFileInfo`
    async fn dom_get_file_info(&self, object_id: &RemoteObjectId) -> Result<GetFileInfoReturn>;

    /// Returns list of detached nodes.
    ///
    /// CDP: `DOM.getDetachedDomNodes`
    async fn dom_get_detached_dom_nodes(&self) -> Result<GetDetachedDomNodesReturn>;

    /// Enables console to refer to the node with given id via $x (see Command Line API for more details
    /// $x functions).
    ///
    /// CDP: `DOM.setInspectedNode`
    async fn dom_set_inspected_node(&self, node_id: &NodeId) -> Result<()>;

    /// Sets node name for a node with given id.
    ///
    /// CDP: `DOM.setNodeName`
    async fn dom_set_node_name(&self, node_id: &NodeId, name: &str) -> Result<SetNodeNameReturn>;

    /// Sets node value for a node with given id.
    ///
    /// CDP: `DOM.setNodeValue`
    async fn dom_set_node_value(&self, node_id: &NodeId, value: &str) -> Result<()>;

    /// Sets node HTML markup, returns new node id.
    ///
    /// CDP: `DOM.setOuterHTML`
    async fn dom_set_outer_html(&self, node_id: &NodeId, outer_html: &str) -> Result<()>;

    /// Undoes the last performed action.
    ///
    /// CDP: `DOM.undo`
    async fn dom_undo(&self) -> Result<()>;

    /// Returns iframe node that owns iframe with the given domain.
    ///
    /// CDP: `DOM.getFrameOwner`
    async fn dom_get_frame_owner(&self, frame_id: &FrameId) -> Result<GetFrameOwnerReturn>;

    /// Returns the query container of the given node based on container query
    /// conditions: containerName, physical and logical axes, and whether it queries
    /// scroll-state or anchored elements. If no axes are provided and
    /// queriesScrollState is false, the style container is returned, which is the
    /// direct parent or the closest element with a matching container-name.
    ///
    /// CDP: `DOM.getContainerForNode`
    async fn dom_get_container_for_node(
        &self,
        params: &GetContainerForNodeParams,
    ) -> Result<GetContainerForNodeReturn>;

    /// Returns the descendants of a container query container that have
    /// container queries against this container.
    ///
    /// CDP: `DOM.getQueryingDescendantsForContainer`
    async fn dom_get_querying_descendants_for_container(
        &self,
        node_id: &NodeId,
    ) -> Result<GetQueryingDescendantsForContainerReturn>;

    /// Returns the target anchor element of the given anchor query according to
    /// https://www.w3.org/TR/css-anchor-position-1/#target.
    ///
    /// CDP: `DOM.getAnchorElement`
    async fn dom_get_anchor_element(
        &self,
        params: &GetAnchorElementParams,
    ) -> Result<GetAnchorElementReturn>;

    /// When enabling, this API force-opens the popover identified by nodeId
    /// and keeps it open until disabled.
    ///
    /// CDP: `DOM.forceShowPopover`
    async fn dom_force_show_popover(
        &self,
        params: &ForceShowPopoverParams,
    ) -> Result<ForceShowPopoverReturn>;
}

// ── Impl ─────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CollectClassNamesFromSubtreeInternalParams<'a> {
    node_id: &'a NodeId,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DiscardSearchResultsInternalParams<'a> {
    search_id: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GetAttributesInternalParams<'a> {
    node_id: &'a NodeId,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GetRelayoutBoundaryInternalParams<'a> {
    node_id: &'a NodeId,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PushNodeByPathToFrontendInternalParams<'a> {
    path: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PushNodesByBackendIdsToFrontendInternalParams<'a> {
    backend_node_ids: &'a [BackendNodeId],
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct QuerySelectorInternalParams<'a> {
    node_id: &'a NodeId,
    selector: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct QuerySelectorAllInternalParams<'a> {
    node_id: &'a NodeId,
    selector: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RemoveAttributeInternalParams<'a> {
    node_id: &'a NodeId,
    name: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RemoveNodeInternalParams<'a> {
    node_id: &'a NodeId,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RequestNodeInternalParams<'a> {
    object_id: &'a RemoteObjectId,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetNodeStackTracesEnabledInternalParams {
    enable: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GetNodeStackTracesInternalParams<'a> {
    node_id: &'a NodeId,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GetFileInfoInternalParams<'a> {
    object_id: &'a RemoteObjectId,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetInspectedNodeInternalParams<'a> {
    node_id: &'a NodeId,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetNodeNameInternalParams<'a> {
    node_id: &'a NodeId,
    name: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetNodeValueInternalParams<'a> {
    node_id: &'a NodeId,
    value: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetOuterHTMLInternalParams<'a> {
    node_id: &'a NodeId,
    outer_html: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GetFrameOwnerInternalParams<'a> {
    frame_id: &'a FrameId,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GetQueryingDescendantsForContainerInternalParams<'a> {
    node_id: &'a NodeId,
}

impl DomCommands for CdpSession {
    async fn dom_collect_class_names_from_subtree(
        &self,
        node_id: &NodeId,
    ) -> Result<CollectClassNamesFromSubtreeReturn> {
        let params = CollectClassNamesFromSubtreeInternalParams { node_id };
        self.call("DOM.collectClassNamesFromSubtree", &params).await
    }

    async fn dom_copy_to(&self, params: &CopyToParams) -> Result<CopyToReturn> {
        self.call("DOM.copyTo", params).await
    }

    async fn dom_describe_node(&self, params: &DescribeNodeParams) -> Result<DescribeNodeReturn> {
        self.call("DOM.describeNode", params).await
    }

    async fn dom_scroll_into_view_if_needed(
        &self,
        params: &ScrollIntoViewIfNeededParams,
    ) -> Result<()> {
        self.call_no_response("DOM.scrollIntoViewIfNeeded", params)
            .await
    }

    async fn dom_disable(&self) -> Result<()> {
        self.call_no_response("DOM.disable", &serde_json::json!({}))
            .await
    }

    async fn dom_discard_search_results(&self, search_id: &str) -> Result<()> {
        let params = DiscardSearchResultsInternalParams { search_id };
        self.call_no_response("DOM.discardSearchResults", &params)
            .await
    }

    async fn dom_enable(&self, params: &EnableParams) -> Result<()> {
        self.call_no_response("DOM.enable", params).await
    }

    async fn dom_focus(&self, params: &FocusParams) -> Result<()> {
        self.call_no_response("DOM.focus", params).await
    }

    async fn dom_get_attributes(&self, node_id: &NodeId) -> Result<GetAttributesReturn> {
        let params = GetAttributesInternalParams { node_id };
        self.call("DOM.getAttributes", &params).await
    }

    async fn dom_get_box_model(&self, params: &GetBoxModelParams) -> Result<GetBoxModelReturn> {
        self.call("DOM.getBoxModel", params).await
    }

    async fn dom_get_content_quads(
        &self,
        params: &GetContentQuadsParams,
    ) -> Result<GetContentQuadsReturn> {
        self.call("DOM.getContentQuads", params).await
    }

    async fn dom_get_document(&self, params: &GetDocumentParams) -> Result<GetDocumentReturn> {
        self.call("DOM.getDocument", params).await
    }

    async fn dom_get_nodes_for_subtree_by_style(
        &self,
        params: &GetNodesForSubtreeByStyleParams,
    ) -> Result<GetNodesForSubtreeByStyleReturn> {
        self.call("DOM.getNodesForSubtreeByStyle", params).await
    }

    async fn dom_get_node_for_location(
        &self,
        params: &GetNodeForLocationParams,
    ) -> Result<GetNodeForLocationReturn> {
        self.call("DOM.getNodeForLocation", params).await
    }

    async fn dom_get_outer_html(&self, params: &GetOuterHTMLParams) -> Result<GetOuterHTMLReturn> {
        self.call("DOM.getOuterHTML", params).await
    }

    async fn dom_get_relayout_boundary(
        &self,
        node_id: &NodeId,
    ) -> Result<GetRelayoutBoundaryReturn> {
        let params = GetRelayoutBoundaryInternalParams { node_id };
        self.call("DOM.getRelayoutBoundary", &params).await
    }

    async fn dom_get_search_results(
        &self,
        params: &GetSearchResultsParams,
    ) -> Result<GetSearchResultsReturn> {
        self.call("DOM.getSearchResults", params).await
    }

    async fn dom_hide_highlight(&self) -> Result<()> {
        self.call_no_response("DOM.hideHighlight", &serde_json::json!({}))
            .await
    }

    async fn dom_highlight_node(&self) -> Result<()> {
        self.call_no_response("DOM.highlightNode", &serde_json::json!({}))
            .await
    }

    async fn dom_highlight_rect(&self) -> Result<()> {
        self.call_no_response("DOM.highlightRect", &serde_json::json!({}))
            .await
    }

    async fn dom_mark_undoable_state(&self) -> Result<()> {
        self.call_no_response("DOM.markUndoableState", &serde_json::json!({}))
            .await
    }

    async fn dom_move_to(&self, params: &MoveToParams) -> Result<MoveToReturn> {
        self.call("DOM.moveTo", params).await
    }

    async fn dom_perform_search(
        &self,
        params: &PerformSearchParams,
    ) -> Result<PerformSearchReturn> {
        self.call("DOM.performSearch", params).await
    }

    async fn dom_push_node_by_path_to_frontend(
        &self,
        path: &str,
    ) -> Result<PushNodeByPathToFrontendReturn> {
        let params = PushNodeByPathToFrontendInternalParams { path };
        self.call("DOM.pushNodeByPathToFrontend", &params).await
    }

    async fn dom_push_nodes_by_backend_ids_to_frontend(
        &self,
        backend_node_ids: &[BackendNodeId],
    ) -> Result<PushNodesByBackendIdsToFrontendReturn> {
        let params = PushNodesByBackendIdsToFrontendInternalParams { backend_node_ids };
        self.call("DOM.pushNodesByBackendIdsToFrontend", &params)
            .await
    }

    async fn dom_query_selector(
        &self,
        node_id: &NodeId,
        selector: &str,
    ) -> Result<QuerySelectorReturn> {
        let params = QuerySelectorInternalParams { node_id, selector };
        self.call("DOM.querySelector", &params).await
    }

    async fn dom_query_selector_all(
        &self,
        node_id: &NodeId,
        selector: &str,
    ) -> Result<QuerySelectorAllReturn> {
        let params = QuerySelectorAllInternalParams { node_id, selector };
        self.call("DOM.querySelectorAll", &params).await
    }

    async fn dom_get_top_layer_elements(&self) -> Result<GetTopLayerElementsReturn> {
        self.call("DOM.getTopLayerElements", &serde_json::json!({}))
            .await
    }

    async fn dom_get_element_by_relation(
        &self,
        params: &GetElementByRelationParams,
    ) -> Result<GetElementByRelationReturn> {
        self.call("DOM.getElementByRelation", params).await
    }

    async fn dom_redo(&self) -> Result<()> {
        self.call_no_response("DOM.redo", &serde_json::json!({}))
            .await
    }

    async fn dom_remove_attribute(&self, node_id: &NodeId, name: &str) -> Result<()> {
        let params = RemoveAttributeInternalParams { node_id, name };
        self.call_no_response("DOM.removeAttribute", &params).await
    }

    async fn dom_remove_node(&self, node_id: &NodeId) -> Result<()> {
        let params = RemoveNodeInternalParams { node_id };
        self.call_no_response("DOM.removeNode", &params).await
    }

    async fn dom_request_child_nodes(&self, params: &RequestChildNodesParams) -> Result<()> {
        self.call_no_response("DOM.requestChildNodes", params).await
    }

    async fn dom_request_node(&self, object_id: &RemoteObjectId) -> Result<RequestNodeReturn> {
        let params = RequestNodeInternalParams { object_id };
        self.call("DOM.requestNode", &params).await
    }

    async fn dom_resolve_node(&self, params: &ResolveNodeParams) -> Result<ResolveNodeReturn> {
        self.call("DOM.resolveNode", params).await
    }

    async fn dom_set_attribute_value(&self, params: &SetAttributeValueParams) -> Result<()> {
        self.call_no_response("DOM.setAttributeValue", params).await
    }

    async fn dom_set_attributes_as_text(&self, params: &SetAttributesAsTextParams) -> Result<()> {
        self.call_no_response("DOM.setAttributesAsText", params)
            .await
    }

    async fn dom_set_file_input_files(&self, params: &SetFileInputFilesParams) -> Result<()> {
        self.call_no_response("DOM.setFileInputFiles", params).await
    }

    async fn dom_set_node_stack_traces_enabled(&self, enable: bool) -> Result<()> {
        let params = SetNodeStackTracesEnabledInternalParams { enable };
        self.call_no_response("DOM.setNodeStackTracesEnabled", &params)
            .await
    }

    async fn dom_get_node_stack_traces(
        &self,
        node_id: &NodeId,
    ) -> Result<GetNodeStackTracesReturn> {
        let params = GetNodeStackTracesInternalParams { node_id };
        self.call("DOM.getNodeStackTraces", &params).await
    }

    async fn dom_get_file_info(&self, object_id: &RemoteObjectId) -> Result<GetFileInfoReturn> {
        let params = GetFileInfoInternalParams { object_id };
        self.call("DOM.getFileInfo", &params).await
    }

    async fn dom_get_detached_dom_nodes(&self) -> Result<GetDetachedDomNodesReturn> {
        self.call("DOM.getDetachedDomNodes", &serde_json::json!({}))
            .await
    }

    async fn dom_set_inspected_node(&self, node_id: &NodeId) -> Result<()> {
        let params = SetInspectedNodeInternalParams { node_id };
        self.call_no_response("DOM.setInspectedNode", &params).await
    }

    async fn dom_set_node_name(&self, node_id: &NodeId, name: &str) -> Result<SetNodeNameReturn> {
        let params = SetNodeNameInternalParams { node_id, name };
        self.call("DOM.setNodeName", &params).await
    }

    async fn dom_set_node_value(&self, node_id: &NodeId, value: &str) -> Result<()> {
        let params = SetNodeValueInternalParams { node_id, value };
        self.call_no_response("DOM.setNodeValue", &params).await
    }

    async fn dom_set_outer_html(&self, node_id: &NodeId, outer_html: &str) -> Result<()> {
        let params = SetOuterHTMLInternalParams {
            node_id,
            outer_html,
        };
        self.call_no_response("DOM.setOuterHTML", &params).await
    }

    async fn dom_undo(&self) -> Result<()> {
        self.call_no_response("DOM.undo", &serde_json::json!({}))
            .await
    }

    async fn dom_get_frame_owner(&self, frame_id: &FrameId) -> Result<GetFrameOwnerReturn> {
        let params = GetFrameOwnerInternalParams { frame_id };
        self.call("DOM.getFrameOwner", &params).await
    }

    async fn dom_get_container_for_node(
        &self,
        params: &GetContainerForNodeParams,
    ) -> Result<GetContainerForNodeReturn> {
        self.call("DOM.getContainerForNode", params).await
    }

    async fn dom_get_querying_descendants_for_container(
        &self,
        node_id: &NodeId,
    ) -> Result<GetQueryingDescendantsForContainerReturn> {
        let params = GetQueryingDescendantsForContainerInternalParams { node_id };
        self.call("DOM.getQueryingDescendantsForContainer", &params)
            .await
    }

    async fn dom_get_anchor_element(
        &self,
        params: &GetAnchorElementParams,
    ) -> Result<GetAnchorElementReturn> {
        self.call("DOM.getAnchorElement", params).await
    }

    async fn dom_force_show_popover(
        &self,
        params: &ForceShowPopoverParams,
    ) -> Result<ForceShowPopoverReturn> {
        self.call("DOM.forceShowPopover", params).await
    }
}
