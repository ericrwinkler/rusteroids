//! Text rendering system
//!
//! This module provides text rendering capabilities for both 3D world text
//! (billboards, floating damage numbers) and 2D screen-space UI (HUD, menus).
//!
//! # Architecture
//!
//! - [`FontAtlas`]: Manages font texture atlas and glyph metadata
//! - [`TextLayout`]: Converts strings to positioned quads for rendering
//! - [`create_text_material`]: Helper for creating text materials
//!
//! # Example
//!
//! ```no_run
//! use rust_engine::render::text::{FontAtlas, TextLayout, create_text_material};
//! use rust_engine::foundation::math::Vec3;
//!
//! // Load font and create atlas
//! let font_data = std::fs::read("resources/fonts/default.ttf")?;
//! let mut font_atlas = FontAtlas::new(&font_data, 48.0)?;
//! let texture_handle = font_atlas.upload_to_gpu(&mut texture_manager)?;
//!
//! // Layout text string
//! let layout = TextLayout::new(font_atlas.clone());
//! let (vertices, indices) = layout.layout_text("Hello World");
//!
//! // Create material for rendering
//! let white = Vec3::new(1.0, 1.0, 1.0);
//! let material = create_text_material(texture_handle, white);
//! ```

mod font_atlas;
mod text_layout;
mod text_material;
mod text_renderer;
mod world_text;

pub use font_atlas::{FontAtlas, GlyphInfo};
pub use text_layout::{TextLayout, TextVertex, TextBounds, text_vertices_to_mesh};
pub use text_material::{create_lit_text_material, create_unlit_text_material};
pub use text_renderer::TextRenderer;
pub use world_text::{WorldTextManager, WorldTextHandle, WorldTextConfig};
