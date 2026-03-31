use crate::error::Result;
use crate::session::CdpSession;

/// `Emulation` domain CDP methods.
///
/// Reference: <https://chromedevtools.github.io/devtools-protocol/tot/Emulation/>
pub trait EmulationCommands {
    /// Enables or disables touch event emulation.
    ///
    /// When enabled, the browser reports `ontouchstart` as supported and
    /// dispatches touch events instead of mouse events for CDP input.
    ///
    /// CDP: `Emulation.setTouchEmulationEnabled`
    async fn emulation_set_touch_enabled(&self, enabled: bool) -> Result<()>;
}

impl EmulationCommands for CdpSession {
    async fn emulation_set_touch_enabled(&self, enabled: bool) -> Result<()> {
        self.call_no_response(
            "Emulation.setTouchEmulationEnabled",
            &serde_json::json!({"enabled": enabled}),
        )
        .await
    }
}
