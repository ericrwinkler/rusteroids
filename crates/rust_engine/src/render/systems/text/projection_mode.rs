//! Text projection modes for world-space and screen-space rendering

use crate::foundation::math::Vec2;

/// Text rendering projection mode
///
/// Determines how text is projected and positioned:
/// - `WorldSpace`: 3D perspective projection with camera billboarding
/// - `ScreenSpace`: 2D orthographic projection with fixed screen coordinates
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextProjectionMode {
    /// World-space 3D text with perspective projection
    ///
    /// - Camera billboarding (faces camera)
    /// - 3D transforms
    /// - Depth testing enabled
    /// - Used for damage numbers, labels, floating text
    WorldSpace,
    
    /// Screen-space 2D text with orthographic projection
    ///
    /// - Fixed screen coordinates (pixels)
    /// - No depth testing (UI overlays)
    /// - Anchor-based positioning
    /// - Used for UI, HUD, menus
    ScreenSpace,
}

/// Text position specification (supports both 2D and 3D)
#[derive(Debug, Clone, Copy)]
pub enum TextPosition {
    /// World-space 3D position
    World(crate::foundation::math::Vec3),
    
    /// Screen-space 2D position (pixels from top-left)
    Screen(Vec2),
}

impl TextPosition {
    /// Create world-space position
    pub fn world(x: f32, y: f32, z: f32) -> Self {
        TextPosition::World(crate::foundation::math::Vec3::new(x, y, z))
    }
    
    /// Create screen-space position
    pub fn screen(x: f32, y: f32) -> Self {
        TextPosition::Screen(Vec2::new(x, y))
    }
}
