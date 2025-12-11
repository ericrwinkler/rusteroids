//! # Renderable Collector
//!
//! This system collects all renderable entities and submits them to the batch renderer
//! for efficient GPU submission. It replaces the single-object rendering approach
//! with batch processing for better performance.

use crate::ecs::{Entity, World};
use crate::ecs::components::{RenderableComponent, MovementComponent};
use crate::render::{
    RenderQueue, 
    RenderCommand, 
    CommandType,
    BatchRenderer,
    GraphicsEngine
};
use nalgebra::Matrix4;

/// System responsible for collecting renderables and submitting them as batches
pub struct RenderableCollector {
    /// The batch renderer for efficient GPU submission
    batch_renderer: BatchRenderer,
    
    /// Render queue for collecting commands
    render_queue: RenderQueue,
    
    /// Flag to enable/disable the system
    enabled: bool,
}

impl RenderableCollector {
    /// Create a new renderable collector
    pub fn new() -> Self {
        Self {
            batch_renderer: BatchRenderer::new(),
            render_queue: RenderQueue::new(),
            enabled: true,
        }
    }
    
    /// Enable or disable the collector
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
    
    /// Check if the collector is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
    
    /// Update the collector - collect entities and render them
    pub fn update(&mut self, world: &World, graphics_engine: &mut GraphicsEngine) {
        if !self.enabled {
            return;
        }
        
        // Clear the previous frame's commands
        self.render_queue.clear();
        
        // Collect all renderable entities
        self.collect_renderables(world);
        
        // Sort commands by depth for proper transparency handling
        self.render_queue.sort_commands();
        
        // Submit batches to the graphics engine
        if let Err(e) = self.batch_renderer.execute_with_renderer(&self.render_queue, graphics_engine) {
            eprintln!("Batch rendering error: {}", e);
        }
    }
    
    /// Collect all entities that have renderable components
    fn collect_renderables(&mut self, world: &World) {
        for entity in world.entities() {
            if let Some(renderable) = world.get_component::<RenderableComponent>(*entity) {
                if !renderable.visible {
                    continue; // Skip invisible entities
                }
                
                // Calculate transform matrix for this entity
                let transform = self.calculate_entity_transform(world, *entity);
                
                // Determine command type based on transparency
                let command_type = if renderable.is_transparent {
                    CommandType::Transparent
                } else {
                    CommandType::Opaque
                };
                
                // Create render command using the factory methods
                let command = match command_type {
                    CommandType::Opaque => RenderCommand::opaque(
                        *entity,
                        renderable.material.id,
                        transform,
                        transform.m34, // Use Z component for depth sorting
                    ),
                    CommandType::Transparent => RenderCommand::transparent(
                        *entity,
                        renderable.material.id,
                        transform,
                        transform.m34,
                    ),
                };
                
                // Add to render queue
                self.render_queue.add_command(command);
            }
        }
    }
    
    /// Calculate the world transform matrix for an entity
    fn calculate_entity_transform(&self, world: &World, entity: Entity) -> Matrix4<f32> {
        // Start with identity transform
        let mut transform = Matrix4::identity();
        
        // Check if the entity has movement component for position
        if let Some(movement) = world.get_component::<MovementComponent>(entity) {
            // Apply translation based on current position
            // For now, we assume position is stored in the velocity component as current position
            // This is a simplified approach - in a full implementation, you'd have a Transform component
            let position = movement.velocity; // Placeholder - should be actual position
            transform = Matrix4::new_translation(&position) * transform;
        }
        
        // TODO: Add support for rotation and scale when Transform component is added
        // TODO: Add support for parent-child relationships for hierarchical transforms
        
        transform
    }
    
    /// Get rendering statistics from the batch renderer
    pub fn get_stats(&self) -> &crate::render::systems::batching::BatchStats {
        self.batch_renderer.stats()
    }
    
    /// Get the number of commands currently queued
    pub fn queued_commands(&self) -> usize {
        self.render_queue.command_count()
    }
    
    /// Get the number of opaque commands queued
    pub fn opaque_commands(&self) -> usize {
        self.render_queue.opaque_commands().len()
    }
    
    /// Get the number of transparent commands queued
    pub fn transparent_commands(&self) -> usize {
        self.render_queue.transparent_commands().len()
    }
}

impl Default for RenderableCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::World;
    use crate::ecs::components::RenderableComponent;
    use crate::render::resources::materials::MaterialId;
    use crate::render::primitives::Mesh;
    
    #[test]
    fn test_renderable_collector_creation() {
        let system = RenderableCollector::new();
        assert!(system.is_enabled());
        assert_eq!(system.queued_commands(), 0);
    }
    
    #[test]
    fn test_enable_disable() {
        let mut system = RenderableCollector::new();
        assert!(system.is_enabled());
        
        system.set_enabled(false);
        assert!(!system.is_enabled());
        
        system.set_enabled(true);
        assert!(system.is_enabled());
    }
    
    #[test]
    fn test_collect_renderables() {
        let mut world = World::new();
        let mut system = RenderableCollector::new();
        
        // Add a renderable entity
        let entity = world.create_entity();
        let renderable = RenderableComponent::new(MaterialId(0), Mesh::cube());
        world.add_component(entity, renderable);
        
        // Collect renderables
        system.collect_renderables(&world);
        
        // Should have one command queued
        assert_eq!(system.queued_commands(), 1);
    }
    
    #[test]
    fn test_invisible_entities_skipped() {
        let mut world = World::new();
        let mut system = RenderableCollector::new();
        
        // Add an invisible renderable entity
        let entity = world.create_entity();
        let mut renderable = RenderableComponent::new(MaterialId(0), Mesh::cube());
        renderable.visible = false;
        world.add_component(entity, renderable);
        
        // Collect renderables
        system.collect_renderables(&world);
        
        // Should have no commands queued
        assert_eq!(system.queued_commands(), 0);
    }
}
