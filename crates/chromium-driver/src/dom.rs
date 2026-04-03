use std::collections::HashMap;
use std::time::Duration;

use base64::Engine;
use rand::RngExt;
use rand_distr::{Distribution, Normal};

use crate::cdp::dom::{
    BackendNodeId, BoxModel, DomCommands, EnableParams, GetBoxModelParams, GetDocumentParams,
    GetOuterHtmlParams, NodeId, ResolveNodeParams,
};
use crate::cdp::input::{
    DispatchKeyEventParams, DispatchKeyEventType, DispatchTouchEventParams, DispatchTouchEventType,
    InputCommands, TouchPoint,
};
use crate::cdp::page::{CaptureScreenshotFormat, CaptureScreenshotParams, PageCommands, Viewport};
use crate::error::{CdpError, Result};
use crate::keyboard::{self, KeySequence};
use crate::runtime::JsObject;
use crate::session::CdpSession;

// ── Human-like timing constants ───────────────────────────────────────────

const KEY_HOLD_MEAN_MS: f64 = 50.0;
const KEY_HOLD_STDDEV_MS: f64 = 15.0;
const CLICK_OFFSET_MAX_PX: f64 = 3.0;
const PRE_CLICK_MEAN_MS: f64 = 200.0;
const PRE_CLICK_STDDEV_MS: f64 = 80.0;
const TOUCH_HOLD_MEAN_MS: f64 = 70.0;
const TOUCH_HOLD_STDDEV_MS: f64 = 25.0;
const POST_CLICK_MEAN_MS: f64 = 300.0;
const POST_CLICK_STDDEV_MS: f64 = 100.0;

/// Samples a duration from a gaussian distribution, clamped to [mean/4, mean*3].
/// Returns Duration::ZERO if mean is zero.
fn gaussian_delay(mean_ms: f64, stddev_ms: f64) -> Duration {
    if mean_ms <= 0.0 {
        return Duration::ZERO;
    }
    let normal = Normal::new(mean_ms, stddev_ms.max(0.1)).unwrap();
    let sample = normal.sample(&mut rand::rng());
    let clamped = sample.clamp(mean_ms / 4.0, mean_ms * 3.0);
    Duration::from_millis(clamped as u64)
}

/// Samples a random offset within [-max, max] using uniform distribution.
fn jitter_offset(max_px: f64) -> f64 {
    if max_px <= 0.0 {
        return 0.0;
    }
    rand::rng().random_range(-max_px..=max_px)
}

/// Returns (mean_ms, stddev_ms) for inter-key delay based on text length.
///
/// Short texts (≤10 chars): ~100ms/key (careful typing).
/// Longer texts ramp down to ~40ms/key (fluent typing), which is still
/// within human range (~150 WPM). The curve is a simple linear interpolation
/// clamped at both ends.
fn typing_speed(char_count: usize) -> (f64, f64) {
    const SLOW_MEAN: f64 = 100.0; // short messages
    const FAST_MEAN: f64 = 40.0; // long messages
    const RAMP_START: f64 = 10.0; // chars where speedup begins
    const RAMP_END: f64 = 100.0; // chars where speedup plateaus

    let n = char_count as f64;
    let t = ((n - RAMP_START) / (RAMP_END - RAMP_START)).clamp(0.0, 1.0);
    let mean = SLOW_MEAN + t * (FAST_MEAN - SLOW_MEAN);
    let stddev = mean * 0.3;
    (mean, stddev)
}

async fn human_sleep(mean_ms: f64, stddev_ms: f64) {
    let d = gaussian_delay(mean_ms, stddev_ms);
    if !d.is_zero() {
        tokio::time::sleep(d).await;
    }
}

/// High-level DOM interface for querying and interacting with page elements.
///
/// Wraps a [`CdpSession`] and manages the DOM domain lifecycle.
///
/// **Important**: Caches the document root node ID. Calling [`Dom::invalidate`]
/// forces a fresh `DOM.getDocument` on the next query. This is necessary if
/// the page navigates or the DOM is replaced entirely. Within a stable page
/// (e.g. WhatsApp Web SPA), the cached root remains valid.
///
/// # Example
///
/// ```rust,no_run
/// # async fn example(page: &chromium_driver::PageSession) -> chromium_driver::Result<()> {
/// use chromium_driver::dom::Dom;
/// use std::time::Duration;
///
/// let dom = Dom::enable(page.cdp()).await?;
/// let el = dom.wait_for("div[role='textbox']", Duration::from_secs(5)).await?;
/// el.click().await?;
/// el.type_text("hello").await?;
/// el.press_key("Enter").await?;
/// dom.disable().await?;
/// # Ok(())
/// # }
/// ```
pub struct Dom {
    cdp: CdpSession,
    cached_root: std::sync::Mutex<Option<NodeId>>,
    /// For frame-rooted Doms: the stable backend node ID of the frame's document.
    /// Used to re-resolve a fresh NodeId when the cached root is invalidated.
    frame_backend_id: Option<BackendNodeId>,
}

impl Dom {
    pub(crate) fn new(cdp: CdpSession) -> Self {
        Self {
            cdp,
            cached_root: std::sync::Mutex::new(None),
            frame_backend_id: None,
        }
    }

    /// Creates a `Dom` rooted at a frame's document, identified by its stable
    /// `BackendNodeId`. The `NodeId` is resolved lazily on each `root_id()` call
    /// after invalidation, so it survives across `DOM.getDocument` calls.
    pub(crate) fn for_frame(cdp: CdpSession, backend_node_id: BackendNodeId) -> Self {
        Self {
            cdp,
            cached_root: std::sync::Mutex::new(None),
            frame_backend_id: Some(backend_node_id),
        }
    }

    /// Enables the DOM domain and returns a new `Dom` handle.
    pub async fn enable(cdp: &CdpSession) -> Result<Self> {
        cdp.dom_enable(&EnableParams::default()).await?;
        Ok(Self {
            cdp: cdp.clone(),
            cached_root: std::sync::Mutex::new(None),
            frame_backend_id: None,
        })
    }

    /// Disables the DOM domain.
    pub async fn disable(&self) -> Result<()> {
        self.cdp.dom_disable().await
    }

    /// Finds the first element matching a CSS selector.
    ///
    /// Returns `None` if no element matches.
    pub async fn try_query_selector(&self, selector: &str) -> Result<Option<Element>> {
        let root_id = self.root_id().await?;
        let qs = self.cdp.dom_query_selector(root_id, selector).await?;
        if qs.node_id.0 > 0 {
            Ok(Some(Element {
                cdp: self.cdp.clone(),
                node_id: qs.node_id,
            }))
        } else {
            Ok(None)
        }
    }

    /// Finds the first element matching a CSS selector.
    ///
    /// Returns an error if no element matches.
    pub async fn query_selector(&self, selector: &str) -> Result<Element> {
        self.try_query_selector(selector)
            .await?
            .ok_or_else(|| CdpError::Protocol {
                code: -1,
                message: format!("no element matches selector: {selector}"),
            })
    }

    /// Finds all elements matching a CSS selector.
    pub async fn query_selector_all(&self, selector: &str) -> Result<Vec<Element>> {
        let root_id = self.root_id().await?;
        let qs = self.cdp.dom_query_selector_all(root_id, selector).await?;
        Ok(qs
            .node_ids
            .into_iter()
            .filter(|id| id.0 > 0)
            .map(|node_id| Element {
                cdp: self.cdp.clone(),
                node_id,
            })
            .collect())
    }

    /// Waits for an element matching the selector to appear in the DOM.
    ///
    /// Polls at ~500ms intervals. Returns the element once found, or
    /// a timeout error.
    pub async fn wait_for(&self, selector: &str, timeout: Duration) -> Result<Element> {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            if let Some(el) = self.try_query_selector(selector).await? {
                return Ok(el);
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(CdpError::Timeout(timeout));
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    /// Returns the full outer HTML of the `<html>` element.
    pub async fn page_html(&self) -> Result<String> {
        let html_el = self.query_selector("html").await?;
        html_el.outer_html().await
    }

    /// Swipes at explicit viewport coordinates — does not use element box model.
    ///
    /// Simulates a finger drag from `(x, start_y)` to `(x, end_y)`.
    /// Use this for virtualised containers where the element box model height is
    /// the full virtual scrollable height (far larger than the visible viewport),
    /// which would place touch events outside the screen if element methods were used.
    pub async fn swipe_vertical(&self, x: f64, start_y: f64, end_y: f64) -> Result<()> {
        let distance = (end_y - start_y).abs();
        let steps = 5 + (distance / 100.0) as usize;
        let step_dy = (end_y - start_y) / steps as f64;

        self.cdp
            .input_dispatch_touch_event(&DispatchTouchEventParams {
                event_type: DispatchTouchEventType::TouchStart,
                touch_points: vec![TouchPoint {
                    x,
                    y: start_y,
                    ..Default::default()
                }],
                ..Default::default()
            })
            .await?;

        let mut y = start_y;
        for _ in 0..steps {
            y += step_dy + jitter_offset(2.0);
            y = if step_dy > 0.0 {
                y.min(end_y)
            } else {
                y.max(end_y)
            };
            human_sleep(20.0, 8.0).await;
            self.cdp
                .input_dispatch_touch_event(&DispatchTouchEventParams {
                    event_type: DispatchTouchEventType::TouchMove,
                    touch_points: vec![TouchPoint {
                        x: x + jitter_offset(1.0),
                        y,
                        ..Default::default()
                    }],
                    ..Default::default()
                })
                .await?;
        }

        human_sleep(TOUCH_HOLD_MEAN_MS, TOUCH_HOLD_STDDEV_MS).await;
        self.cdp
            .input_dispatch_touch_event(&DispatchTouchEventParams {
                event_type: DispatchTouchEventType::TouchEnd,
                touch_points: vec![],
                ..Default::default()
            })
            .await?;

        Ok(())
    }

    /// Forces the next query to re-fetch the document root.
    pub fn invalidate(&self) {
        *self.cached_root.lock().unwrap() = None;
    }

    async fn root_id(&self) -> Result<NodeId> {
        {
            let cached = self.cached_root.lock().unwrap();
            if let Some(id) = *cached {
                return Ok(id);
            }
        }

        let id = if let Some(backend_id) = self.frame_backend_id {
            // Frame-rooted Dom: get full tree first, then resolve the backend ID.
            let _ = self
                .cdp
                .dom_get_document(&GetDocumentParams {
                    depth: Some(-1),
                    pierce: Some(true),
                })
                .await?;
            let ret = self
                .cdp
                .dom_push_nodes_by_backend_ids_to_frontend(&[backend_id])
                .await?;
            let node_id = ret
                .node_ids
                .first()
                .copied()
                .ok_or_else(|| CdpError::Protocol {
                    code: -1,
                    message: "pushNodesByBackendIds returned empty".into(),
                })?;
            if node_id.0 <= 0 {
                return Err(CdpError::Protocol {
                    code: -1,
                    message: "pushNodesByBackendIds returned invalid node id".into(),
                });
            }
            node_id
        } else {
            // Top-level Dom: use default getDocument.
            let doc = self
                .cdp
                .dom_get_document(&GetDocumentParams::default())
                .await?;
            doc.root.node_id
        };

        *self.cached_root.lock().unwrap() = Some(id);
        Ok(id)
    }
}

/// A reference to a DOM element with methods for inspection and interaction.
///
/// Obtained from [`Dom::query_selector`], [`Dom::query_selector_all`],
/// [`Dom::wait_for`], or [`Element::query_selector`].
///
/// Interaction methods (`click`, `type_text`, `press_key`) use built-in
/// gaussian-distributed timing for realistic human-like behavior.
pub struct Element {
    cdp: CdpSession,
    node_id: NodeId,
}

impl Element {
    /// Returns the CDP node ID of this element.
    pub fn node_id(&self) -> NodeId {
        self.node_id
    }

    // --- Inspection ---

    /// Returns the outer HTML of this element.
    pub async fn outer_html(&self) -> Result<String> {
        let ret = self
            .cdp
            .dom_get_outer_html(&GetOuterHtmlParams {
                node_id: Some(self.node_id),
                ..Default::default()
            })
            .await?;
        Ok(ret.outer_html)
    }

    /// Returns the visible text content of this element (via outerHTML stripping tags).
    pub async fn text(&self) -> Result<String> {
        let html = self.outer_html().await?;
        Ok(strip_tags(&html))
    }

    /// Returns the value of a specific attribute, or `None` if not present.
    pub async fn attribute(&self, name: &str) -> Result<Option<String>> {
        let attrs = self.attributes().await?;
        Ok(attrs.get(name).cloned())
    }

    /// Returns all attributes as a name->value map.
    pub async fn attributes(&self) -> Result<HashMap<String, String>> {
        let ret = self.cdp.dom_get_attributes(self.node_id).await?;

        let mut map = HashMap::new();
        for pair in ret.attributes.chunks(2) {
            if let [name, value] = pair {
                map.insert(name.clone(), value.clone());
            }
        }
        Ok(map)
    }

    /// Returns the CSS box model for this element.
    pub async fn box_model(&self) -> Result<BoxModel> {
        let ret = self
            .cdp
            .dom_get_box_model(&GetBoxModelParams {
                node_id: Some(self.node_id),
                ..Default::default()
            })
            .await?;
        Ok(ret.model)
    }

    // --- Scoped queries ---

    /// Finds the first child element matching a CSS selector within this element.
    pub async fn try_query_selector(&self, selector: &str) -> Result<Option<Element>> {
        let qs = self.cdp.dom_query_selector(self.node_id, selector).await?;
        if qs.node_id.0 > 0 {
            Ok(Some(Element {
                cdp: self.cdp.clone(),
                node_id: qs.node_id,
            }))
        } else {
            Ok(None)
        }
    }

    /// Finds the first child element matching a CSS selector within this element.
    ///
    /// Returns an error if no element matches.
    pub async fn query_selector(&self, selector: &str) -> Result<Element> {
        self.try_query_selector(selector)
            .await?
            .ok_or_else(|| CdpError::Protocol {
                code: -1,
                message: format!("no element matches selector: {selector}"),
            })
    }

    /// Finds all child elements matching a CSS selector within this element.
    pub async fn query_selector_all(&self, selector: &str) -> Result<Vec<Element>> {
        let qs = self
            .cdp
            .dom_query_selector_all(self.node_id, selector)
            .await?;
        Ok(qs
            .node_ids
            .into_iter()
            .filter(|id| id.0 > 0)
            .map(|node_id| Element {
                cdp: self.cdp.clone(),
                node_id,
            })
            .collect())
    }

    // --- JS bridge ---

    /// Resolves this DOM node to a JavaScript `JsObject`.
    ///
    /// The returned `JsObject` can be used with [`JsObject::eval`] to call
    /// functions with this element as `this`.
    pub async fn resolve(&self) -> Result<JsObject> {
        let ret = self
            .cdp
            .dom_resolve_node(&ResolveNodeParams {
                node_id: Some(self.node_id),
                ..Default::default()
            })
            .await?;
        JsObject::new(self.cdp.clone(), ret.object).ok_or_else(|| CdpError::Protocol {
            code: -1,
            message: "DOM.resolveNode returned object without objectId".into(),
        })
    }

    // --- Interaction ---

    /// Taps this element simulating a touchscreen interaction.
    ///
    /// Dispatches `touchStart` → (hold) → `touchEnd` with gaussian-distributed
    /// delays and a small random offset from the element center.
    /// A pre-tap delay simulates the time a human takes to locate and reach
    /// for the element. All events are `isTrusted: true`.
    ///
    /// **The caller is responsible for ensuring the element is visible and
    /// not obscured before calling this method.**
    pub async fn click(&self) -> Result<()> {
        let (cx, cy) = self.center().await?;
        let x = cx + jitter_offset(CLICK_OFFSET_MAX_PX);
        let y = cy + jitter_offset(CLICK_OFFSET_MAX_PX);

        // Pre-tap delay: human locates the element and moves finger
        human_sleep(PRE_CLICK_MEAN_MS, PRE_CLICK_STDDEV_MS).await;

        // Finger touches screen
        self.cdp
            .input_dispatch_touch_event(&DispatchTouchEventParams {
                event_type: DispatchTouchEventType::TouchStart,
                touch_points: vec![TouchPoint {
                    x,
                    y,
                    ..Default::default()
                }],
                ..Default::default()
            })
            .await?;

        // Finger hold duration
        human_sleep(TOUCH_HOLD_MEAN_MS, TOUCH_HOLD_STDDEV_MS).await;

        // Finger lifts
        self.cdp
            .input_dispatch_touch_event(&DispatchTouchEventParams {
                event_type: DispatchTouchEventType::TouchEnd,
                touch_points: vec![],
                ..Default::default()
            })
            .await?;

        // Post-tap delay: wait for UI reaction before next action
        human_sleep(POST_CLICK_MEAN_MS, POST_CLICK_STDDEV_MS).await;

        Ok(())
    }

    /// Types text into this element character by character with human-like timing.
    ///
    /// Uses ABNT2 keyboard layout mapping to produce realistic key event sequences:
    /// - Regular characters: `rawKeyDown` → `char` → `keyUp` with correct `code` and `key`
    /// - Accented characters: dead key sequence (e.g. ´ then e → é)
    /// - Unmapped characters (emoji, symbols): simulated Ctrl+V paste
    ///
    /// All events are `isTrusted: true` with gaussian-distributed timing.
    pub async fn type_text(&self, text: &str) -> Result<()> {
        let char_count = text.chars().count();
        let (interval_mean, interval_stddev) = typing_speed(char_count);

        for ch in text.chars() {
            let seq = keyboard::abnt2_sequence(ch);
            let ch_str = ch.to_string();

            match seq {
                KeySequence::Simple {
                    key,
                    code,
                    shift,
                    vk,
                } => {
                    self.dispatch_key_press(key, code, &ch_str, shift, vk)
                        .await?;
                }
                KeySequence::DeadKey {
                    dead_key,
                    dead_code,
                    dead_shift,
                    dead_vk,
                    base_key,
                    base_code,
                    base_shift,
                    base_vk,
                } => {
                    self.dispatch_dead_key(dead_key, dead_code, dead_shift, dead_vk)
                        .await?;

                    human_sleep(interval_mean, interval_stddev).await;

                    self.dispatch_key_press(base_key, base_code, &ch_str, base_shift, base_vk)
                        .await?;
                }
                KeySequence::Paste => {
                    self.dispatch_paste(&ch_str).await?;
                }
            }

            // Inter-key delay
            human_sleep(interval_mean, interval_stddev).await;
        }
        Ok(())
    }

    /// Presses a special key (e.g. `"Enter"`, `"Tab"`, `"Escape"`, `"Backspace"`)
    /// with human-like key hold timing.
    pub async fn press_key(&self, key: &str) -> Result<()> {
        self.cdp
            .input_dispatch_key_event(&DispatchKeyEventParams {
                event_type: DispatchKeyEventType::KeyDown,
                key: Some(key.into()),
                ..Default::default()
            })
            .await?;

        human_sleep(KEY_HOLD_MEAN_MS, KEY_HOLD_STDDEV_MS).await;

        self.cdp
            .input_dispatch_key_event(&DispatchKeyEventParams {
                event_type: DispatchKeyEventType::KeyUp,
                key: Some(key.into()),
                ..Default::default()
            })
            .await?;

        Ok(())
    }

    /// Dispatches a full key press: rawKeyDown → (hold) → char → keyUp.
    async fn dispatch_key_press(
        &self,
        key: &str,
        code: &str,
        text: &str,
        shift: bool,
        _vk: i32,
    ) -> Result<()> {
        let modifiers = if shift { Some(8) } else { None };

        // rawKeyDown
        self.cdp
            .input_dispatch_key_event(&DispatchKeyEventParams {
                event_type: DispatchKeyEventType::RawKeyDown,
                key: Some(key.into()),
                code: Some(code.into()),
                modifiers,
                ..Default::default()
            })
            .await?;

        human_sleep(KEY_HOLD_MEAN_MS, KEY_HOLD_STDDEV_MS).await;

        // char (inserts the text)
        self.cdp
            .input_dispatch_key_event(&DispatchKeyEventParams {
                event_type: DispatchKeyEventType::Char,
                text: Some(text.into()),
                ..Default::default()
            })
            .await?;

        // keyUp
        self.cdp
            .input_dispatch_key_event(&DispatchKeyEventParams {
                event_type: DispatchKeyEventType::KeyUp,
                key: Some(key.into()),
                code: Some(code.into()),
                modifiers,
                ..Default::default()
            })
            .await?;

        Ok(())
    }

    /// Dispatches a dead key press/release (no text inserted).
    async fn dispatch_dead_key(&self, key: &str, code: &str, shift: bool, _vk: i32) -> Result<()> {
        let modifiers = if shift { Some(8) } else { None };

        self.cdp
            .input_dispatch_key_event(&DispatchKeyEventParams {
                event_type: DispatchKeyEventType::RawKeyDown,
                key: Some(key.into()),
                code: Some(code.into()),
                modifiers,
                ..Default::default()
            })
            .await?;

        human_sleep(KEY_HOLD_MEAN_MS, KEY_HOLD_STDDEV_MS).await;

        self.cdp
            .input_dispatch_key_event(&DispatchKeyEventParams {
                event_type: DispatchKeyEventType::KeyUp,
                key: Some(key.into()),
                code: Some(code.into()),
                modifiers,
                ..Default::default()
            })
            .await?;

        Ok(())
    }

    /// Inserts text for characters not on the ABNT2 layout (emoji, special symbols).
    ///
    /// Uses `Input.insertText` which directly inserts into the focused element,
    /// similar to how a paste operation works.
    async fn dispatch_paste(&self, text: &str) -> Result<()> {
        self.cdp.input_insert_text(text).await
    }

    // --- Capture ---

    /// Takes a PNG screenshot of this element and returns the raw bytes.
    pub async fn screenshot_png(&self) -> Result<Vec<u8>> {
        let bm = self.box_model().await?;
        let q = &bm.content;
        if q.len() < 8 {
            return Err(CdpError::Protocol {
                code: -1,
                message: "invalid box model for screenshot".into(),
            });
        }

        let x = q[0];
        let y = q[1];
        let width = q[2] - q[0];
        let height = q[5] - q[1];

        let ret = self
            .cdp
            .page_capture_screenshot(&CaptureScreenshotParams {
                format: Some(CaptureScreenshotFormat::Png),
                clip: Some(Viewport {
                    x,
                    y,
                    width,
                    height,
                    scale: 1.0,
                }),
                ..Default::default()
            })
            .await?;

        let png = base64::engine::general_purpose::STANDARD
            .decode(&ret.data)
            .map_err(|e| CdpError::Protocol {
                code: -1,
                message: format!("base64 decode: {e}"),
            })?;

        Ok(png)
    }

    // --- Scroll ---

    /// Swipes up on this element to scroll its content down.
    ///
    /// Simulates a human finger drag: touch near the bottom, move upward in
    /// several steps with gaussian-distributed timing, then release.
    /// `distance` is the total vertical pixels to drag.
    pub async fn swipe_up(&self, distance: f64) -> Result<()> {
        let bm = self.box_model().await?;
        let q = &bm.content;
        if q.len() < 8 {
            return Err(CdpError::Protocol {
                code: -1,
                message: "invalid box model for swipe".into(),
            });
        }

        let cx = (q[0] + q[2]) / 2.0 + jitter_offset(CLICK_OFFSET_MAX_PX);
        let top_y = q[1];
        let bottom_y = q[5];
        let height = bottom_y - top_y;

        // Start near the bottom 75% of the element
        let start_y = top_y + height * 0.75 + jitter_offset(CLICK_OFFSET_MAX_PX);
        let end_y = (start_y - distance).max(top_y + 10.0);

        // Touch down
        self.cdp
            .input_dispatch_touch_event(&DispatchTouchEventParams {
                event_type: DispatchTouchEventType::TouchStart,
                touch_points: vec![TouchPoint {
                    x: cx,
                    y: start_y,
                    ..Default::default()
                }],
                ..Default::default()
            })
            .await?;

        // Move in steps
        let steps = 5 + (distance / 100.0) as usize;
        let step_dy = (start_y - end_y) / steps as f64;
        let mut y = start_y;
        for _ in 0..steps {
            y -= step_dy + jitter_offset(2.0);
            y = y.max(end_y);
            human_sleep(20.0, 8.0).await;
            self.cdp
                .input_dispatch_touch_event(&DispatchTouchEventParams {
                    event_type: DispatchTouchEventType::TouchMove,
                    touch_points: vec![TouchPoint {
                        x: cx + jitter_offset(1.0),
                        y,
                        ..Default::default()
                    }],
                    ..Default::default()
                })
                .await?;
        }

        // Release
        human_sleep(TOUCH_HOLD_MEAN_MS, TOUCH_HOLD_STDDEV_MS).await;
        self.cdp
            .input_dispatch_touch_event(&DispatchTouchEventParams {
                event_type: DispatchTouchEventType::TouchEnd,
                touch_points: vec![],
                ..Default::default()
            })
            .await?;

        Ok(())
    }

    /// Swipes down on this element to scroll its content up (toward the top).
    ///
    /// Opposite of [`swipe_up`](Self::swipe_up): touch near the top, drag
    /// downward. `distance` is the total vertical pixels to drag.
    pub async fn swipe_down(&self, distance: f64) -> Result<()> {
        let bm = self.box_model().await?;
        let q = &bm.content;
        if q.len() < 8 {
            return Err(CdpError::Protocol {
                code: -1,
                message: "invalid box model for swipe".into(),
            });
        }

        let cx = (q[0] + q[2]) / 2.0 + jitter_offset(CLICK_OFFSET_MAX_PX);
        let top_y = q[1];
        let bottom_y = q[5];
        let height = bottom_y - top_y;

        // Start near the top 25% of the element
        let start_y = top_y + height * 0.25 + jitter_offset(CLICK_OFFSET_MAX_PX);
        let end_y = (start_y + distance).min(bottom_y - 10.0);

        self.cdp
            .input_dispatch_touch_event(&DispatchTouchEventParams {
                event_type: DispatchTouchEventType::TouchStart,
                touch_points: vec![TouchPoint {
                    x: cx,
                    y: start_y,
                    ..Default::default()
                }],
                ..Default::default()
            })
            .await?;

        let steps = 5 + (distance / 100.0) as usize;
        let step_dy = (end_y - start_y) / steps as f64;
        let mut y = start_y;
        for _ in 0..steps {
            y += step_dy + jitter_offset(2.0);
            y = y.min(end_y);
            human_sleep(20.0, 8.0).await;
            self.cdp
                .input_dispatch_touch_event(&DispatchTouchEventParams {
                    event_type: DispatchTouchEventType::TouchMove,
                    touch_points: vec![TouchPoint {
                        x: cx + jitter_offset(1.0),
                        y,
                        ..Default::default()
                    }],
                    ..Default::default()
                })
                .await?;
        }

        human_sleep(TOUCH_HOLD_MEAN_MS, TOUCH_HOLD_STDDEV_MS).await;
        self.cdp
            .input_dispatch_touch_event(&DispatchTouchEventParams {
                event_type: DispatchTouchEventType::TouchEnd,
                touch_points: vec![],
                ..Default::default()
            })
            .await?;

        Ok(())
    }

    // --- Internal ---

    /// Returns the x center coordinate of this element from its box model.
    ///
    /// Only the horizontal bounds are used — y is left to the caller.
    /// Useful for virtualised containers where the box_model height is the
    /// full virtual height (much larger than the viewport).
    pub async fn center_x(&self) -> Result<f64> {
        let bm = self.box_model().await?;
        let q = &bm.content;
        if q.len() < 8 {
            return Err(CdpError::Protocol {
                code: -1,
                message: "invalid box model for center_x".into(),
            });
        }
        Ok((q[0] + q[2]) / 2.0)
    }

    async fn center(&self) -> Result<(f64, f64)> {
        let bm = self.box_model().await?;
        let q = &bm.content;
        if q.len() < 8 {
            return Err(CdpError::Protocol {
                code: -1,
                message: "invalid box model for center calculation".into(),
            });
        }
        let cx = (q[0] + q[2]) / 2.0;
        let cy = (q[1] + q[5]) / 2.0;
        Ok((cx, cy))
    }
}

fn strip_tags(html: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }
    result
}
