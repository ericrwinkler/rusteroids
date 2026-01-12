//! Trail system for updating trail emitters and generating trail geometry

use crate::ecs::{World, Entity};
use crate::ecs::components::{TransformComponent, TrailEmitterComponent};
use crate::foundation::math::{Vec3, Vec2};
use crate::render::systems::billboard::BillboardQuad;

/// System for updating trail emitters
pub struct TrailSystem {
    /// Cached billboard quads for rendering (regenerated each frame)
    billboard_quads: Vec<BillboardQuad>,
}

impl TrailSystem {
    /// Create a new trail system
    pub fn new() -> Self {
        Self {
            billboard_quads: Vec::with_capacity(1024),
        }
    }
    
    /// Update all trail emitters in the world
    pub fn update(&mut self, world: &mut World, delta_time: f32) {
        // Get all entities with trail emitters
        let entities: Vec<Entity> = world
            .query::<TrailEmitterComponent>()
            .iter()
            .map(|(entity, _)| *entity)
            .collect();
        
        for entity in entities {
            self.update_entity_trail(world, entity, delta_time);
        }
    }
    
    /// Update trail for a single entity
    fn update_entity_trail(&mut self, world: &mut World, entity: Entity, delta_time: f32) {
        // Get components
        let transform = match world.get_component::<TransformComponent>(entity) {
            Some(t) => t.clone(),
            None => return,
        };
        
        let trail_emitter = match world.get_component_mut::<TrailEmitterComponent>(entity) {
            Some(t) => t,
            None => return,
        };
        
        // Update segment ages and remove expired segments
        trail_emitter.update_segments(delta_time);
        
        // Check if we should spawn a new segment
        let current_position = transform.position;
        
        if trail_emitter.should_spawn_segment(current_position) {
            // Calculate velocity from position delta (simple approximation)
            // In a real system, you'd use MovementComponent or track previous position
            let velocity = if let Some(last_segment) = trail_emitter.segments().back() {
                let delta = current_position - last_segment.position;
                if delta.norm() > 0.001 {
                    delta.normalize()
                } else {
                    Vec3::new(0.0, 0.0, -1.0) // Default forward
                }
            } else {
                Vec3::new(0.0, 0.0, -1.0) // Default forward for first segment
            };
            
            trail_emitter.spawn_segment(current_position, velocity);
        }
    }
    
    /// Generate billboard quads from all trail emitters
    /// Returns a vector of billboard quads ready for rendering
    pub fn generate_billboard_quads(&mut self, world: &World) -> &Vec<BillboardQuad> {
        self.billboard_quads.clear();
        
        // Query all entities with trail emitters
        let trail_entities = world.query::<TrailEmitterComponent>();
        
        for (entity, trail_emitter) in trail_entities.iter() {
            if !trail_emitter.enabled {
                continue;
            }
            
            // Check if entity has transform (needed for rendering)
            if world.get_component::<TransformComponent>(*entity).is_none() {
                continue;
            }
            
            // Generate quads for this trail
            self.generate_trail_quads(trail_emitter);
        }
        
        &self.billboard_quads
    }
    
    /// Generate billboard quads for a single trail emitter
    fn generate_trail_quads(&mut self, trail_emitter: &TrailEmitterComponent) {
        let segments = trail_emitter.segments();
        
        if segments.len() < 2 {
            return; // Need at least 2 segments to form a trail
        }
        
        // Generate ribbon quads connecting consecutive segments
        for i in 0..segments.len() - 1 {
            let seg_current = &segments[i];
            let seg_next = &segments[i + 1];
            
            // Calculate alpha fade for this segment
            let alpha = seg_current.calculate_alpha(trail_emitter.segment_lifetime);
            
            // Create color with fade
            let mut color = trail_emitter.color;
            color.w *= alpha;
            
            // Position billboard to stretch from current segment to next segment
            // This creates a continuous trail with no gaps
            let midpoint = (seg_current.position + seg_next.position) * 0.5;
            
            // Calculate actual velocity direction from segment positions
            // This is the direction from older segment to newer segment (forward along trail)
            let velocity_dir = (seg_next.position - seg_current.position).normalize();
            
            // Calculate average width
            let width = (seg_current.width + seg_next.width) * 0.5;
            
            // Calculate exact distance between segments for billboard length
            // This ensures no gaps - billboard stretches exactly from segment to segment
            let segment_length = (seg_next.position - seg_current.position).norm();
            
            // Create billboard quad for this segment
            // size.x = length along velocity, size.y = width perpendicular to velocity
            let billboard = BillboardQuad::new(midpoint)
                .with_size(Vec2::new(segment_length, width))
                .with_color(color)
                .with_velocity_alignment(velocity_dir);
            
            self.billboard_quads.push(billboard);
        }
    }
    
    /// Get the number of billboard quads generated
    pub fn quad_count(&self) -> usize {
        self.billboard_quads.len()
    }
}

impl Default for TrailSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::components::TrailEmitterFactory;
    
    #[test]
    fn test_trail_segment_spawning() {
        let mut world = World::new();
        let mut trail_system = TrailSystem::new();
        
        // Create entity with trail emitter
        let entity = world.create_entity();
        let mut trail_emitter = TrailEmitterComponent::new()
            .with_spawn_distance(1.0);
        trail_emitter.initialize(Vec3::zero());
        
        let transform = TransformComponent::new(Vec3::zero());
        
        world.add_component(entity, trail_emitter);
        world.add_component(entity, transform);
        
        // Update - no movement, no new segments
        trail_system.update(&mut world, 0.016);
        let trail = world.get_component::<TrailEmitterComponent>(entity).unwrap();
        assert_eq!(trail.segment_count(), 0);
        
        // Move entity far enough to spawn segment
        let mut transform = world.get_component_mut::<TransformComponent>(entity).unwrap();
        transform.position = Vec3::new(0.0, 0.0, 2.0);
        drop(transform);
        
        trail_system.update(&mut world, 0.016);
        let trail = world.get_component::<TrailEmitterComponent>(entity).unwrap();
        assert!(trail.segment_count() > 0);
    }
}
