//! UI System Module
//! 
//! Provides a clean separation between UI logic and rendering backend.
//! 
//! Architecture:
//! - UIManager: Central UI system managing scene graph, input, and events
//! - widgets/: UI widget definitions (Panel, Button, Text)
//! - rendering/: Backend-agnostic rendering infrastructure
//! - input/: UI input processing

pub mod manager;
pub mod backend;
pub mod widgets;
pub mod rendering;
pub mod input;

pub use manager::UIManager;
pub use backend::UIRenderBackend;

// Re-export widgets
pub use widgets::{
    UIElement, UIPanel, UIText, UIButton, ButtonState,
    Anchor, HorizontalAlign, VerticalAlign, UILayout,
};

// Re-export rendering types
pub use rendering::{
    UIRenderer, UIRenderCommand, UIRenderData,
    RenderQuad, RenderText,
};

// Re-export input types
pub use input::{UIInputProcessor, UIInputEvent, MouseButton};

// Re-export events
pub use crate::events::EventSystem;

/// Unique identifier for UI elements
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UINodeId(pub u64);
