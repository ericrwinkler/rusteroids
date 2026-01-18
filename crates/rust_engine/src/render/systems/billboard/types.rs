//! Billboard data structures and types
//!
//! Billboards are rendered using separate mesh pools per texture type.
//! See MeshType enum for supported billboard variants (Billboard, BillboardBullet, BillboardExplosion).

use crate::foundation::math::{Vec2, Vec3, Vec4, Mat4};

/// Billboard orientation mode
#[derive(Debug, Clone, Copy)]
pub enum BillboardOrientation {
    /// Always faces camera (perpendicular to view direction)
    ScreenAligned,
    
    /// One axis locked to velocity/direction, rotates around it to face camera
    VelocityAligned {
        /// Direction vector to align the billboard's right axis with
        direction: Vec3
    },
    
    /// One axis locked to world axis (e.g., Y-up), rotates around it to face camera
    WorldAxisAligned {
        /// World axis to lock the billboard's up axis to
        axis: Vec3
    },
}

/// Individual billboard quad data
#[derive(Debug, Clone)]
pub struct BillboardQuad {
    /// World position of billboard center
    pub position: Vec3,
    
    /// Size in world units (width, height)
    pub size: Vec2,
    
    /// RGBA color tint
    pub color: Vec4,
    
    /// Billboard orientation mode
    pub orientation: BillboardOrientation,
    
    /// UV coordinates for the four corners [bottom-left, bottom-right, top-right, top-left]
    pub uv_coords: [Vec2; 4],
}

impl BillboardQuad {
    /// Create a new billboard with default settings
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            size: Vec2::new(1.0, 1.0),
            color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            orientation: BillboardOrientation::ScreenAligned,
            uv_coords: [
                Vec2::new(0.0, 0.0), // bottom-left
                Vec2::new(1.0, 0.0), // bottom-right
                Vec2::new(1.0, 1.0), // top-right
                Vec2::new(0.0, 1.0), // top-left
            ],
        }
    }
    
    /// Set billboard size
    pub fn with_size(mut self, size: Vec2) -> Self {
        self.size = size;
        self
    }
    
    /// Set billboard color
    pub fn with_color(mut self, color: Vec4) -> Self {
        self.color = color;
        self
    }
    
    /// Set billboard orientation to velocity-aligned
    pub fn with_velocity_alignment(mut self, direction: Vec3) -> Self {
        self.orientation = BillboardOrientation::VelocityAligned { direction };
        self
    }
    
    /// Set billboard orientation to world-axis-aligned
    pub fn with_world_axis_alignment(mut self, axis: Vec3) -> Self {
        self.orientation = BillboardOrientation::WorldAxisAligned { axis };
        self
    }
}

/// GPU instance data for billboard rendering
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BillboardInstance {
    /// Model-to-world transformation matrix
    pub transform: Mat4,
    
    /// RGBA color tint
    pub color: Vec4,
    
    /// UV offset and scale (xy = offset, zw = scale)
    pub uv_offset_scale: Vec4,
}

/// Blend mode for billboard rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlendMode {
    /// No blending (fully opaque)
    Opaque,
    
    /// Standard alpha blending
    AlphaBlend,
    
    /// Additive blending (for glowing effects)
    Additive,
}

impl Default for BlendMode {
    fn default() -> Self {
        BlendMode::AlphaBlend
    }
}
