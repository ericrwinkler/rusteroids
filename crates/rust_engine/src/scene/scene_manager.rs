//! Scene Manager - Bridge between ECS and Renderer
//!
//! Following Game Engine Architecture Chapter 11.2.7 - Scene Graphs and
//! Chapter 16.2 - Runtime Object Model Architectures.
//!
//! The Scene Manager:
//! 1. Syncs ECS components to rendering data (Transform + Renderable → RenderableObject)
//! 2. Maintains spatial acceleration structure (SceneGraph)
//! 3. Performs visibility culling
//! 4. Generates optimized render queues

use crate::ecs::{World, Entity};
use crate::ecs::components::{TransformComponent, RenderableComponent};
use crate::scene::{SceneGraph, SimpleListGraph, RenderableObject, RenderQueue, AABB};
use crate::foundation::math::{Vec3, Transform};
use crate::render::primitives::Mesh;
use crate::render::resources::materials::MaterialId;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock, Mutex};

/// Scene Manager configuration
#[derive(Debug, Clone)]
pub struct SceneConfig {
    /// Enable dirty tracking (only sync changed entities)
    pub enable_dirty_tracking: bool,
    
    /// Enable spatial culling
    pub enable_culling: bool,
    
    /// Default AABB extents for entities without explicit bounds
    pub default_extents: Vec3,
}

impl Default for SceneConfig {
    fn default() -> Self {
        Self {
            enable_dirty_tracking: true,
            enable_culling: true,
            default_extents: Vec3::new(1.0, 1.0, 1.0),
        }
    }
}

/// Scene Manager - coordinates ECS, rendering, and spatial queries
///
/// This is the central hub for rendering. It bridges the gap between
/// gameplay (ECS) and graphics (Renderer).
pub struct SceneManager {
    /// Configuration
    config: SceneConfig,
    
    /// Spatial acceleration structure (pluggable via trait)
    scene_graph: Arc<RwLock<Box<dyn SceneGraph>>>,
    
    /// Cache of renderable objects (Entity → RenderableObject)
    renderable_cache: Arc<RwLock<HashMap<Entity, RenderableObject>>>,
    
    /// Set of entities that changed since last sync (dirty tracking)
    dirty_entities: Arc<Mutex<HashSet<Entity>>>,
    
    /// Active camera entity (for culling)
    active_camera: Option<Entity>,
}

impl SceneManager {
    /// Create a new scene manager with default configuration
    pub fn new() -> Self {
        Self::with_config(SceneConfig::default())
    }
    
    /// Create a scene manager with custom configuration
    pub fn with_config(config: SceneConfig) -> Self {
        Self {
            config,
            scene_graph: Arc::new(RwLock::new(Box::new(SimpleListGraph::new()))),
            renderable_cache: Arc::new(RwLock::new(HashMap::new())),
            dirty_entities: Arc::new(Mutex::new(HashSet::new())),
            active_camera: None,
        }
    }
    
    /// Set the active camera entity for culling
    pub fn set_active_camera(&mut self, camera_entity: Entity) {
        self.active_camera = Some(camera_entity);
    }
    
    /// Mark an entity as dirty (needs sync from ECS)
    pub fn mark_entity_dirty(&self, entity: Entity) {
        if self.config.enable_dirty_tracking {
            let mut dirty = self.dirty_entities.lock().unwrap();
            dirty.insert(entity);
        }
    }
    
    /// Sync ECS world to scene (update renderable cache)
    ///
    /// This is the key method that bridges ECS → Renderer.
    /// Call once per frame after ECS systems have updated.
    ///
    /// Uses automatic change detection from ECS World to only sync
    /// entities with modified Transform or Renderable components.
    pub fn sync_from_world(&mut self, world: &mut World) {
        let mut cache = self.renderable_cache.write().unwrap();
        let mut graph = self.scene_graph.write().unwrap();
        
        // Get entities that changed (Transform or Renderable components)
        let changed_entities = if self.config.enable_dirty_tracking {
            world.get_changed_renderable_entities()
        } else {
            // If dirty tracking disabled, sync all entities
            world.entities().cloned().collect()
        };
        
        // Sync each changed entity
        for entity in &changed_entities {
            // Check if entity still exists in ECS
            // We check by seeing if we can get any component - if entity was destroyed,
            // it won't have any components
            let has_transform = world.get_component::<TransformComponent>(*entity).is_some();
            
            if !has_transform {
                // Entity was destroyed - remove from cache and scene graph
                cache.remove(entity);
                graph.remove(*entity);
                continue;
            }
            
            // Get Transform and Renderable components
            let transform = world.get_component::<TransformComponent>(*entity);
            let renderable = world.get_component::<RenderableComponent>(*entity);
            
            match (transform, renderable) {
                (Some(t), Some(r)) if r.should_render() => {
                    // Entity has both components and is visible - update or create
                    let transform_matrix = t.to_math_transform().to_matrix();
                    
                    if let Some(cached_obj) = cache.get_mut(entity) {
                        // Update existing renderable
                        cached_obj.transform = transform_matrix;
                        cached_obj.material_id = r.material_id;
                        cached_obj.visible = r.visible;
                        cached_obj.is_transparent = r.is_transparent;
                        cached_obj.render_layer = r.render_layer;
                        cached_obj.mark_dirty();
                        
                        // Update in scene graph
                        let bounds = self.compute_bounds(&t.position);
                        graph.update(*entity, bounds);
                    } else {
                        // Create new renderable
                        let obj = RenderableObject::new(
                            *entity,
                            r.material_id,
                            transform_matrix,
                            r.visible,
                            r.is_transparent,
                            r.render_layer,
                        );
                        
                        cache.insert(*entity, obj);
                        
                        // Add to scene graph
                        let bounds = self.compute_bounds(&t.position);
                        graph.add(*entity, bounds);
                    }
                }
                _ => {
                    // Entity doesn't have required components or is hidden - remove if cached
                    if cache.remove(entity).is_some() {
                        graph.remove(*entity);
                    }
                }
            }
        }
        
        // Clear change tracking in ECS for synced entities
        if self.config.enable_dirty_tracking {
            for entity in changed_entities {
                world.clear_entity_changes(entity);
            }
        }
    }
    
    /// Build render queue for current frame
    ///
    /// Queries visible objects (with optional frustum culling) and
    /// organizes them into batches by material.
    pub fn build_render_queue(&self) -> RenderQueue {
        let cache = self.renderable_cache.read().unwrap();
        
        // For now, include all cached objects (culling TODO)
        let objects: Vec<RenderableObject> = cache.values().cloned().collect();
        
        RenderQueue::from_objects(&objects)
    }
    
    /// Get number of entities in the scene
    pub fn entity_count(&self) -> usize {
        let cache = self.renderable_cache.read().unwrap();
        cache.len()
    }
    
    /// Check if an entity is in the scene
    pub fn has_entity(&self, entity: Entity) -> bool {
        let cache = self.renderable_cache.read().unwrap();
        cache.contains_key(&entity)
    }
    
    /// Clear all cached data
    pub fn clear(&mut self) {
        let mut cache = self.renderable_cache.write().unwrap();
        let mut graph = self.scene_graph.write().unwrap();
        
        cache.clear();
        graph.clear();
    }
    
    /// Compute AABB bounds for an entity (simplified for now)
    fn compute_bounds(&self, position: &Vec3) -> AABB {
        AABB::from_center_extents(*position, self.config.default_extents)
    }
    
    // ========================================================================
    // Proposal #1 Phase 2: Convenience APIs for Entity Creation/Destruction
    // ========================================================================
    
    /// Create a renderable entity with mesh and material
    ///
    /// This is a high-level convenience method that:
    /// 1. Creates an ECS entity
    /// 2. Adds Transform and Renderable components
    /// 3. Marks entity for sync (will be picked up on next sync_from_world)
    ///
    /// Returns the created entity ID.
    pub fn create_renderable_entity(
        &self,
        world: &mut World,
        mesh: Mesh,
        material_id: MaterialId,
        transform: Transform,
    ) -> Entity {
        // Create entity
        let entity = world.create_entity();
        
        // Add Transform component
        let transform_component = TransformComponent {
            position: transform.position,
            rotation: transform.rotation,
            scale: transform.scale,
        };
        world.add_component(entity, transform_component);
        
        // Add Renderable component
        let renderable = RenderableComponent::new(material_id, mesh);
        world.add_component(entity, renderable);
        
        // Mark dirty for next sync
        self.mark_entity_dirty(entity);
        
        entity
    }
    
    /// Destroy an entity and remove it from the scene
    ///
    /// This is a high-level convenience method that:
    /// 1. Removes entity from scene graph and render cache
    /// 2. Removes all components from ECS
    /// 3. Marks entity as destroyed
    ///
    /// Note: This does NOT free GPU resources (mesh handles, textures, etc).
    /// Resource cleanup should be handled by the Resource Manager (Proposal #4).
    pub fn destroy_entity(&self, world: &mut World, entity: Entity) {
        // Remove from render cache and scene graph
        {
            let mut cache = self.renderable_cache.write().unwrap();
            cache.remove(&entity);
        }
        {
            let mut graph = self.scene_graph.write().unwrap();
            graph.remove(entity);
        }
        
        // Remove from dirty tracking
        {
            let mut dirty = self.dirty_entities.lock().unwrap();
            dirty.remove(&entity);
        }
        
        // Remove components from ECS (World doesn't have destroy_entity yet)
        // For now, just remove the key components to mark it as inactive
        world.remove_component::<TransformComponent>(entity);
        world.remove_component::<RenderableComponent>(entity);
        
        // Note: When World gets destroy_entity(), we'll call that here
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
    use crate::render::resources::materials::MaterialId;
    
    #[test]
    fn test_scene_manager_creation() {
        let scene_manager = SceneManager::new();
        assert_eq!(scene_manager.entity_count(), 0);
    }
    
    #[test]
    fn test_sync_from_world() {
        let mut world = World::new();
        let mut scene_manager = SceneManager::new();
        
        // Create entity with Transform and Renderable
        let entity = world.create_entity();
        let transform = TransformComponent::from_position(Vec3::new(5.0, 0.0, 0.0));
        let renderable = RenderableComponent::new(
            MaterialId(0),
            crate::render::primitives::Mesh::cube(),
        );
        
        world.add_component(entity, transform);
        world.add_component(entity, renderable);
        
        // Mark entity dirty and sync
        scene_manager.mark_entity_dirty(entity);
        scene_manager.sync_from_world(&mut world);
        
        // Verify entity is in scene
        assert_eq!(scene_manager.entity_count(), 1);
        assert!(scene_manager.has_entity(entity));
    }
    
    #[test]
    fn test_build_render_queue() {
        let mut world = World::new();
        let mut scene_manager = SceneManager::new();
        
        // Create two entities with different materials
        for i in 0..2 {
            let entity = world.create_entity();
            let transform = TransformComponent::from_position(Vec3::new(i as f32, 0.0, 0.0));
            let renderable = RenderableComponent::new(
                MaterialId(i),
                crate::render::primitives::Mesh::cube(),
            );
            
            world.add_component(entity, transform);
            world.add_component(entity, renderable);
            scene_manager.mark_entity_dirty(entity);
        }
        
        scene_manager.sync_from_world(&mut world);
        
        // Build render queue
        let queue = scene_manager.build_render_queue();
        
        assert_eq!(queue.total_object_count(), 2);
        assert_eq!(queue.batch_count(), 2); // 2 different materials
    }
    
    #[test]
    fn test_automatic_change_detection_integration() {
        let mut world = World::new();
        let mut scene_manager = SceneManager::new();
        
        // Create entity with Transform and Renderable
        let entity = world.create_entity();
        let transform = TransformComponent::from_position(Vec3::new(1.0, 2.0, 3.0));
        let renderable = RenderableComponent::new(
            MaterialId(0),
            crate::render::primitives::Mesh::cube(),
        );
        
        world.add_component(entity, transform);
        world.add_component(entity, renderable);
        
        // Sync without marking dirty - automatic detection should work
        scene_manager.sync_from_world(&mut world);
        
        // Verify entity is in scene
        assert_eq!(scene_manager.entity_count(), 1);
        assert!(scene_manager.has_entity(entity));
        
        // Get the cached renderable and verify it's marked dirty
        let cache = scene_manager.renderable_cache.read().unwrap();
        let cached_obj = cache.get(&entity).expect("Entity should be cached");
        assert!(cached_obj.dirty, "Newly synced object should be marked dirty");
        drop(cache);
        
        // Clear changes and mutate the transform
        world.clear_all_changes();
        
        if let Some(t) = world.get_component_mut::<TransformComponent>(entity) {
            t.position = Vec3::new(10.0, 20.0, 30.0);
        }
        
        // Sync again - should detect the mutation
        scene_manager.sync_from_world(&mut world);
        
        // Verify the transform was updated and marked dirty
        let cache = scene_manager.renderable_cache.read().unwrap();
        let cached_obj = cache.get(&entity).expect("Entity should still be cached");
        assert!(cached_obj.dirty, "Modified object should be marked dirty");
        
        // Verify transform matrix reflects new position (rough check via matrix multiplication)
        let pos_vec = cached_obj.transform * crate::foundation::math::Vec4::new(0.0, 0.0, 0.0, 1.0);
        assert!((pos_vec.x - 10.0).abs() < 0.001, "Transform matrix should have new X position");
    }
    
    #[test]
    fn test_change_detection_with_visibility_toggle() {
        let mut world = World::new();
        let mut scene_manager = SceneManager::new();
        
        // Create visible entity
        let entity = world.create_entity();
        let transform = TransformComponent::from_position(Vec3::new(1.0, 0.0, 0.0));
        let mut renderable = RenderableComponent::new(
            MaterialId(0),
            crate::render::primitives::Mesh::cube(),
        );
        renderable.visible = true;
        
        world.add_component(entity, transform);
        world.add_component(entity, renderable);
        
        // Initial sync
        scene_manager.sync_from_world(&mut world);
        assert_eq!(scene_manager.entity_count(), 1);
        
        // Clear changes and make invisible
        world.clear_all_changes();
        
        if let Some(r) = world.get_component_mut::<RenderableComponent>(entity) {
            r.visible = false;
        }
        
        // Sync - entity should be removed from scene (not visible)
        scene_manager.sync_from_world(&mut world);
        assert_eq!(scene_manager.entity_count(), 0, "Invisible entities should be removed");
        
        // Make visible again
        world.clear_all_changes();
        if let Some(r) = world.get_component_mut::<RenderableComponent>(entity) {
            r.visible = true;
        }
        
        // Sync - entity should be added back
        scene_manager.sync_from_world(&mut world);
        assert_eq!(scene_manager.entity_count(), 1, "Visible entities should be added back");
    }
}
