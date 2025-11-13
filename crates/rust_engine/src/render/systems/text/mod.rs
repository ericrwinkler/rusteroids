//! Text rendering system
//!
//! Font atlas management, text layout, and text rendering.

pub mod text_renderer;
pub mod font_atlas;
pub mod text_layout;
pub mod text_material;
pub mod world_text;
pub mod projection_mode;

pub use text_renderer::*;
pub use font_atlas::*;
pub use text_layout::*;
pub use text_material::*;
pub use world_text::*;
pub use projection_mode::*;
