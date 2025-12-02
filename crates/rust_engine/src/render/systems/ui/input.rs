//! UI input handling system
//!
//! DEPRECATED: This module has been moved to `input::ui_input`.
//! Input processing is not a rendering concern and has been properly separated.
//!
//! This re-export is maintained for backwards compatibility.

#[deprecated(
    since = "0.2.0",
    note = "Use `crate::input::ui_input` instead. Input processing has been moved out of the render module."
)]
pub use crate::input::ui_input::{MouseButton, UIInputEvent, UIInputProcessor};

/// Alias for backwards compatibility - use `UIInputEvent` instead
#[deprecated(since = "0.2.0", note = "Use UIInputEvent instead")]
pub type UIEvent = UIInputEvent;
