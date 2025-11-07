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
    text_layout: TextLayout,
}

impl TextRenderer {
    /// Create a new text renderer
    pub fn new(font_atlas: FontAtlas) -> Self {
        let text_layout = TextLayout::new(font_atlas.clone());
        Self {
            font_atlas,
            text_layout,
        }
    }
    
    /// Get the font atlas
    pub fn font_atlas(&self) -> &FontAtlas {
        &self.font_atlas
    }
    
    /// Get the text layout engine
    pub fn text_layout(&self) -> &TextLayout {
        &self.text_layout
    }
}
