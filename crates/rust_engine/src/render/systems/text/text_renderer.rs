//! Text rendering utilities
//!
//! Simple wrapper providing utilities for text rendering.
//! Actual rendering happens through the pool system.

use super::{FontAtlas, TextLayout};

/// Simple text renderer that wraps font atlas and layout engine
/// 
/// Note: This is just a utility wrapper. Text rendering happens via
/// the pool system (MeshType::TextQuad).
pub struct TextRenderer {
    font_atlas: FontAtlas,
}

impl TextRenderer {
    /// Create a new text renderer
    pub fn new(font_atlas: FontAtlas) -> Self {
        Self { font_atlas }
    }
    
    /// Get the font atlas
    pub fn font_atlas(&self) -> &FontAtlas {
        &self.font_atlas
    }
    
    /// Create a text layout engine for this renderer's font atlas
    /// 
    /// TextLayout is cheap to create - it just holds a reference to the FontAtlas
    pub fn create_text_layout(&self) -> TextLayout<'_> {
        TextLayout::new(&self.font_atlas)
    }
}
