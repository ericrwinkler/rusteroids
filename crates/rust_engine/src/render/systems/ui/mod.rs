//! UI rendering system
//!
//! Provides a simple immediate-mode UI system for labels, panels, and buttons.
//! Designed following Game Engine Architecture principles:
//! - Data-oriented component design
//! - Separation of rendering and input logic
//! - Integration with existing ECS and rendering systems

pub mod components;
pub mod layout;
pub mod renderer;
pub mod input;

pub use components::*;
pub use layout::*;
pub use renderer::*;
pub use input::*;
