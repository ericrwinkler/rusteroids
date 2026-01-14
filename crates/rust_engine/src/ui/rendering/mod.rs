//! UI rendering module
//!
//! Backend-agnostic UI rendering infrastructure

pub mod data;
pub mod vertex;
pub mod commands;
pub mod renderer;

// Re-export commonly used types
pub use data::{UIRenderData, RenderQuad, RenderText};
pub use vertex::{UIVertex, PanelVertex};
pub use commands::UIRenderCommand;
pub use renderer::{UIRenderer, render_ui_data};
