//! Shared CDP types, re-exported from the (generated) modules that own them so
//! the whole crate uses a single definition of each. See `build.rs`.

pub use crate::cdp::browser::BrowserContextId;
pub use crate::cdp::common::MonotonicTime;
pub use crate::cdp::page::{Frame, FrameId, NavigationEntry};
pub use crate::cdp::target::{SessionId, TargetId, TargetInfo};
