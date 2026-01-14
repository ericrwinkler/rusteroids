//! UI widgets module
//!
//! Contains all UI widget types (panels, buttons, text, etc.)

pub mod core;
pub mod panel;
pub mod text;
pub mod button;
pub mod layout;

// Re-export core types
pub use core::{Anchor, UIElement, HorizontalAlign, VerticalAlign};

// Re-export widget types
pub use panel::UIPanel;
pub use text::UIText;
pub use button::{UIButton, ButtonState};
pub use layout::UILayout;
