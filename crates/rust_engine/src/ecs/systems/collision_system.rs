//! ECS collision system wrapper
//!
//! Based on Game Engine Architecture 3rd Edition:
//! - Section 13.3: Collision Detection System
//! - Section 16.6: Updating Game Objects in Real Time
//!
//! This module provides an ECS-aware wrapper around the core collision system,
//! integrating it with World, Components, and providing automatic updates.

use crate::ecs::{World, Entity};
use crate::ecs::components::{ColliderComponent, CollisionStateComponent, TransformComponent};
use crate::physics::collision_system::{PhysicsCollisionSystem, CollisionPair};
use crate::spatial::spatial_query::SpatialQuery;
use crate::debug::collision_debug::CollisionDebugVisualizer;
use std::collections::HashSet;

/// ECS collision system that integrates collision detection with the ECS World
/// 
/// This system:
/// - Automatically syncs ColliderComponents with the PhysicsCollisionSystem
/// - Updates CollisionStateComponents each frame
/// - Optionally provides debug visualization
pub struct EcsCollisionSystem {
    collision_system: PhysicsCollisionSystem,
    debug_visualizer: Option<CollisionDebugVisualizer>,
    selected_entity: Option<Entity>,
}

impl EcsCollisionSystem {
    /// Create a new ECS collision system with the given spatial query
    pub fn new(spatial_query: Box<dyn SpatialQuery>) -> Self {
        Self {
            collision_system: PhysicsCollisionSystem::new(spatial_query),
            debug_visualizer: None,
            selected_entity: None,
        }
    }
    
    /// Enable debug visualization
    pub fn enable_debug(&mut self, enabled: bool) {
        if enabled && self.debug_visualizer.is_none() {
            self.debug_visualizer = Some(CollisionDebugVisualizer::new());
        }
        if let Some(viz) = &mut self.debug_visualizer {
            viz.set_enabled(enabled);
        }
    }
    
    /// Set which entity should have debug visualization (for broad-phase sphere)
    pub fn set_debug_entity(&mut self, entity: Option<Entity>) {
        // Only clear if switching to a different entity
        if self.selected_entity != entity {
            // Clear old visualization
            if let Some(old_entity) = self.selected_entity {
                if let Some(viz) = &mut self.debug_visualizer {
                    viz.clear_broad_phase(old_entity);
                }
            }
            
            self.selected_entity = entity;
        }
    }
    
    /// Register a new collider (called when ColliderComponent is added)
    pub fn register_collider(&mut self, entity: Entity, collider: &ColliderComponent, world: &World) {
        // Get initial position from TransformComponent
        let initial_position = world.get_component::<TransformComponent>(entity)
            .map(|t| t.position)
            .unwrap_or(crate::foundation::math::Vec3::zeros());
        
        self.collision_system.register_collider(
            entity,
            collider.shape.clone(),
            collider.layer,
            collider.mask,
            collider.is_trigger,
            collider.bounding_radius,
            initial_position,
        );
    }
    
    /// Unregister a collider (called when ColliderComponent is removed)
    pub fn unregister_collider(&mut self, entity: Entity) {
        self.collision_system.unregister_collider(entity);
        
        // Clear debug visualization
        if let Some(viz) = &mut self.debug_visualizer {
            viz.clear_broad_phase(entity);
        }
    }
    
    /// Main update function - performs collision detection and updates components
    /// 
    /// GEA 16.6: "Game object updates are typically performed once per frame"
    pub fn update(&mut self, world: &mut World, delta_time: f32) {
        // Step 1: Sync spatial query positions (broad-phase only)
        self.sync_positions_for_broad_phase(world);
        
        // Step 2: Perform collision detection (broad + narrow phase)
        // PhysicsCollisionSystem will query TransformComponent directly for narrow-phase
        self.collision_system.detect_collisions(world);
        
        // Step 3: Update CollisionStateComponents with results
        self.update_collision_states(world);
        
        // Step 4: Update debug visualization if enabled
        if self.debug_visualizer.is_some() {
            // Capture values before borrowing to avoid borrow conflicts
            let selected_entity = self.selected_entity;
            let viz = self.debug_visualizer.as_mut().unwrap();
            viz.update(delta_time);
            Self::update_debug_visualization_impl(world, viz, selected_entity, &self.collision_system);
        }
    }
    
    /// Sync spatial query positions for broad-phase (no shape updates needed)
    /// With model-space shapes, we only update positions in spatial structure
    fn sync_positions_for_broad_phase(&mut self, world: &World) {
        // Query all entities with ColliderComponent
        let colliders = world.query::<ColliderComponent>();
        
        for (entity, collider) in colliders {
            // Get transform for this entity
            if let Some(transform) = world.get_component::<TransformComponent>(entity) {
                // Update spatial query position (bounding_radius is already in world-space)
                self.collision_system.update_collider_position(
                    entity,
                    transform.position,
                    collider.bounding_radius,
                );
            }
        }
    }
    /// Update CollisionStateComponents with current collision data
    fn update_collision_states(&mut self, world: &mut World) {
        // Get collision enter/exit events
        let entered = self.collision_system.get_collision_entered();
        let exited = self.collision_system.get_collision_exited();
        let current = self.collision_system.get_current_collisions();
        
        // Build sets for quick lookup
        let mut colliding_entities: std::collections::HashMap<Entity, HashSet<Entity>> = 
            std::collections::HashMap::new();
        
        for pair in current {
            colliding_entities.entry(pair.entity_a)
                .or_insert_with(HashSet::new)
                .insert(pair.entity_b);
            colliding_entities.entry(pair.entity_b)
                .or_insert_with(HashSet::new)
                .insert(pair.entity_a);
        }
        
        let mut entered_map: std::collections::HashMap<Entity, Vec<Entity>> = 
            std::collections::HashMap::new();
        for pair in &entered {
            entered_map.entry(pair.entity_a)
                .or_insert_with(Vec::new)
                .push(pair.entity_b);
            entered_map.entry(pair.entity_b)
                .or_insert_with(Vec::new)
                .push(pair.entity_a);
        }
        
        let mut exited_map: std::collections::HashMap<Entity, Vec<Entity>> = 
            std::collections::HashMap::new();
        for pair in &exited {
            exited_map.entry(pair.entity_a)
                .or_insert_with(Vec::new)
                .push(pair.entity_b);
            exited_map.entry(pair.entity_b)
                .or_insert_with(Vec::new)
                .push(pair.entity_a);
        }
        
        // Pre-collect collider data to avoid borrow conflicts
        let entity_colliders: std::collections::HashMap<Entity, (u32, u32)> = world
            .query::<ColliderComponent>()
            .into_iter()
            .map(|(entity, collider)| (entity, (collider.layer, collider.mask)))
            .collect();
        
        // Get selected entity's bounding radius for debug
        let selected_bounding_radius = if let Some(selected) = self.selected_entity {
            world.get_component::<ColliderComponent>(selected).map(|c| c.bounding_radius)
        } else {
            None
        };
        
        let selected_transform_position = if let Some(selected) = self.selected_entity {
            world.get_component::<TransformComponent>(selected).map(|t| t.position)
        } else {
            None
        };
        
        // Update all CollisionStateComponents
        let states = world.query_mut::<CollisionStateComponent>();
        
        for (entity, state) in states {
            // Clear per-frame data
            state.clear_frame_data();
            
            // Update colliding_with
            if let Some(colliders) = colliding_entities.get(&entity) {
                state.colliding_with = colliders.clone();
            } else {
                state.colliding_with.clear();
            }
            
            // Update entered
            if let Some(entered_entities) = entered_map.get(&entity) {
                state.collision_entered = entered_entities.clone();
            }
            
            // Update exited
            if let Some(exited_entities) = exited_map.get(&entity) {
                state.collision_exited = exited_entities.clone();
            }
            
            // Update nearby entities (with layer filtering)
            let mut nearby_filtered = Vec::new();
            if let Some((layer, mask)) = entity_colliders.get(&entity) {
                let all_nearby = self.collision_system.query_nearby(entity);
                
                for nearby_entity in all_nearby {
                    if let Some((nearby_layer, nearby_mask)) = entity_colliders.get(&nearby_entity) {
                        // Apply same layer filtering as broad-phase
                        if crate::physics::CollisionLayers::should_collide(
                            *layer,
                            *mask,
                            *nearby_layer,
                            *nearby_mask,
                        ) {
                            nearby_filtered.push(nearby_entity);
                        }
                    }
                }
            }
            state.nearby_entities = nearby_filtered.into_iter().collect();
        }
    }
    
    /// Update debug visualization
    fn update_debug_visualization_impl(
        world: &World, 
        viz: &mut CollisionDebugVisualizer,
        selected_entity: Option<Entity>,
        collision_system: &PhysicsCollisionSystem,
    ) {
        // Visualize selected entity's broad-phase query
        if let Some(selected) = selected_entity {
            if let Some(collider) = world.get_component::<ColliderComponent>(selected) {
                // Use the position stored in the octree, not the transform!
                // The octree query uses its stored position, so visualization must match
                if let Some((octree_pos, _)) = collision_system.spatial_query().get_entity_data(selected) {
                    // Draw broad-phase query sphere (actual bounding radius)
                    viz.draw_broad_phase_query(
                        selected,
                        octree_pos,  // Use octree position, not transform position
                        collider.bounding_radius,
                    );
                }
            }
        }
        
        // Visualize all collision shapes
        let colliders = world.query::<ColliderComponent>();
        
        for (entity, collider) in colliders {
            if collider.debug_draw {
                if let Some(state) = world.get_component::<CollisionStateComponent>(entity) {
                    viz.draw_collision_shape(
                        entity,
                        &collider.shape,
                        state.is_colliding(),
                    );
                }
            }
        }
    }
    
    /// Get reference to underlying collision system
    pub fn collision_system(&self) -> &PhysicsCollisionSystem {
        &self.collision_system
    }
    
    /// Get mutable reference to underlying collision system
    pub fn collision_system_mut(&mut self) -> &mut PhysicsCollisionSystem {
        &mut self.collision_system
    }
    
    /// Get reference to spatial query (shorthand for collision_system().spatial_query())
    pub fn spatial_query(&self) -> &dyn SpatialQuery {
        self.collision_system.spatial_query()
    }
    
    /// Get mutable reference to spatial query (shorthand for collision_system_mut().spatial_query_mut())
    pub fn spatial_query_mut(&mut self) -> &mut dyn SpatialQuery {
        self.collision_system.spatial_query_mut()
    }
    
    /// Get reference to debug visualizer (if enabled)
    pub fn debug_visualizer(&self) -> Option<&CollisionDebugVisualizer> {
        self.debug_visualizer.as_ref()
    }
    
    /// Get mutable reference to debug visualizer (if enabled)
    pub fn debug_visualizer_mut(&mut self) -> Option<&mut CollisionDebugVisualizer> {
        self.debug_visualizer.as_mut()
    }
}
