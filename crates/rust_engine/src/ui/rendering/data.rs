//! Backend-agnostic UI rendering data structures

use crate::foundation::math::{Vec2, Vec4};
use crate::render::resources::materials::TextureHandle;

/// Backend-agnostic UI rendering data
/// 
/// UI Manager exports this structure containing all UI elements to render.
/// Renderer consumes this without needing to know UI node hierarchy.
#[derive(Debug, Clone, Default)]
pub struct UIRenderData {
    /// UI quads (panels, buttons, sprites)
    pub quads: Vec<RenderQuad>,
    
    /// UI text elements
    pub texts: Vec<RenderText>,
}

impl UIRenderData {
    /// Create empty UI render data with no elements
    pub fn empty() -> Self {
        Self::default()
    }
}

/// UI quad primitive (panel, button background, sprite) for rendering
#[derive(Debug, Clone)]
pub struct RenderQuad {
    /// Screen position (pixels from top-left)
    pub position: Vec2,
    
    /// Size (width, height in pixels)
    pub size: Vec2,
    
    /// Color (RGBA)
    pub color: Vec4,
    
    /// Optional texture
    pub texture: Option<TextureHandle>,
    
    /// Z-order for layering (higher = front)
    pub depth: f32,
}

/// UI text primitive for rendering
#[derive(Debug, Clone)]
pub struct RenderText {
    /// Screen position (pixels from top-left)
    pub position: Vec2,
    
    /// Text content
    pub text: String,
    
    /// Font ID (index into font atlas)
    pub font: usize,
    
    /// Text color (RGBA)
    pub color: Vec4,
    
    /// Font size (pixels)
    pub size: f32,
    
    /// Z-order for layering (higher = front)
    pub depth: f32,
}
