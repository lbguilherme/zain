use serde::{Deserialize, Serialize};

use crate::cdp::runtime::RemoteObject;
use crate::error::Result;
use crate::session::CdpSession;
use crate::types::FrameId;

// ── Types ──────────────────────────────────────────────────────────────────

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
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackendNode {
    /// `Node`'s nodeType.
    pub node_type: i64,
    /// `Node`'s nodeName.
    pub node_name: String,
    pub backend_node_id: BackendNodeId,
}

/// Pseudo element type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PseudoType {
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
    #[serde(rename = "interest-hint")]
    InterestHint,
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
}

/// Shadow root type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShadowRootType {
    #[serde(rename = "user-agent")]
    UserAgent,
    #[serde(rename = "open")]
    Open,
    #[serde(rename = "closed")]
    Closed,
}

/// Document compatibility mode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompatibilityMode {
    QuirksMode,
    LimitedQuirksMode,
    NoQuirksMode,
}

/// ContainerSelector physical axes
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PhysicalAxes {
    Horizontal,
    Vertical,
    Both,
}

/// ContainerSelector logical axes
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogicalAxes {
    Inline,
    Block,
    Both,
}

/// Physical scroll orientation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ScrollOrientation {
    Horizontal,
    Vertical,
}

/// DOM interaction is implemented in terms of mirror objects that represent the actual DOM nodes.
/// DOMNode is a base node mirror type.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    /// Node identifier that is passed into the rest of the DOM messages as the `nodeId`. Backend
    /// will only push node with given `id` once. It is aware of all requested nodes and will only
    /// fire DOM events for nodes known to the client.
    pub node_id: NodeId,
    /// The id of the parent node if any.
    #[serde(default)]
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
    #[serde(default)]
    pub child_node_count: Option<i64>,
    /// Child nodes of this node when requested with children.
    #[serde(default)]
    pub children: Option<Vec<Node>>,
    /// Attributes of the `Element` node in the form of flat array `[name1, value1, name2, value2]`.
    #[serde(default)]
    pub attributes: Option<Vec<String>>,
    /// Document URL that `Document` or `FrameOwner` node points to.
    #[serde(default)]
    pub document_url: Option<String>,
    /// Base URL that `Document` or `FrameOwner` node uses for URL completion.
    #[serde(default)]
    pub base_url: Option<String>,
    /// `DocumentType`'s publicId.
    #[serde(default)]
    pub public_id: Option<String>,
    /// `DocumentType`'s systemId.
    #[serde(default)]
    pub system_id: Option<String>,
    /// `DocumentType`'s internalSubset.
    #[serde(default)]
    pub internal_subset: Option<String>,
    /// `Document`'s XML version in case of XML documents.
    #[serde(default)]
    pub xml_version: Option<String>,
    /// `Attr`'s name.
    #[serde(default)]
    pub name: Option<String>,
    /// `Attr`'s value.
    #[serde(default)]
    pub value: Option<String>,
    /// Pseudo element type for this node.
    #[serde(default)]
    pub pseudo_type: Option<PseudoType>,
    /// Pseudo element identifier for this node. Only present if there is a
    /// valid pseudoType.
    #[serde(default)]
    pub pseudo_identifier: Option<String>,
    /// Shadow root type.
    #[serde(default)]
    pub shadow_root_type: Option<ShadowRootType>,
    /// Frame ID for frame owner elements.
    #[serde(default)]
    pub frame_id: Option<FrameId>,
    /// Content document for frame owner elements.
    #[serde(default)]
    pub content_document: Option<Box<Node>>,
    /// Shadow root list for given element host.
    #[serde(default)]
    pub shadow_roots: Option<Vec<Node>>,
    /// Content document fragment for template elements.
    #[serde(default)]
    pub template_content: Option<Box<Node>>,
    /// Pseudo elements associated with this node.
    #[serde(default)]
    pub pseudo_elements: Option<Vec<Node>>,
    /// Distributed nodes for given insertion point.
    #[serde(default)]
    pub distributed_nodes: Option<Vec<BackendNode>>,
    /// Whether the node is SVG.
    #[serde(default)]
    pub is_svg: Option<bool>,
    #[serde(default)]
    pub compatibility_mode: Option<CompatibilityMode>,
    #[serde(default)]
    pub assigned_slot: Option<BackendNode>,
    #[serde(default)]
    pub is_scrollable: Option<bool>,
    #[serde(default)]
    pub affected_by_starting_styles: Option<bool>,
    #[serde(default)]
    pub adopted_style_sheets: Option<Vec<StyleSheetId>>,
    /// Ad provenance for this node.
    #[serde(default)]
    pub ad_provenance: Option<serde_json::Value>,
}

/// A structure to hold the top-level node of a detached tree and an array of its retained descendants.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetachedElementInfo {
    pub tree_node: Node,
    pub retained_node_ids: Vec<NodeId>,
}

/// A structure holding an RGBA color.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rgba {
    /// The red component, in the [0-255] range.
    pub r: i64,
    /// The green component, in the [0-255] range.
    pub g: i64,
    /// The blue component, in the [0-255] range.
    pub b: i64,
    /// The alpha component, in the [0-1] range (default: 1).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub a: Option<f64>,
}

/// An array of quad vertices, x immediately followed by y for each point, points clock-wise.
pub type Quad = Vec<f64>;

/// Box model.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoxModel {
    /// Content box
    pub content: Quad,
    /// Padding box
    pub padding: Quad,
    /// Border box
    pub border: Quad,
    /// Margin box
    pub margin: Quad,
    /// Node width
    pub width: i64,
    /// Node height
    pub height: i64,
    /// Shape outside coordinates
    #[serde(default)]
    pub shape_outside: Option<ShapeOutsideInfo>,
}

/// CSS Shape Outside details.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShapeOutsideInfo {
    /// Shape bounds
    pub bounds: Quad,
    /// Shape coordinate details
    pub shape: Vec<serde_json::Value>,
    /// Margin shape bounds
    pub margin_shape: Vec<serde_json::Value>,
}

/// Rectangle.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rect {
    /// X coordinate
    pub x: f64,
    /// Y coordinate
    pub y: f64,
    /// Rectangle width
    pub width: f64,
    /// Rectangle height
    pub height: f64,
}

/// CSS computed style property.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CssComputedStyleProperty {
    /// Computed style property name.
    pub name: String,
    /// Computed style property value.
    pub value: String,
}

/// Type of relation for `getElementByRelation`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ElementRelation {
    PopoverTarget,
    InterestTarget,
    CommandFor,
}

/// Whether to include whitespaces in the children array of returned Nodes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum IncludeWhitespace {
    None,
    All,
}

// ── Param types ────────────────────────────────────────────────────────────

/// Parameters for `DOM.collectClassNamesFromSubtree`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectClassNamesFromSubtreeParams {
    /// Id of the node to collect class names.
    pub node_id: NodeId,
}

/// Parameters for `DOM.copyTo`.
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

/// Parameters for `DOM.describeNode`.
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
    pub object_id: Option<String>,
    /// The maximum depth at which children should be retrieved, defaults to 1. Use -1 for the
    /// entire subtree or provide an integer larger than 0.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth: Option<i64>,
    /// Whether or not iframes and shadow roots should be traversed when returning the subtree
    /// (default is false).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pierce: Option<bool>,
}

/// Parameters for `DOM.scrollIntoViewIfNeeded`.
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
    pub object_id: Option<String>,
    /// The rect to be scrolled into view, relative to the node's border box, in CSS pixels.
    /// When omitted, center of the node will be used, similar to Element.scrollIntoView.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rect: Option<Rect>,
}

/// Parameters for `DOM.enable`.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnableParams {
    /// Whether to include whitespaces in the children array of returned Nodes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_whitespace: Option<IncludeWhitespace>,
}

/// Parameters for `DOM.focus`.
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
    pub object_id: Option<String>,
}

/// Parameters for `DOM.getBoxModel`.
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
    pub object_id: Option<String>,
}

/// Parameters for `DOM.getContentQuads`.
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
    pub object_id: Option<String>,
}

/// Parameters for `DOM.getDocument`.
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

/// Parameters for `DOM.getNodesForSubtreeByStyle`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetNodesForSubtreeByStyleParams {
    /// Node ID pointing to the root of a subtree.
    pub node_id: NodeId,
    /// The style to filter nodes by (includes nodes if any of properties matches).
    pub computed_styles: Vec<CssComputedStyleProperty>,
    /// Whether or not iframes and shadow roots in the same target should be traversed when returning the
    /// results (default is false).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pierce: Option<bool>,
}

/// Parameters for `DOM.getNodeForLocation`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetNodeForLocationParams {
    /// X coordinate.
    pub x: i64,
    /// Y coordinate.
    pub y: i64,
    /// False to skip to the nearest non-UA shadow root ancestor (default: false).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_user_agent_shadow_dom: Option<bool>,
    /// Whether to ignore pointer-events: none on elements and hit test them.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore_pointer_events_none: Option<bool>,
}

/// Parameters for `DOM.getOuterHTML`.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetOuterHtmlParams {
    /// Identifier of the node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<NodeId>,
    /// Identifier of the backend node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend_node_id: Option<BackendNodeId>,
    /// JavaScript object id of the node wrapper.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_id: Option<String>,
    /// Include all shadow roots. Equals to false if not specified.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_shadow_dom: Option<bool>,
}

/// Parameters for `DOM.getSearchResults`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSearchResultsParams {
    /// Unique search session identifier.
    pub search_id: String,
    /// Start index of the search result to be returned.
    pub from_index: i64,
    /// End index of the search result to be returned.
    pub to_index: i64,
}

/// Parameters for `DOM.moveTo`.
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

/// Parameters for `DOM.performSearch`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformSearchParams {
    /// Plain text or query selector or XPath search query.
    pub query: String,
    /// True to search in user agent shadow DOM.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_user_agent_shadow_dom: Option<bool>,
}

/// Parameters for `DOM.requestChildNodes`.
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

/// Parameters for `DOM.resolveNode`.
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
    pub execution_context_id: Option<i64>,
}

/// Parameters for `DOM.setAttributeValue`.
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

/// Parameters for `DOM.setAttributesAsText`.
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

/// Parameters for `DOM.setFileInputFiles`.
#[derive(Debug, Clone, Serialize)]
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
    pub object_id: Option<String>,
}

/// Parameters for `DOM.setOuterHTML`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetOuterHtmlParams {
    /// Id of the node to set markup for.
    pub node_id: NodeId,
    /// Outer HTML markup to set.
    #[serde(rename = "outerHTML")]
    pub outer_html: String,
}

/// Parameters for `DOM.getContainerForNode`.
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

/// Parameters for `DOM.getAnchorElement`.
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

// ── Return types ───────────────────────────────────────────────────────────

/// Return type for `DOM.collectClassNamesFromSubtree`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectClassNamesFromSubtreeReturn {
    /// Class name list.
    pub class_names: Vec<String>,
}

/// Return type for `DOM.copyTo`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyToReturn {
    /// Id of the node clone.
    pub node_id: NodeId,
}

/// Return type for `DOM.describeNode`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DescribeNodeReturn {
    /// Node description.
    pub node: Node,
}

/// Return type for `DOM.getAttributes`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetAttributesReturn {
    /// An interleaved array of node attribute names and values.
    pub attributes: Vec<String>,
}

/// Return type for `DOM.getBoxModel`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBoxModelReturn {
    /// Box model for the node.
    pub model: BoxModel,
}

/// Return type for `DOM.getContentQuads`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetContentQuadsReturn {
    /// Quads that describe node layout relative to viewport.
    pub quads: Vec<Quad>,
}

/// Return type for `DOM.getDocument`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetDocumentReturn {
    /// Resulting node.
    pub root: Node,
}

/// Return type for `DOM.getNodesForSubtreeByStyle`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetNodesForSubtreeByStyleReturn {
    /// Resulting nodes.
    pub node_ids: Vec<NodeId>,
}

/// Return type for `DOM.getNodeForLocation`.
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

/// Return type for `DOM.getOuterHTML`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetOuterHtmlReturn {
    /// Outer HTML markup.
    #[serde(rename = "outerHTML")]
    pub outer_html: String,
}

/// Return type for `DOM.getRelayoutBoundary`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetRelayoutBoundaryReturn {
    /// Relayout boundary node id for the given node.
    pub node_id: NodeId,
}

/// Return type for `DOM.getSearchResults`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSearchResultsReturn {
    /// Ids of the search result nodes.
    pub node_ids: Vec<NodeId>,
}

/// Return type for `DOM.moveTo`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveToReturn {
    /// New id of the moved node.
    pub node_id: NodeId,
}

/// Return type for `DOM.performSearch`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformSearchReturn {
    /// Unique search session identifier.
    pub search_id: String,
    /// Number of search results.
    pub result_count: i64,
}

/// Return type for `DOM.pushNodeByPathToFrontend`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PushNodeByPathToFrontendReturn {
    /// Id of the node for given path.
    pub node_id: NodeId,
}

/// Return type for `DOM.pushNodesByBackendIdsToFrontend`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PushNodesByBackendIdsToFrontendReturn {
    /// The array of ids of pushed nodes that correspond to the backend ids specified in
    /// backendNodeIds.
    pub node_ids: Vec<NodeId>,
}

/// Return type for `DOM.querySelector`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuerySelectorReturn {
    /// Query selector result.
    pub node_id: NodeId,
}

/// Return type for `DOM.querySelectorAll`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuerySelectorAllReturn {
    /// Query selector result.
    pub node_ids: Vec<NodeId>,
}

/// Return type for `DOM.getTopLayerElements`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTopLayerElementsReturn {
    /// NodeIds of top layer elements
    pub node_ids: Vec<NodeId>,
}

/// Return type for `DOM.getElementByRelation`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetElementByRelationReturn {
    /// NodeId of the element matching the queried relation.
    pub node_id: NodeId,
}

/// Return type for `DOM.requestNode`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestNodeReturn {
    /// Node id for given object.
    pub node_id: NodeId,
}

/// Return type for `DOM.resolveNode`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveNodeReturn {
    /// JavaScript object wrapper for given node.
    pub object: RemoteObject,
}

/// Return type for `DOM.setNodeName`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetNodeNameReturn {
    /// New node's id.
    pub node_id: NodeId,
}

/// Return type for `DOM.getNodeStackTraces`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetNodeStackTracesReturn {
    /// Creation stack trace, if available.
    #[serde(default)]
    pub creation: Option<serde_json::Value>,
}

/// Return type for `DOM.getFileInfo`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetFileInfoReturn {
    pub path: String,
}

/// Return type for `DOM.getDetachedDomNodes`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetDetachedDomNodesReturn {
    /// The list of detached nodes
    pub detached_nodes: Vec<DetachedElementInfo>,
}

/// Return type for `DOM.getFrameOwner`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetFrameOwnerReturn {
    /// Resulting node.
    pub backend_node_id: BackendNodeId,
    /// Id of the node at given coordinates, only when enabled and requested document.
    #[serde(default)]
    pub node_id: Option<NodeId>,
}

/// Return type for `DOM.getContainerForNode`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetContainerForNodeReturn {
    /// The container node for the given node, or null if not found.
    #[serde(default)]
    pub node_id: Option<NodeId>,
}

/// Return type for `DOM.getQueryingDescendantsForContainer`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetQueryingDescendantsForContainerReturn {
    /// Descendant nodes with container queries against the given container.
    pub node_ids: Vec<NodeId>,
}

/// Return type for `DOM.getAnchorElement`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetAnchorElementReturn {
    /// The anchor element of the given anchor query.
    pub node_id: NodeId,
}

/// Return type for `DOM.forceShowPopover`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForceShowPopoverReturn {
    /// List of popovers that were closed in order to respect popover stacking order.
    pub node_ids: Vec<NodeId>,
}

// ── Events ─────────────────────────────────────────────────────────────────

/// Fired when `Element`'s attribute is modified.
///
/// CDP: `DOM.attributeModified`
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
///
/// CDP: `DOM.adoptedStyleSheetsModified`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdoptedStyleSheetsModifiedEvent {
    /// Id of the node that has changed.
    pub node_id: NodeId,
    /// New adoptedStyleSheets array.
    pub adopted_style_sheets: Vec<StyleSheetId>,
}

/// Fired when `Element`'s attribute is removed.
///
/// CDP: `DOM.attributeRemoved`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttributeRemovedEvent {
    /// Id of the node that has changed.
    pub node_id: NodeId,
    /// Attribute name.
    pub name: String,
}

/// Mirrors `DOMCharacterDataModified` event.
///
/// CDP: `DOM.characterDataModified`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterDataModifiedEvent {
    /// Id of the node that has changed.
    pub node_id: NodeId,
    /// New text value.
    pub character_data: String,
}

/// Fired when `Container`'s child node count has changed.
///
/// CDP: `DOM.childNodeCountUpdated`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChildNodeCountUpdatedEvent {
    /// Id of the node that has changed.
    pub node_id: NodeId,
    /// New node count.
    pub child_node_count: i64,
}

/// Mirrors `DOMNodeInserted` event.
///
/// CDP: `DOM.childNodeInserted`
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
///
/// CDP: `DOM.childNodeRemoved`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChildNodeRemovedEvent {
    /// Parent id.
    pub parent_node_id: NodeId,
    /// Id of the node that has been removed.
    pub node_id: NodeId,
}

/// Called when distribution is changed.
///
/// CDP: `DOM.distributedNodesUpdated`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributedNodesUpdatedEvent {
    /// Insertion point where distributed nodes were updated.
    pub insertion_point_id: NodeId,
    /// Distributed nodes for given insertion point.
    pub distributed_nodes: Vec<BackendNode>,
}

/// Fired when `Document` has been totally updated. Node ids are no longer valid.
///
/// CDP: `DOM.documentUpdated`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentUpdatedEvent {}

/// Fired when `Element`'s inline style is modified via a CSS property modification.
///
/// CDP: `DOM.inlineStyleInvalidated`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InlineStyleInvalidatedEvent {
    /// Ids of the nodes for which the inline styles have been invalidated.
    pub node_ids: Vec<NodeId>,
}

/// Called when a pseudo element is added to an element.
///
/// CDP: `DOM.pseudoElementAdded`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PseudoElementAddedEvent {
    /// Pseudo element's parent element id.
    pub parent_id: NodeId,
    /// The added pseudo element.
    pub pseudo_element: Node,
}

/// Called when top layer elements are changed.
///
/// CDP: `DOM.topLayerElementsUpdated`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TopLayerElementsUpdatedEvent {}

/// Fired when a node's scrollability state changes.
///
/// CDP: `DOM.scrollableFlagUpdated`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScrollableFlagUpdatedEvent {
    /// The id of the node.
    pub node_id: NodeId,
    /// If the node is scrollable.
    pub is_scrollable: bool,
}

/// Fired when a node's ad related state changes.
///
/// CDP: `DOM.adRelatedStateUpdated`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdRelatedStateUpdatedEvent {
    /// The id of the node.
    pub node_id: NodeId,
    /// The provenance of the ad related node, if it is ad related.
    #[serde(default)]
    pub ad_provenance: Option<serde_json::Value>,
}

/// Fired when a node's starting styles changes.
///
/// CDP: `DOM.affectedByStartingStylesFlagUpdated`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AffectedByStartingStylesFlagUpdatedEvent {
    /// The id of the node.
    pub node_id: NodeId,
    /// If the node has starting styles.
    pub affected_by_starting_styles: bool,
}

/// Called when a pseudo element is removed from an element.
///
/// CDP: `DOM.pseudoElementRemoved`
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
///
/// CDP: `DOM.setChildNodes`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetChildNodesEvent {
    /// Parent node id to populate with children.
    pub parent_id: NodeId,
    /// Child nodes array.
    pub nodes: Vec<Node>,
}

/// Called when shadow root is popped from the element.
///
/// CDP: `DOM.shadowRootPopped`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShadowRootPoppedEvent {
    /// Host element id.
    pub host_id: NodeId,
    /// Shadow root id.
    pub root_id: NodeId,
}

/// Called when shadow root is pushed into the element.
///
/// CDP: `DOM.shadowRootPushed`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShadowRootPushedEvent {
    /// Host element id.
    pub host_id: NodeId,
    /// Shadow root.
    pub root: Node,
}

// ── Domain trait ────────────────────────────────────────────────────────────

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
        node_id: NodeId,
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
    async fn dom_get_attributes(&self, node_id: NodeId) -> Result<GetAttributesReturn>;

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
    async fn dom_get_outer_html(&self, params: &GetOuterHtmlParams) -> Result<GetOuterHtmlReturn>;

    /// Returns the id of the nearest ancestor that is a relayout boundary.
    ///
    /// CDP: `DOM.getRelayoutBoundary`
    async fn dom_get_relayout_boundary(&self, node_id: NodeId)
        -> Result<GetRelayoutBoundaryReturn>;

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
    async fn dom_perform_search(
        &self,
        params: &PerformSearchParams,
    ) -> Result<PerformSearchReturn>;

    /// Requests that the node is sent to the caller given its path. // FIXME, use XPath
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
        node_id: NodeId,
        selector: &str,
    ) -> Result<QuerySelectorReturn>;

    /// Executes `querySelectorAll` on a given node.
    ///
    /// CDP: `DOM.querySelectorAll`
    async fn dom_query_selector_all(
        &self,
        node_id: NodeId,
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
        node_id: NodeId,
        relation: ElementRelation,
    ) -> Result<GetElementByRelationReturn>;

    /// Re-does the last undone action.
    ///
    /// CDP: `DOM.redo`
    async fn dom_redo(&self) -> Result<()>;

    /// Removes attribute with given name from an element with given id.
    ///
    /// CDP: `DOM.removeAttribute`
    async fn dom_remove_attribute(&self, node_id: NodeId, name: &str) -> Result<()>;

    /// Removes node with given id.
    ///
    /// CDP: `DOM.removeNode`
    async fn dom_remove_node(&self, node_id: NodeId) -> Result<()>;

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
    async fn dom_request_node(&self, object_id: &str) -> Result<RequestNodeReturn>;

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
    async fn dom_get_node_stack_traces(
        &self,
        node_id: NodeId,
    ) -> Result<GetNodeStackTracesReturn>;

    /// Returns file information for the given File wrapper.
    ///
    /// CDP: `DOM.getFileInfo`
    async fn dom_get_file_info(&self, object_id: &str) -> Result<GetFileInfoReturn>;

    /// Returns list of detached nodes
    ///
    /// CDP: `DOM.getDetachedDomNodes`
    async fn dom_get_detached_dom_nodes(&self) -> Result<GetDetachedDomNodesReturn>;

    /// Enables console to refer to the node with given id via $x (see Command Line API for more details
    /// $x functions).
    ///
    /// CDP: `DOM.setInspectedNode`
    async fn dom_set_inspected_node(&self, node_id: NodeId) -> Result<()>;

    /// Sets node name for a node with given id.
    ///
    /// CDP: `DOM.setNodeName`
    async fn dom_set_node_name(
        &self,
        node_id: NodeId,
        name: &str,
    ) -> Result<SetNodeNameReturn>;

    /// Sets node value for a node with given id.
    ///
    /// CDP: `DOM.setNodeValue`
    async fn dom_set_node_value(&self, node_id: NodeId, value: &str) -> Result<()>;

    /// Sets node HTML markup, returns new node id.
    ///
    /// CDP: `DOM.setOuterHTML`
    async fn dom_set_outer_html(&self, params: &SetOuterHtmlParams) -> Result<()>;

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
        node_id: NodeId,
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
        node_id: NodeId,
        enable: bool,
    ) -> Result<ForceShowPopoverReturn>;
}

// ── Impl ───────────────────────────────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CollectClassNamesFromSubtreeInternalParams {
    node_id: NodeId,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DiscardSearchResultsInternalParams<'a> {
    search_id: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GetAttributesInternalParams {
    node_id: NodeId,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GetRelayoutBoundaryInternalParams {
    node_id: NodeId,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct QuerySelectorInternalParams<'a> {
    node_id: NodeId,
    selector: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GetElementByRelationInternalParams {
    node_id: NodeId,
    relation: ElementRelation,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RemoveAttributeInternalParams<'a> {
    node_id: NodeId,
    name: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RemoveNodeInternalParams {
    node_id: NodeId,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RequestNodeInternalParams<'a> {
    object_id: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetNodeStackTracesEnabledInternalParams {
    enable: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GetNodeStackTracesInternalParams {
    node_id: NodeId,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GetFileInfoInternalParams<'a> {
    object_id: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetInspectedNodeInternalParams {
    node_id: NodeId,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetNodeNameInternalParams<'a> {
    node_id: NodeId,
    name: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetNodeValueInternalParams<'a> {
    node_id: NodeId,
    value: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GetFrameOwnerInternalParams<'a> {
    frame_id: &'a FrameId,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GetQueryingDescendantsForContainerInternalParams {
    node_id: NodeId,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ForceShowPopoverInternalParams {
    node_id: NodeId,
    enable: bool,
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

impl DomCommands for CdpSession {
    async fn dom_collect_class_names_from_subtree(
        &self,
        node_id: NodeId,
    ) -> Result<CollectClassNamesFromSubtreeReturn> {
        self.call(
            "DOM.collectClassNamesFromSubtree",
            &CollectClassNamesFromSubtreeInternalParams { node_id },
        )
        .await
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
        self.call_no_response(
            "DOM.discardSearchResults",
            &DiscardSearchResultsInternalParams { search_id },
        )
        .await
    }

    async fn dom_enable(&self, params: &EnableParams) -> Result<()> {
        self.call_no_response("DOM.enable", params).await
    }

    async fn dom_focus(&self, params: &FocusParams) -> Result<()> {
        self.call_no_response("DOM.focus", params).await
    }

    async fn dom_get_attributes(&self, node_id: NodeId) -> Result<GetAttributesReturn> {
        self.call(
            "DOM.getAttributes",
            &GetAttributesInternalParams { node_id },
        )
        .await
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

    async fn dom_get_outer_html(
        &self,
        params: &GetOuterHtmlParams,
    ) -> Result<GetOuterHtmlReturn> {
        self.call("DOM.getOuterHTML", params).await
    }

    async fn dom_get_relayout_boundary(
        &self,
        node_id: NodeId,
    ) -> Result<GetRelayoutBoundaryReturn> {
        self.call(
            "DOM.getRelayoutBoundary",
            &GetRelayoutBoundaryInternalParams { node_id },
        )
        .await
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
        self.call(
            "DOM.pushNodeByPathToFrontend",
            &PushNodeByPathToFrontendInternalParams { path },
        )
        .await
    }

    async fn dom_push_nodes_by_backend_ids_to_frontend(
        &self,
        backend_node_ids: &[BackendNodeId],
    ) -> Result<PushNodesByBackendIdsToFrontendReturn> {
        self.call(
            "DOM.pushNodesByBackendIdsToFrontend",
            &PushNodesByBackendIdsToFrontendInternalParams { backend_node_ids },
        )
        .await
    }

    async fn dom_query_selector(
        &self,
        node_id: NodeId,
        selector: &str,
    ) -> Result<QuerySelectorReturn> {
        self.call(
            "DOM.querySelector",
            &QuerySelectorInternalParams { node_id, selector },
        )
        .await
    }

    async fn dom_query_selector_all(
        &self,
        node_id: NodeId,
        selector: &str,
    ) -> Result<QuerySelectorAllReturn> {
        self.call(
            "DOM.querySelectorAll",
            &QuerySelectorInternalParams { node_id, selector },
        )
        .await
    }

    async fn dom_get_top_layer_elements(&self) -> Result<GetTopLayerElementsReturn> {
        self.call("DOM.getTopLayerElements", &serde_json::json!({}))
            .await
    }

    async fn dom_get_element_by_relation(
        &self,
        node_id: NodeId,
        relation: ElementRelation,
    ) -> Result<GetElementByRelationReturn> {
        self.call(
            "DOM.getElementByRelation",
            &GetElementByRelationInternalParams { node_id, relation },
        )
        .await
    }

    async fn dom_redo(&self) -> Result<()> {
        self.call_no_response("DOM.redo", &serde_json::json!({}))
            .await
    }

    async fn dom_remove_attribute(&self, node_id: NodeId, name: &str) -> Result<()> {
        self.call_no_response(
            "DOM.removeAttribute",
            &RemoveAttributeInternalParams { node_id, name },
        )
        .await
    }

    async fn dom_remove_node(&self, node_id: NodeId) -> Result<()> {
        self.call_no_response("DOM.removeNode", &RemoveNodeInternalParams { node_id })
            .await
    }

    async fn dom_request_child_nodes(&self, params: &RequestChildNodesParams) -> Result<()> {
        self.call_no_response("DOM.requestChildNodes", params)
            .await
    }

    async fn dom_request_node(&self, object_id: &str) -> Result<RequestNodeReturn> {
        self.call(
            "DOM.requestNode",
            &RequestNodeInternalParams { object_id },
        )
        .await
    }

    async fn dom_resolve_node(&self, params: &ResolveNodeParams) -> Result<ResolveNodeReturn> {
        self.call("DOM.resolveNode", params).await
    }

    async fn dom_set_attribute_value(&self, params: &SetAttributeValueParams) -> Result<()> {
        self.call_no_response("DOM.setAttributeValue", params)
            .await
    }

    async fn dom_set_attributes_as_text(&self, params: &SetAttributesAsTextParams) -> Result<()> {
        self.call_no_response("DOM.setAttributesAsText", params)
            .await
    }

    async fn dom_set_file_input_files(&self, params: &SetFileInputFilesParams) -> Result<()> {
        self.call_no_response("DOM.setFileInputFiles", params)
            .await
    }

    async fn dom_set_node_stack_traces_enabled(&self, enable: bool) -> Result<()> {
        self.call_no_response(
            "DOM.setNodeStackTracesEnabled",
            &SetNodeStackTracesEnabledInternalParams { enable },
        )
        .await
    }

    async fn dom_get_node_stack_traces(
        &self,
        node_id: NodeId,
    ) -> Result<GetNodeStackTracesReturn> {
        self.call(
            "DOM.getNodeStackTraces",
            &GetNodeStackTracesInternalParams { node_id },
        )
        .await
    }

    async fn dom_get_file_info(&self, object_id: &str) -> Result<GetFileInfoReturn> {
        self.call(
            "DOM.getFileInfo",
            &GetFileInfoInternalParams { object_id },
        )
        .await
    }

    async fn dom_get_detached_dom_nodes(&self) -> Result<GetDetachedDomNodesReturn> {
        self.call("DOM.getDetachedDomNodes", &serde_json::json!({}))
            .await
    }

    async fn dom_set_inspected_node(&self, node_id: NodeId) -> Result<()> {
        self.call_no_response(
            "DOM.setInspectedNode",
            &SetInspectedNodeInternalParams { node_id },
        )
        .await
    }

    async fn dom_set_node_name(
        &self,
        node_id: NodeId,
        name: &str,
    ) -> Result<SetNodeNameReturn> {
        self.call(
            "DOM.setNodeName",
            &SetNodeNameInternalParams { node_id, name },
        )
        .await
    }

    async fn dom_set_node_value(&self, node_id: NodeId, value: &str) -> Result<()> {
        self.call_no_response(
            "DOM.setNodeValue",
            &SetNodeValueInternalParams { node_id, value },
        )
        .await
    }

    async fn dom_set_outer_html(&self, params: &SetOuterHtmlParams) -> Result<()> {
        self.call_no_response("DOM.setOuterHTML", params).await
    }

    async fn dom_undo(&self) -> Result<()> {
        self.call_no_response("DOM.undo", &serde_json::json!({}))
            .await
    }

    async fn dom_get_frame_owner(&self, frame_id: &FrameId) -> Result<GetFrameOwnerReturn> {
        self.call(
            "DOM.getFrameOwner",
            &GetFrameOwnerInternalParams { frame_id },
        )
        .await
    }

    async fn dom_get_container_for_node(
        &self,
        params: &GetContainerForNodeParams,
    ) -> Result<GetContainerForNodeReturn> {
        self.call("DOM.getContainerForNode", params).await
    }

    async fn dom_get_querying_descendants_for_container(
        &self,
        node_id: NodeId,
    ) -> Result<GetQueryingDescendantsForContainerReturn> {
        self.call(
            "DOM.getQueryingDescendantsForContainer",
            &GetQueryingDescendantsForContainerInternalParams { node_id },
        )
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
        node_id: NodeId,
        enable: bool,
    ) -> Result<ForceShowPopoverReturn> {
        self.call(
            "DOM.forceShowPopover",
            &ForceShowPopoverInternalParams { node_id, enable },
        )
        .await
    }
}
