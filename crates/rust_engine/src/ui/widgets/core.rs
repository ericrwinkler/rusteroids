//! Core UI widget primitives
//!
//! Shared types and structures used by all UI widgets.

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

/// Horizontal text alignment
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HorizontalAlign {
    /// Left-aligned text
    Left,
    /// Center-aligned text
    Center,
    /// Right-aligned text
    Right,
}

/// Vertical text alignment
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VerticalAlign {
    /// Top-aligned text
    Top,
    /// Middle-aligned text
    Middle,
    /// Bottom-aligned text
    Bottom,
}
