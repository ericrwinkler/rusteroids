//! Panel widget - rectangular backgrounds with optional borders

use super::core::UIElement;
use crate::foundation::math::Vec4;

/// UI panel component - a colored rectangle background
#[derive(Debug, Clone)]
pub struct UIPanel {
    /// Base element properties
    pub element: UIElement,
    
    /// Background color (RGBA)
    pub color: Vec4,
    
    /// Border color (RGBA), if border_width > 0
    pub border_color: Vec4,
    
    /// Border width in pixels
    pub border_width: f32,
    
    /// Corner rounding radius (0 = sharp corners)
    pub corner_radius: f32,
}

impl Default for UIPanel {
    fn default() -> Self {
        Self {
            element: UIElement::default(),
            color: Vec4::new(0.2, 0.2, 0.2, 0.8),
            border_color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            border_width: 0.0,
            corner_radius: 0.0,
        }
    }
}
