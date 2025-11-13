//! UI component types
//!
//! Data-oriented component definitions for UI elements following ECS principles.

use crate::foundation::math::Vec4;

/// Anchor point for UI positioning
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Anchor {
    /// Top-left corner (0, 0) in screen space
    TopLeft,
    /// Top-center
    TopCenter,
    /// Top-right corner
    TopRight,
    /// Middle-left
    MiddleLeft,
    /// Center of screen
    Center,
    /// Middle-right
    MiddleRight,
    /// Bottom-left corner
    BottomLeft,
    /// Bottom-center
    BottomCenter,
    /// Bottom-right corner
    BottomRight,
}

impl Anchor {
    /// Get the normalized anchor position (0.0 to 1.0)
    pub fn to_normalized(&self) -> (f32, f32) {
        match self {
            Anchor::TopLeft => (0.0, 0.0),
            Anchor::TopCenter => (0.5, 0.0),
            Anchor::TopRight => (1.0, 0.0),
            Anchor::MiddleLeft => (0.0, 0.5),
            Anchor::Center => (0.5, 0.5),
            Anchor::MiddleRight => (1.0, 0.5),
            Anchor::BottomLeft => (0.0, 1.0),
            Anchor::BottomCenter => (0.5, 1.0),
            Anchor::BottomRight => (1.0, 1.0),
        }
    }
}

/// Base UI element properties
#[derive(Debug, Clone)]
pub struct UIElement {
    /// Position in screen space (pixels from anchor point)
    pub position: (f32, f32),
    
    /// Size in pixels (width, height)
    pub size: (f32, f32),
    
    /// Anchor point for positioning
    pub anchor: Anchor,
    
    /// Whether this element is visible
    pub visible: bool,
    
    /// Z-order for layering (higher = on top)
    pub z_order: i32,
}

impl Default for UIElement {
    fn default() -> Self {
        Self {
            position: (0.0, 0.0),
            size: (100.0, 50.0),
            anchor: Anchor::TopLeft,
            visible: true,
            z_order: 0,
        }
    }
}

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

/// Horizontal text alignment
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HorizontalAlign {
    Left,
    Center,
    Right,
}

/// Vertical text alignment
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VerticalAlign {
    Top,
    Middle,
    Bottom,
}

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
    pub hover_color: Vec4,
    pub pressed_color: Vec4,
    pub disabled_color: Vec4,
    
    /// Text color
    pub text_color: Vec4,
    
    /// Border properties
    pub border_color: Vec4,
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
