//! Debug drawing primitives and system
//!
//! Based on Game Engine Architecture 3rd Edition, Section 10.2:
//! "Debug drawing facilities allow programmers to render simple shapes like
//! lines, points, spheres and boxes for debugging and visualization purposes."

use crate::foundation::math::{Vec3, Vec4, Quat};
use std::collections::HashMap;

/// Unique identifier for persistent debug shapes
pub type DebugShapeId = String;

/// Debug shape primitives that can be rendered for visualization
#[derive(Clone, Debug)]
pub enum DebugShape {
    /// Line segment from start to end
    Line {
        start: Vec3,
        end: Vec3,
        color: Vec4,
        duration: f32,
    },
    
    /// Sphere at center with radius
    Sphere {
        center: Vec3,
        radius: f32,
        color: Vec4,
        duration: f32,
        wireframe: bool,
    },
    
    /// Box (AABB or OBB) at center with half-extents
    Box {
        center: Vec3,
        extents: Vec3,
        rotation: Quat,
        color: Vec4,
        duration: f32,
        wireframe: bool,
    },
    
    /// Capsule from start to end with radius
    Capsule {
        start: Vec3,
        end: Vec3,
        radius: f32,
        color: Vec4,
        duration: f32,
    },
    
    /// Point at position
    Point {
        position: Vec3,
        color: Vec4,
        size: f32,
        duration: f32,
    },
}

impl DebugShape {
    /// Get remaining duration
    pub fn duration(&self) -> f32 {
        match self {
            DebugShape::Line { duration, .. } => *duration,
            DebugShape::Sphere { duration, .. } => *duration,
            DebugShape::Box { duration, .. } => *duration,
            DebugShape::Capsule { duration, .. } => *duration,
            DebugShape::Point { duration, .. } => *duration,
        }
    }
    
    /// Set duration (returns modified shape)
    pub fn with_duration(mut self, new_duration: f32) -> Self {
        match &mut self {
            DebugShape::Line { duration, .. } => *duration = new_duration,
            DebugShape::Sphere { duration, .. } => *duration = new_duration,
            DebugShape::Box { duration, .. } => *duration = new_duration,
            DebugShape::Capsule { duration, .. } => *duration = new_duration,
            DebugShape::Point { duration, .. } => *duration = new_duration,
        }
        self
    }
    
    /// Decrease duration by delta_time, returns true if expired
    pub fn tick(&mut self, delta_time: f32) -> bool {
        match self {
            DebugShape::Line { duration, .. } => {
                *duration -= delta_time;
                *duration <= 0.0
            }
            DebugShape::Sphere { duration, .. } => {
                *duration -= delta_time;
                *duration <= 0.0
            }
            DebugShape::Box { duration, .. } => {
                *duration -= delta_time;
                *duration <= 0.0
            }
            DebugShape::Capsule { duration, .. } => {
                *duration -= delta_time;
                *duration <= 0.0
            }
            DebugShape::Point { duration, .. } => {
                *duration -= delta_time;
                *duration <= 0.0
            }
        }
    }
}

/// Debug drawing system for rendering debug shapes
/// 
/// GEA 10.2: "Debug rendering systems typically support both temporary shapes
/// (which expire after a certain time) and persistent shapes (which remain
/// until explicitly removed)."
pub struct DebugDrawSystem {
    /// Temporary shapes that expire after their duration
    temporary_shapes: Vec<DebugShape>,
    
    /// Persistent shapes that remain until manually removed
    persistent_shapes: HashMap<DebugShapeId, DebugShape>,
    
    /// Master enable/disable flag
    pub enabled: bool,
}

impl DebugDrawSystem {
    /// Create a new debug draw system
    pub fn new() -> Self {
        Self {
            temporary_shapes: Vec::new(),
            persistent_shapes: HashMap::new(),
            enabled: true,
        }
    }
    
    /// Draw a line segment (temporary)
    pub fn draw_line(&mut self, start: Vec3, end: Vec3, color: Vec4, duration: f32) {
        if !self.enabled {
            return;
        }
        
        self.temporary_shapes.push(DebugShape::Line {
            start,
            end,
            color,
            duration,
        });
    }
    
    /// Draw a sphere (temporary)
    pub fn draw_sphere(&mut self, center: Vec3, radius: f32, color: Vec4, duration: f32) {
        if !self.enabled {
            return;
        }
        
        self.temporary_shapes.push(DebugShape::Sphere {
            center,
            radius,
            color,
            duration,
            wireframe: true,
        });
    }
    
    /// Draw a box (temporary)
    pub fn draw_box(&mut self, center: Vec3, extents: Vec3, color: Vec4, duration: f32) {
        if !self.enabled {
            return;
        }
        
        self.temporary_shapes.push(DebugShape::Box {
            center,
            extents,
            rotation: Quat::identity(),
            color,
            duration,
            wireframe: true,
        });
    }
    
    /// Draw a point (temporary)
    pub fn draw_point(&mut self, position: Vec3, color: Vec4, size: f32, duration: f32) {
        if !self.enabled {
            return;
        }
        
        self.temporary_shapes.push(DebugShape::Point {
            position,
            color,
            size,
            duration,
        });
    }
    
    /// Draw a persistent shape that remains until explicitly removed
    pub fn draw_persistent(&mut self, id: impl Into<String>, shape: DebugShape) {
        if !self.enabled {
            return;
        }
        
        self.persistent_shapes.insert(id.into(), shape);
    }
    
    /// Remove a persistent shape
    pub fn clear_persistent(&mut self, id: &str) {
        self.persistent_shapes.remove(id);
    }
    
    /// Clear all persistent shapes
    pub fn clear_all_persistent(&mut self) {
        self.persistent_shapes.clear();
    }
    
    /// Update shape lifetimes and remove expired temporary shapes
    pub fn update(&mut self, delta_time: f32) {
        if !self.enabled {
            return;
        }
        
        // Remove expired temporary shapes
        self.temporary_shapes.retain_mut(|shape| !shape.tick(delta_time));
    }
    
    /// Get all shapes for rendering (both temporary and persistent)
    pub fn get_shapes(&self) -> Vec<&DebugShape> {
        if !self.enabled {
            return Vec::new();
        }
        
        self.temporary_shapes.iter()
            .chain(self.persistent_shapes.values())
            .collect()
    }
    
    /// Get the number of active shapes
    pub fn shape_count(&self) -> usize {
        self.temporary_shapes.len() + self.persistent_shapes.len()
    }
    
    /// Clear all shapes (temporary and persistent)
    pub fn clear(&mut self) {
        self.temporary_shapes.clear();
        self.persistent_shapes.clear();
    }
}

impl Default for DebugDrawSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_temporary_shape_expiration() {
        let mut system = DebugDrawSystem::new();
        
        // Add a shape with 1 second duration
        system.draw_sphere(
            Vec3::zeros(),
            1.0,
            Vec4::new(1.0, 0.0, 0.0, 1.0),
            1.0,
        );
        
        assert_eq!(system.shape_count(), 1);
        
        // Update for 0.5 seconds
        system.update(0.5);
        assert_eq!(system.shape_count(), 1);
        
        // Update for another 0.6 seconds (total 1.1 seconds)
        system.update(0.6);
        assert_eq!(system.shape_count(), 0);
    }
    
    #[test]
    fn test_persistent_shapes() {
        let mut system = DebugDrawSystem::new();
        
        // Add persistent shape
        system.draw_persistent(
            "test_sphere",
            DebugShape::Sphere {
                center: Vec3::zeros(),
                radius: 1.0,
                color: Vec4::new(1.0, 0.0, 0.0, 1.0),
                duration: f32::INFINITY,
                wireframe: true,
            },
        );
        
        assert_eq!(system.shape_count(), 1);
        
        // Update many times
        for _ in 0..100 {
            system.update(1.0);
        }
        
        // Should still exist
        assert_eq!(system.shape_count(), 1);
        
        // Remove it
        system.clear_persistent("test_sphere");
        assert_eq!(system.shape_count(), 0);
    }
}
