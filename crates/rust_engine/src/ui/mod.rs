//! UI System Module
//! 
//! Provides a clean separation between UI logic and rendering backend.
//! 
//! Architecture:
//! - UIManager: Central UI system managing scene graph, input, and events
//! - Components: Reuse existing components from render::systems::ui
//! - UIRenderBackend: Trait for backend-agnostic rendering

pub mod manager;
pub mod backend;

pub use manager::UIManager;
pub use backend::UIRenderBackend;

// Re-export existing UI components and systems
pub use crate::render::systems::ui::{
    UIRenderer, UIRenderCommand,
    UIElement, UIPanel, UIText, UIButton, ButtonState,
    Anchor, HorizontalAlign, VerticalAlign,
};

// Re-export for external use
pub use crate::input::ui_input::{UIInputProcessor, MouseButton};
pub use crate::events::EventSystem;

/// Unique identifier for UI elements
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UINodeId(pub u64);
