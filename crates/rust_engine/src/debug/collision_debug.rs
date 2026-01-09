//! Collision-specific debug visualization
//!
//! Based on Game Engine Architecture 3rd Edition, Section 10.2:
//! "Debug drawing for collision detection typically includes visualizations
//! of bounding volumes, collision shapes, and query results."

use crate::debug::draw::{DebugDrawSystem, DebugShape};
use crate::foundation::math::{Vec3, Vec4};
use crate::physics::collision::CollisionShape;
use crate::ecs::Entity;

/// Color scheme for collision visualization
#[derive(Clone, Debug)]
pub struct CollisionDebugColors {
    /// Color for broad-phase query spheres
    pub broad_phase: Vec4,
    
    /// Color for collision shapes (not colliding)
    pub shape_default: Vec4,
    
    /// Color for collision shapes (currently colliding)
    pub shape_colliding: Vec4,
    
    /// Color for nearby entities (broad-phase candidates)
    pub nearby: Vec4,
}

impl Default for CollisionDebugColors {
    fn default() -> Self {
        Self {
            broad_phase: Vec4::new(0.5, 0.8, 1.0, 0.15),   // Light blue, transparent
            shape_default: Vec4::new(0.0, 1.0, 0.0, 0.3),  // Green, semi-transparent
            shape_colliding: Vec4::new(1.0, 0.0, 0.0, 0.5), // Red, semi-transparent
            nearby: Vec4::new(0.0, 1.0, 1.0, 0.2),          // Cyan, transparent
        }
    }
}

/// Collision-specific debug visualizer
/// 
/// Integrates with DebugDrawSystem to provide collision-specific visualization:
/// - Broad-phase query spheres
/// - Collision shapes
/// - Collision contacts (future enhancement)
pub struct CollisionDebugVisualizer {
    debug_draw: DebugDrawSystem,
    colors: CollisionDebugColors,
    
    /// Show broad-phase query spheres
    pub show_broad_phase: bool,
    
    /// Show collision shapes
    pub show_shapes: bool,
    
    /// Show nearby entities (broad-phase candidates)
    pub show_nearby: bool,
}

impl CollisionDebugVisualizer {
    /// Create a new collision debug visualizer
    pub fn new() -> Self {
        Self {
            debug_draw: DebugDrawSystem::new(),
            colors: CollisionDebugColors::default(),
            show_broad_phase: true,
            show_shapes: true,
            show_nearby: false,
        }
    }
    
    /// Set custom color scheme
    pub fn with_colors(mut self, colors: CollisionDebugColors) -> Self {
        self.colors = colors;
        self
    }
    
    /// Visualize broad-phase query sphere for an entity
    /// This shows the radius used by the spatial query to find nearby entities
    pub fn draw_broad_phase_query(
        &mut self,
        entity: Entity,
        position: Vec3,
        radius: f32,
    ) {
        if !self.show_broad_phase {
            return;
        }
        
        // Draw semi-transparent sphere showing query range
        // The radius matches what query_nearby uses (typically entity_radius * 2.0)
        let key = format!("broad_phase_{}", entity.id());
        self.debug_draw.draw_persistent(
            key,
            DebugShape::Sphere {
                center: position,
                radius,
                color: self.colors.broad_phase,
                duration: f32::INFINITY,
                wireframe: true,
            }
        );
    }
    
    /// Visualize a collision shape
    pub fn draw_collision_shape(
        &mut self,
        entity: Entity,
        shape: &CollisionShape,
        is_colliding: bool,
    ) {
        if !self.show_shapes {
            return;
        }
        
        let color = if is_colliding {
            self.colors.shape_colliding
        } else {
            self.colors.shape_default
        };
        
        // Note: For visualization, we just show local bounding sphere (no transform applied)
        // For accurate visualization, would need TransformComponent passed in
        match shape {
            CollisionShape::Sphere(radius) => {
                // TODO: Need position from TransformComponent for accurate visualization
                // For now, draw at origin (visualization will be inaccurate)
                self.debug_draw.draw_sphere(
                    Vec3::zeros(), // Placeholder - needs position from TransformComponent
                    *radius,
                    color,
                    0.0, // One frame
                );
            }
            CollisionShape::Mesh(template) => {
                // Draw local bounding sphere (at origin)
                // TODO: Need position/rotation from TransformComponent for accurate visualization
                self.debug_draw.draw_sphere(
                    Vec3::zeros(), // Placeholder
                    template.local_bounding_radius,
                    color,
                    0.0,
                );
            }
        }
    }
    
    /// Visualize nearby entities (broad-phase candidates)
    pub fn draw_nearby_indicator(
        &mut self,
        position: Vec3,
        radius: f32,
    ) {
        if !self.show_nearby {
            return;
        }
        
        self.debug_draw.draw_sphere(
            position,
            radius,
            self.colors.nearby,
            0.0,
        );
    }
    
    /// Clear broad-phase visualization for an entity
    pub fn clear_broad_phase(&mut self, entity: Entity) {
        self.debug_draw.clear_persistent(&format!("broad_phase_{}", entity.id()));
    }
    
    /// Clear all visualization
    pub fn clear(&mut self) {
        self.debug_draw.clear();
    }
    
    /// Update debug system (expire temporary shapes)
    pub fn update(&mut self, delta_time: f32) {
        self.debug_draw.update(delta_time);
    }
    
    /// Get all debug shapes for rendering
    pub fn get_shapes(&self) -> Vec<&DebugShape> {
        self.debug_draw.get_shapes()
    }
    
    /// Enable/disable the entire debug system
    pub fn set_enabled(&mut self, enabled: bool) {
        self.debug_draw.enabled = enabled;
    }
    
    /// Check if debug system is enabled
    pub fn is_enabled(&self) -> bool {
        self.debug_draw.enabled
    }
    
    /// Get reference to underlying debug draw system
    pub fn debug_draw(&self) -> &DebugDrawSystem {
        &self.debug_draw
    }
    
    /// Get mutable reference to underlying debug draw system
    pub fn debug_draw_mut(&mut self) -> &mut DebugDrawSystem {
        &mut self.debug_draw
    }
}

impl Default for CollisionDebugVisualizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::collision::BoundingSphere;
    
    #[test]
    fn test_broad_phase_visualization() {
        let mut viz = CollisionDebugVisualizer::new();
        let entity = Entity::new(1, 0);
        
        viz.draw_broad_phase_query(entity, Vec3::zeros(), 10.0);
        
        // Should have one persistent shape
        let shape_count = viz.get_shapes().count();
        assert_eq!(shape_count, 1);
        
        // Clear it
        viz.clear_broad_phase(entity);
        let shape_count = viz.get_shapes().count();
        assert_eq!(shape_count, 0);
    }
    
    #[test]
    fn test_collision_shape_visualization() {
        let mut viz = CollisionDebugVisualizer::new();
        let entity = Entity::new(1, 0);
        
        let shape = CollisionShape::Sphere(BoundingSphere::new(Vec3::zeros(), 5.0));
        
        // Draw shape (temporary, duration = 0.0)
        viz.draw_collision_shape(entity, &shape, false);
        
        // Should have one shape
        let shape_count = viz.get_shapes().count();
        assert_eq!(shape_count, 1);
        
        // Update past expiration
        viz.update(0.1);
        
        // Should be gone
        let shape_count = viz.get_shapes().count();
        assert_eq!(shape_count, 0);
    }
}
