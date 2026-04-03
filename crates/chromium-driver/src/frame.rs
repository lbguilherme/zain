use crate::cdp::dom::{BackendNodeId, DomCommands, EnableParams, GetDocumentParams, Node};
use crate::cdp::page::{CreateIsolatedWorldParams, FrameTree, PageCommands};
use crate::cdp::runtime::ExecutionContextId;
use crate::dom::Dom;
use crate::error::{CdpError, Result};
use crate::runtime::{self, EvalResult};
use crate::session::CdpSession;
use crate::types::FrameId;

/// Information about a single frame in the page's frame tree.
#[derive(Debug, Clone)]
pub struct FrameInfo {
    pub id: FrameId,
    pub parent_id: Option<FrameId>,
    pub url: String,
    pub security_origin: Option<String>,
    pub mime_type: Option<String>,
}

/// A session scoped to a specific iframe.
///
/// Created via [`PageSession::frame`](crate::PageSession::frame). Provides
/// `eval` and `dom` methods that operate within the iframe's execution context
/// and document, rather than the top-level page.
pub struct FrameSession {
    cdp: CdpSession,
    frame_id: FrameId,
    context_id: ExecutionContextId,
}

impl FrameSession {
    pub fn frame_id(&self) -> &FrameId {
        &self.frame_id
    }

    /// Returns a [`Dom`] handle rooted at this iframe's document.
    ///
    /// Fetches the full DOM tree with `pierce: true` and walks it to find
    /// the `contentDocument` whose `frameId` matches this frame. The Dom is
    /// backed by the stable `BackendNodeId`, so it survives across multiple
    /// `DOM.getDocument` calls (e.g. when multiple frames are used).
    pub async fn dom(&self) -> Result<Dom> {
        let doc = self
            .cdp
            .dom_get_document(&GetDocumentParams {
                depth: Some(-1),
                pierce: Some(true),
            })
            .await?;

        let backend_id =
            find_frame_document_backend_id(&doc.root, &self.frame_id).ok_or_else(|| {
                CdpError::Protocol {
                    code: -1,
                    message: format!("could not find document for frame {}", self.frame_id.0),
                }
            })?;

        Ok(Dom::for_frame(self.cdp.clone(), backend_id))
    }

    /// Evaluates a JavaScript expression in this iframe's execution context.
    pub async fn eval(&self, expression: &str) -> Result<EvalResult> {
        runtime::evaluate_in_context(&self.cdp, expression, self.context_id).await
    }

    /// Evaluates a JavaScript expression in this iframe and returns the result by value.
    pub async fn eval_value(&self, expression: &str) -> Result<serde_json::Value> {
        runtime::evaluate_value_in_context(&self.cdp, expression, self.context_id).await
    }
}

/// Recursively walks the DOM tree to find the `contentDocument` of the
/// `<iframe>` element whose `frameId` matches the target.
/// Returns the stable `BackendNodeId` so it survives across DOM invalidations.
fn find_frame_document_backend_id(node: &Node, target_frame_id: &FrameId) -> Option<BackendNodeId> {
    // If this node is a frame owner (iframe/frame element) with matching frameId,
    // return its contentDocument's backend_node_id.
    if node.frame_id.as_ref() == Some(target_frame_id)
        && let Some(ref content_doc) = node.content_document
    {
        return Some(content_doc.backend_node_id);
    }

    // Recurse into contentDocument (for nested iframes).
    if let Some(ref content_doc) = node.content_document
        && let Some(id) = find_frame_document_backend_id(content_doc, target_frame_id)
    {
        return Some(id);
    }

    // Recurse children.
    if let Some(ref children) = node.children {
        for child in children {
            if let Some(id) = find_frame_document_backend_id(child, target_frame_id) {
                return Some(id);
            }
        }
    }

    // Recurse shadow roots.
    if let Some(ref shadow_roots) = node.shadow_roots {
        for sr in shadow_roots {
            if let Some(id) = find_frame_document_backend_id(sr, target_frame_id) {
                return Some(id);
            }
        }
    }

    None
}

pub(crate) fn flatten_frame_tree(tree: &FrameTree) -> Vec<FrameInfo> {
    let mut out = Vec::new();
    collect_frames(tree, &mut out);
    out
}

fn collect_frames(tree: &FrameTree, out: &mut Vec<FrameInfo>) {
    out.push(FrameInfo {
        id: tree.frame.id.clone(),
        parent_id: tree.frame.parent_id.clone(),
        url: tree.frame.url.clone(),
        security_origin: tree.frame.security_origin.clone(),
        mime_type: tree.frame.mime_type.clone(),
    });
    if let Some(children) = &tree.child_frames {
        for child in children {
            collect_frames(child, out);
        }
    }
}

pub(crate) async fn enter_frame(cdp: &CdpSession, frame_id: &FrameId) -> Result<FrameSession> {
    let _ = cdp.dom_enable(&EnableParams::default()).await;

    let ret = cdp
        .page_create_isolated_world(&CreateIsolatedWorldParams {
            frame_id: frame_id.clone(),
            world_name: Some("chromium-driver-frame".into()),
            grant_univeral_access: Some(true),
        })
        .await?;

    Ok(FrameSession {
        cdp: cdp.clone(),
        frame_id: frame_id.clone(),
        context_id: ExecutionContextId(ret.execution_context_id),
    })
}
