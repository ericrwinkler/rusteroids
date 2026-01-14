//! Button widget - interactive clickable buttons

use super::core::UIElement;
use crate::foundation::math::Vec4;

/// Button state for visual feedback
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ButtonState {
    /// Normal resting state
    Normal,
    /// Mouse is hovering over button
    Hovered,
    /// Button is being pressed
    Pressed,
    /// Button is disabled (non-interactive)
    Disabled,
}

/// UI button component
#[derive(Debug, Clone)]
pub struct UIButton {
    /// Base element properties
    pub element: UIElement,
    
    /// Button label text
    pub text: String,
    
    /// Font size for label
    pub font_size: f32,
    
    /// Current button state
    pub state: ButtonState,
    
    /// Colors for different states
    pub normal_color: Vec4,
    /// Hover state color
    pub hover_color: Vec4,
    /// Pressed state color
    pub pressed_color: Vec4,
    /// Disabled state color
    pub disabled_color: Vec4,
    
    /// Text color
    pub text_color: Vec4,
    
    /// Border properties
    pub border_color: Vec4,
    /// Border width in pixels
    pub border_width: f32,
    
    /// Whether the button is enabled
    pub enabled: bool,
    
    /// Callback ID for click events (optional)
    pub on_click_id: Option<u32>,
}

impl Default for UIButton {
    fn default() -> Self {
        Self {
            element: UIElement::default(),
            text: String::new(),
            font_size: 20.0,
            state: ButtonState::Normal,
            normal_color: Vec4::new(0.3, 0.3, 0.3, 0.9),
            hover_color: Vec4::new(0.4, 0.4, 0.5, 1.0),
            pressed_color: Vec4::new(0.5, 0.5, 0.6, 1.0),
            disabled_color: Vec4::new(0.2, 0.2, 0.2, 0.5),
            text_color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            border_color: Vec4::new(0.6, 0.6, 0.6, 1.0),
            border_width: 2.0,
            enabled: true,
            on_click_id: None,
        }
    }
}

impl UIButton {
    /// Get the current color based on button state
    pub fn get_current_color(&self) -> Vec4 {
        if !self.enabled {
            return self.disabled_color;
        }
        
        match self.state {
            ButtonState::Normal => self.normal_color,
            ButtonState::Hovered => self.hover_color,
            ButtonState::Pressed => self.pressed_color,
            ButtonState::Disabled => self.disabled_color,
        }
    }
}
