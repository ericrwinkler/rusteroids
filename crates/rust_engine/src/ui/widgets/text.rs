//! Text widget - labels and text display

use super::core::{UIElement, HorizontalAlign, VerticalAlign};
use crate::foundation::math::Vec4;

/// UI text label component
#[derive(Debug, Clone)]
pub struct UIText {
    /// Base element properties
    pub element: UIElement,
    
    /// Text content to display
    pub text: String,
    
    /// Font size in pixels
    pub font_size: f32,
    
    /// Text color (RGBA)
    pub color: Vec4,
    
    /// Horizontal alignment within the element bounds
    pub h_align: HorizontalAlign,
    
    /// Vertical alignment within the element bounds
    pub v_align: VerticalAlign,
}

impl Default for UIText {
    fn default() -> Self {
        Self {
            element: UIElement::default(),
            text: String::new(),
            font_size: 24.0,
            color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            h_align: HorizontalAlign::Left,
            v_align: VerticalAlign::Top,
        }
    }
}
