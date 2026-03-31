use std::collections::HashMap;
use std::time::Duration;

use base64::Engine;
use rand::RngExt;
use rand_distr::{Distribution, Normal};

use crate::cdp::dom::{BoxModel, DomCommands};
use crate::cdp::input::{
    DispatchKeyEventParams, DispatchTouchEventParams, InputCommands, TouchPoint,
};
use crate::cdp::page::{CaptureScreenshotParams, PageCommands, Viewport};
use crate::error::{CdpError, Result};
use crate::keyboard::{self, KeySequence};
use crate::runtime::JsObject;
use crate::session::CdpSession;

/// Timing configuration for human-like input simulation.
///
/// All delays are sampled from a gaussian (normal) distribution clamped to
/// avoid negative or extreme values. Use [`HumanDelay::default()`] for
/// realistic typing, or [`HumanDelay::INSTANT`] for tests.
///
/// # Example
///
/// ```rust
/// use chromium_driver::dom::HumanDelay;
///
/// // Realistic defaults
/// let timing = HumanDelay::default();
///
/// // Zero delays for tests
/// let fast = HumanDelay::INSTANT;
///
/// // Custom
/// let custom = HumanDelay {
///     key_interval_mean_ms: 150.0,
///     key_interval_stddev_ms: 40.0,
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Copy)]
pub struct HumanDelay {
    /// Mean delay between keystrokes in milliseconds (default: 100).
    pub key_interval_mean_ms: f64,
    /// Stddev for keystroke interval in milliseconds (default: 30).
    pub key_interval_stddev_ms: f64,
    /// Mean duration a key is held down in milliseconds (default: 50).
    pub key_hold_mean_ms: f64,
    /// Stddev for key hold in milliseconds (default: 15).
    pub key_hold_stddev_ms: f64,
    /// Max random offset in pixels from element center for taps (default: 3.0).
    pub click_offset_max_px: f64,
    /// Mean delay before tapping — simulates locating the element (default: 200).
    pub pre_click_mean_ms: f64,
    /// Stddev for pre-tap delay (default: 80).
    pub pre_click_stddev_ms: f64,
    /// Mean duration finger stays on screen during tap (default: 70).
    pub touch_hold_mean_ms: f64,
    /// Stddev for touch hold (default: 25).
    pub touch_hold_stddev_ms: f64,
    /// Mean delay after tapping — waits for UI reaction (default: 300).
    pub post_click_mean_ms: f64,
    /// Stddev for post-tap delay (default: 100).
    pub post_click_stddev_ms: f64,
}

impl HumanDelay {
    /// Zero delays and no jitter. Use in tests for deterministic, fast execution.
    pub const INSTANT: Self = Self {
        key_interval_mean_ms: 0.0,
        key_interval_stddev_ms: 0.0,
        key_hold_mean_ms: 0.0,
        key_hold_stddev_ms: 0.0,
        click_offset_max_px: 0.0,
        pre_click_mean_ms: 0.0,
        pre_click_stddev_ms: 0.0,
        touch_hold_mean_ms: 0.0,
        touch_hold_stddev_ms: 0.0,
        post_click_mean_ms: 0.0,
        post_click_stddev_ms: 0.0,
    };
}

impl Default for HumanDelay {
    fn default() -> Self {
        Self {
            key_interval_mean_ms: 100.0,
            key_interval_stddev_ms: 30.0,
            key_hold_mean_ms: 50.0,
            key_hold_stddev_ms: 15.0,
            click_offset_max_px: 3.0,
            pre_click_mean_ms: 200.0,
            pre_click_stddev_ms: 80.0,
            touch_hold_mean_ms: 70.0,
            touch_hold_stddev_ms: 25.0,
            post_click_mean_ms: 300.0,
            post_click_stddev_ms: 100.0,
        }
    }
}

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
/// # Example
///
/// ```rust,no_run
/// # async fn example(page: &chromium_driver::PageSession) -> chromium_driver::Result<()> {
/// use chromium_driver::dom::{Dom, HumanDelay};
/// use std::time::Duration;
///
/// let dom = Dom::enable(page.cdp()).await?;
/// let timing = HumanDelay::default();
/// let el = dom.wait_for("div[role='textbox']", Duration::from_secs(5)).await?;
/// el.click(&timing).await?;
/// el.type_text("hello", &timing).await?;
/// el.press_key("Enter", &timing).await?;
/// dom.disable().await?;
/// # Ok(())
/// # }
/// ```
pub struct Dom {
    cdp: CdpSession,
}

impl Dom {
    pub(crate) fn new(cdp: CdpSession) -> Self {
        Self { cdp }
    }

    /// Enables the DOM domain and returns a new `Dom` handle.
    pub async fn enable(cdp: &CdpSession) -> Result<Self> {
        cdp.dom_enable().await?;
        Ok(Self { cdp: cdp.clone() })
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
        if qs.node_id > 0 {
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
            .filter(|id| *id > 0)
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

    async fn root_id(&self) -> Result<i64> {
        let doc = self.cdp.dom_get_document(0).await?;
        Ok(doc.root.node_id)
    }
}

/// A reference to a DOM element with methods for inspection and interaction.
///
/// Obtained from [`Dom::query_selector`], [`Dom::query_selector_all`],
/// [`Dom::wait_for`], or [`Element::query_selector`].
///
/// Interaction methods (`click`, `type_text`, `press_key`) take a [`HumanDelay`]
/// parameter that controls gaussian-distributed timing for realistic behavior.
pub struct Element {
    cdp: CdpSession,
    node_id: i64,
}

impl Element {
    /// Returns the CDP node ID of this element.
    pub fn node_id(&self) -> i64 {
        self.node_id
    }

    // --- Inspection ---

    /// Returns the outer HTML of this element.
    pub async fn outer_html(&self) -> Result<String> {
        let ret = self.cdp.dom_get_outer_html(self.node_id).await?;
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
        let ret = self.cdp.dom_get_box_model(self.node_id).await?;
        Ok(ret.model)
    }

    // --- Scoped queries ---

    /// Finds the first child element matching a CSS selector within this element.
    pub async fn try_query_selector(&self, selector: &str) -> Result<Option<Element>> {
        let qs = self.cdp.dom_query_selector(self.node_id, selector).await?;
        if qs.node_id > 0 {
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
            .filter(|id| *id > 0)
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
        let ret = self.cdp.dom_resolve_node(self.node_id, None).await?;
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
    pub async fn click(&self, timing: &HumanDelay) -> Result<()> {
        let (cx, cy) = self.center().await?;
        let x = cx + jitter_offset(timing.click_offset_max_px);
        let y = cy + jitter_offset(timing.click_offset_max_px);

        // Pre-tap delay: human locates the element and moves finger
        human_sleep(
            timing.pre_click_mean_ms,
            timing.pre_click_stddev_ms,
        )
        .await;

        // Finger touches screen
        self.cdp
            .input_dispatch_touch_event(&DispatchTouchEventParams {
                event_type: "touchStart".into(),
                touch_points: vec![TouchPoint { x, y }],
            })
            .await?;

        // Finger hold duration
        human_sleep(timing.touch_hold_mean_ms, timing.touch_hold_stddev_ms).await;

        // Finger lifts
        self.cdp
            .input_dispatch_touch_event(&DispatchTouchEventParams {
                event_type: "touchEnd".into(),
                touch_points: vec![],
            })
            .await?;

        // Post-tap delay: wait for UI reaction before next action
        human_sleep(
            timing.post_click_mean_ms,
            timing.post_click_stddev_ms,
        )
        .await;

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
    pub async fn type_text(&self, text: &str, timing: &HumanDelay) -> Result<()> {
        for ch in text.chars() {
            let seq = keyboard::abnt2_sequence(ch);
            let ch_str = ch.to_string();

            match seq {
                KeySequence::Simple { key, code, shift, vk } => {
                    self.dispatch_key_press(key, code, &ch_str, shift, vk, timing).await?;
                }
                KeySequence::DeadKey {
                    dead_key, dead_code, dead_shift, dead_vk,
                    base_key, base_code, base_shift, base_vk,
                } => {
                    // Dead key press (no text inserted)
                    self.dispatch_dead_key(dead_key, dead_code, dead_shift, dead_vk, timing).await?;

                    human_sleep(timing.key_interval_mean_ms, timing.key_interval_stddev_ms).await;

                    // Base key press (produces the composed character)
                    self.dispatch_key_press(base_key, base_code, &ch_str, base_shift, base_vk, timing).await?;
                }
                KeySequence::Paste => {
                    self.dispatch_paste(&ch_str, timing).await?;
                }
            }

            // Inter-key delay
            human_sleep(timing.key_interval_mean_ms, timing.key_interval_stddev_ms).await;
        }
        Ok(())
    }

    /// Presses a special key (e.g. `"Enter"`, `"Tab"`, `"Escape"`, `"Backspace"`)
    /// with human-like key hold timing.
    pub async fn press_key(&self, key: &str, timing: &HumanDelay) -> Result<()> {
        self.cdp
            .input_dispatch_key_event(&DispatchKeyEventParams {
                event_type: "keyDown".into(),
                key: Some(key.into()),
                ..Default::default()
            })
            .await?;

        human_sleep(timing.key_hold_mean_ms, timing.key_hold_stddev_ms).await;

        self.cdp
            .input_dispatch_key_event(&DispatchKeyEventParams {
                event_type: "keyUp".into(),
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
        timing: &HumanDelay,
    ) -> Result<()> {
        let modifiers = if shift { Some(8) } else { None };

        // rawKeyDown
        self.cdp
            .input_dispatch_key_event(&DispatchKeyEventParams {
                event_type: "rawKeyDown".into(),
                key: Some(key.into()),
                code: Some(code.into()),
                modifiers,
                ..Default::default()
            })
            .await?;

        human_sleep(timing.key_hold_mean_ms, timing.key_hold_stddev_ms).await;

        // char (inserts the text)
        self.cdp
            .input_dispatch_key_event(&DispatchKeyEventParams {
                event_type: "char".into(),
                text: Some(text.into()),
                ..Default::default()
            })
            .await?;

        // keyUp
        self.cdp
            .input_dispatch_key_event(&DispatchKeyEventParams {
                event_type: "keyUp".into(),
                key: Some(key.into()),
                code: Some(code.into()),
                modifiers,
                ..Default::default()
            })
            .await?;

        Ok(())
    }

    /// Dispatches a dead key press/release (no text inserted).
    async fn dispatch_dead_key(
        &self,
        key: &str,
        code: &str,
        shift: bool,
        _vk: i32,
        timing: &HumanDelay,
    ) -> Result<()> {
        let modifiers = if shift { Some(8) } else { None };

        self.cdp
            .input_dispatch_key_event(&DispatchKeyEventParams {
                event_type: "rawKeyDown".into(),
                key: Some(key.into()),
                code: Some(code.into()),
                modifiers,
                ..Default::default()
            })
            .await?;

        human_sleep(timing.key_hold_mean_ms, timing.key_hold_stddev_ms).await;

        self.cdp
            .input_dispatch_key_event(&DispatchKeyEventParams {
                event_type: "keyUp".into(),
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
    async fn dispatch_paste(&self, text: &str, _timing: &HumanDelay) -> Result<()> {
        self.cdp
            .call_no_response(
                "Input.insertText",
                &serde_json::json!({"text": text}),
            )
            .await?;

        Ok(())
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
                format: Some("png".into()),
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
    pub async fn swipe_up(&self, distance: f64, timing: &HumanDelay) -> Result<()> {
        let bm = self.box_model().await?;
        let q = &bm.content;
        if q.len() < 8 {
            return Err(CdpError::Protocol {
                code: -1,
                message: "invalid box model for swipe".into(),
            });
        }

        let cx = (q[0] + q[2]) / 2.0 + jitter_offset(timing.click_offset_max_px);
        let top_y = q[1];
        let bottom_y = q[5];
        let height = bottom_y - top_y;

        // Start near the bottom 75% of the element
        let start_y = top_y + height * 0.75 + jitter_offset(timing.click_offset_max_px);
        let end_y = (start_y - distance).max(top_y + 10.0);

        // Touch down
        self.cdp
            .input_dispatch_touch_event(&DispatchTouchEventParams {
                event_type: "touchStart".into(),
                touch_points: vec![TouchPoint { x: cx, y: start_y }],
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
                    event_type: "touchMove".into(),
                    touch_points: vec![TouchPoint { x: cx + jitter_offset(1.0), y }],
                })
                .await?;
        }

        // Release
        human_sleep(timing.touch_hold_mean_ms, timing.touch_hold_stddev_ms).await;
        self.cdp
            .input_dispatch_touch_event(&DispatchTouchEventParams {
                event_type: "touchEnd".into(),
                touch_points: vec![],
            })
            .await?;

        Ok(())
    }

    /// Swipes down on this element to scroll its content up (toward the top).
    ///
    /// Opposite of [`swipe_up`](Self::swipe_up): touch near the top, drag
    /// downward. `distance` is the total vertical pixels to drag.
    pub async fn swipe_down(&self, distance: f64, timing: &HumanDelay) -> Result<()> {
        let bm = self.box_model().await?;
        let q = &bm.content;
        if q.len() < 8 {
            return Err(CdpError::Protocol {
                code: -1,
                message: "invalid box model for swipe".into(),
            });
        }

        let cx = (q[0] + q[2]) / 2.0 + jitter_offset(timing.click_offset_max_px);
        let top_y = q[1];
        let bottom_y = q[5];
        let height = bottom_y - top_y;

        // Start near the top 25% of the element
        let start_y = top_y + height * 0.25 + jitter_offset(timing.click_offset_max_px);
        let end_y = (start_y + distance).min(bottom_y - 10.0);

        self.cdp
            .input_dispatch_touch_event(&DispatchTouchEventParams {
                event_type: "touchStart".into(),
                touch_points: vec![TouchPoint { x: cx, y: start_y }],
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
                    event_type: "touchMove".into(),
                    touch_points: vec![TouchPoint { x: cx + jitter_offset(1.0), y }],
                })
                .await?;
        }

        human_sleep(timing.touch_hold_mean_ms, timing.touch_hold_stddev_ms).await;
        self.cdp
            .input_dispatch_touch_event(&DispatchTouchEventParams {
                event_type: "touchEnd".into(),
                touch_points: vec![],
            })
            .await?;

        Ok(())
    }

    // --- Internal ---

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
