//! # Scene Manager
//!
//! High-level coordinator for dynamic scene management. This module orchestrates
//! the ECS world, rendering system, and integration with the main application loop
//! to provide efficient batch rendering of multiple objects.

use crate::ecs::{World, Entity};
use crate::ecs::components::{RenderableComponent, MovementComponent, LifecycleComponent};
use crate::ecs::systems::RenderableCollector;
use crate::render::{GraphicsEngine, material::MaterialId, mesh::Mesh};
use nalgebra::Vector3;
use std::time::{Duration, Instant};

/// Configuration for scene management
#[derive(Debug, Clone)]
pub struct SceneConfig {
    /// Maximum number of entities in the scene
    pub max_entities: usize,
    
    /// Whether to enable automatic cleanup of expired entities
    pub auto_cleanup: bool,
    
    /// Target frame rate for performance monitoring
    pub target_fps: f32,
    
    /// Enable performance statistics collection
    pub enable_stats: bool,
}

impl Default for SceneConfig {
    fn default() -> Self {
        Self {
            max_entities: 10000,
            auto_cleanup: true,
            target_fps: 60.0,
            enable_stats: true,
        }
    }
}

/// Performance statistics for scene management
#[derive(Debug, Clone, Default)]
pub struct SceneStats {
    /// Current number of entities in the scene
    pub entity_count: usize,
    
    /// Number of renderable entities
    pub renderable_count: usize,
    
    /// Number of moving entities
    pub moving_count: usize,
    
    /// Frame time in milliseconds
    pub frame_time_ms: f32,
    
    /// Current FPS
    pub fps: f32,
    
    /// Time spent on ECS updates (microseconds)
    pub ecs_update_time_us: u64,
    
    /// Time spent on rendering (microseconds)  
    pub render_time_us: u64,
}

impl SceneStats {
    /// Calculate total frame time in microseconds
    pub fn total_frame_time_us(&self) -> u64 {
        self.ecs_update_time_us + self.render_time_us
    }
    
    /// Check if performance is meeting target
    pub fn is_performance_good(&self, target_fps: f32) -> bool {
        self.fps >= target_fps * 0.95 // 95% of target is considered good
    }
}

/// High-level scene manager for coordinating dynamic rendering
pub struct SceneManager {
    /// ECS world containing all entities and components
    world: World,
    
    /// Renderable collector for batch submission
    renderable_collector: RenderableCollector,
    
    /// Scene configuration
    config: SceneConfig,
    
    /// Performance statistics
    stats: SceneStats,
    
    /// Last frame time for FPS calculation
    last_frame_time: Instant,
    
    /// Frame time samples for smoothed FPS calculation
    frame_times: Vec<Duration>,
    
    /// Maximum frame time samples to keep
    max_frame_samples: usize,
}

impl SceneManager {
    /// Create a new scene manager with default configuration
    pub fn new() -> Self {
        Self::with_config(SceneConfig::default())
    }
    
    /// Create a scene manager with custom configuration
    pub fn with_config(config: SceneConfig) -> Self {
        Self {
            world: World::new(),
            renderable_collector: RenderableCollector::new(),
            config,
            stats: SceneStats::default(),
            last_frame_time: Instant::now(),
            frame_times: Vec::new(),
            max_frame_samples: 60, // Keep 1 second of samples at 60 FPS
        }
    }
    
    /// Create a static teapot entity in the scene
    pub fn create_teapot(&mut self, material_id: MaterialId, position: Vector3<f32>) -> Entity {
        let entity = self.world.create_entity();
        
        // Add renderable component
        let renderable = RenderableComponent::new(material_id, Mesh::cube()); // Using cube for now
        self.world.add_component(entity, renderable);
        
        // Add movement component for position (even if not moving)
        let mut movement = MovementComponent::new();
        movement.velocity = position; // Use velocity as position for now
        self.world.add_component(entity, movement);
        
        // Add lifecycle component
        let lifecycle = LifecycleComponent::new(0.0); // Use 0.0 as spawn time
        self.world.add_component(entity, lifecycle);
        
        entity
    }
    
    /// Create a moving teapot entity with velocity
    pub fn create_moving_teapot(
        &mut self, 
        material_id: MaterialId, 
        position: Vector3<f32>,
        velocity: Vector3<f32>
    ) -> Entity {
        let entity = self.create_teapot(material_id, position);
        
        // Update movement component with actual velocity
        if let Some(movement) = self.world.get_component_mut::<MovementComponent>(entity) {
            movement.velocity = velocity;
        }
        
        entity
    }
    
    /// Remove an entity from the scene
    pub fn remove_entity(&mut self, entity: Entity) {
        // Remove all components for this entity
        // Note: The World doesn't have a remove_entity method, so we'll need to remove components manually
        // This is a simplified approach - in a full implementation, you'd extend World with proper entity removal
        self.world.remove_component::<RenderableComponent>(entity);
        self.world.remove_component::<MovementComponent>(entity);
        self.world.remove_component::<LifecycleComponent>(entity);
    }
    
    /// Update the scene for one frame
    pub fn update(&mut self, delta_time: f32, graphics_engine: &mut GraphicsEngine) {
        let _frame_start = Instant::now(); // Prefix with underscore to avoid unused warning
        
        // Update ECS systems
        let ecs_start = Instant::now();
        self.update_movement_system(delta_time);
        self.update_lifecycle_system();
        let ecs_time = ecs_start.elapsed();
        
        // Render the scene
        let render_start = Instant::now();
        self.renderable_collector.update(&self.world, graphics_engine);
        let render_time = render_start.elapsed();
        
        // Update performance statistics
        if self.config.enable_stats {
            self.update_stats(ecs_time, render_time);
        }
        
        // Cleanup expired entities if enabled
        if self.config.auto_cleanup {
            self.cleanup_expired_entities();
        }
    }
    
    /// Update movement for all entities with MovementComponent
    fn update_movement_system(&mut self, delta_time: f32) {
        for entity in self.world.entities().cloned().collect::<Vec<_>>() {
            if let Some(movement) = self.world.get_component_mut::<MovementComponent>(entity) {
                movement.integrate(delta_time); // Use integrate method instead of integrate_physics
            }
        }
    }
    
    /// Update lifecycle for all entities
    fn update_lifecycle_system(&mut self) {
        let mut entities_to_remove = Vec::new();
        let entities: Vec<Entity> = self.world.entities().cloned().collect();
        
        for entity in entities {
            if let Some(lifecycle) = self.world.get_component_mut::<LifecycleComponent>(entity) {
                lifecycle.update(0.0, 0.016); // Use placeholder values for now
                
                // Mark expired entities for removal
                if lifecycle.should_destroy() { // Use should_destroy instead of is_expired
                    entities_to_remove.push(entity);
                }
            }
        }
        
        // Remove expired entities
        for entity in entities_to_remove {
            self.remove_entity(entity); // Use our custom remove_entity method
        }
    }
    
    /// Clean up entities marked for disposal
    fn cleanup_expired_entities(&mut self) {
        // This is handled by update_lifecycle_system for now
        // Could be extended with more sophisticated cleanup logic
    }
    
    /// Update performance statistics
    fn update_stats(&mut self, ecs_time: Duration, render_time: Duration) {
        let now = Instant::now();
        let frame_time = now.duration_since(self.last_frame_time);
        self.last_frame_time = now;
        
        // Update frame time samples
        self.frame_times.push(frame_time);
        if self.frame_times.len() > self.max_frame_samples {
            self.frame_times.remove(0);
        }
        
        // Calculate smoothed FPS
        if !self.frame_times.is_empty() {
            let avg_frame_time = self.frame_times.iter().sum::<Duration>() / self.frame_times.len() as u32;
            self.stats.fps = if avg_frame_time.as_nanos() > 0 {
                1_000_000_000.0 / avg_frame_time.as_nanos() as f32
            } else {
                0.0
            };
            self.stats.frame_time_ms = avg_frame_time.as_millis() as f32;
        }
        
        // Update entity counts
        self.stats.entity_count = self.world.entities().count(); // Use count() instead of entity_count()
        self.stats.renderable_count = self.count_entities_with::<RenderableComponent>();
        self.stats.moving_count = self.count_entities_with::<MovementComponent>();
        
        // Update timing statistics
        self.stats.ecs_update_time_us = ecs_time.as_micros() as u64;
        self.stats.render_time_us = render_time.as_micros() as u64;
    }
    
    /// Count entities that have a specific component type
    fn count_entities_with<T: crate::ecs::Component>(&self) -> usize {
        self.world.entities()
            .filter(|entity| self.world.get_component::<T>(**entity).is_some())
            .count()
    }
    
    /// Get current performance statistics
    pub fn stats(&self) -> &SceneStats {
        &self.stats
    }
    
    /// Get scene configuration
    pub fn config(&self) -> &SceneConfig {
        &self.config
    }
    
    /// Get current entity count
    pub fn entity_count(&self) -> usize {
        self.world.entities().count() // Use count() instead of entity_count()
    }
    
    /// Check if performance is meeting targets
    pub fn is_performance_good(&self) -> bool {
        self.stats.is_performance_good(self.config.target_fps)
    }
    
    /// Get rendering system statistics
    pub fn rendering_stats(&self) -> &crate::render::batch_renderer::BatchStats {
        self.renderable_collector.get_stats()
    }
}

impl Default for SceneManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scene_manager_creation() {
        let scene = SceneManager::new();
        assert_eq!(scene.entity_count(), 0);
        assert!(scene.is_performance_good());
    }

    #[test]
    fn test_create_teapot() {
        let mut scene = SceneManager::new();
        let material_id = MaterialId(1);
        let position = Vector3::new(1.0, 2.0, 3.0);
        
        let entity = scene.create_teapot(material_id, position);
        assert_eq!(scene.entity_count(), 1);
        
        // Verify components were added
        assert!(scene.world.get_component::<RenderableComponent>(entity).is_some());
        assert!(scene.world.get_component::<MovementComponent>(entity).is_some());
        assert!(scene.world.get_component::<LifecycleComponent>(entity).is_some());
    }

    #[test]
    fn test_remove_entity() {
        let mut scene = SceneManager::new();
        let entity = scene.create_teapot(MaterialId(1), Vector3::zeros());
        assert_eq!(scene.entity_count(), 1);
        
        scene.remove_entity(entity);
        assert_eq!(scene.entity_count(), 0);
    }

    #[test]
    fn test_performance_stats() {
        let scene = SceneManager::new();
        let stats = scene.stats();
        
        assert_eq!(stats.entity_count, 0);
        assert_eq!(stats.renderable_count, 0);
        assert_eq!(stats.moving_count, 0);
    }
}
