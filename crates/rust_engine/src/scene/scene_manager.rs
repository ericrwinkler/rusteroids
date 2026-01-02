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
use crate::render::resources::materials::Material;
use crate::render::systems::dynamic::MeshType;
use crate::render::GraphicsEngine;
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
                        let _needs_gpu_update = cached_obj.dirty ||
                            cached_obj.transform != transform_matrix ||
                            cached_obj.material.id != r.material.id;
                        
                        cached_obj.mesh = r.mesh.clone();
                        cached_obj.mesh_type = r.mesh_type;
                        cached_obj.transform = transform_matrix;
                        cached_obj.material = r.material.clone();
                        cached_obj.visible = r.visible;
                        cached_obj.is_transparent = r.is_transparent;
                        cached_obj.render_layer = r.render_layer;
                        cached_obj.mark_dirty();
                        
                        // TODO: Update GPU resources if changed (needs GraphicsEngine reference)
                        // This will be handled during rendering for now
                        
                        // Update in scene graph
                        let bounds = self.compute_bounds(&t.position);
                        graph.update(*entity, bounds);
                    } else {
                        // Create new renderable
                        let obj = RenderableObject::new(
                            *entity,
                            r.mesh.clone(),
                            r.mesh_type,
                            r.material.clone(),
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
    /// 3. Automatically detects transparency from Material.required_pipeline()
    /// 4. Marks entity for sync (will be picked up on next sync_from_world)
    ///
    /// Returns the created entity ID.
    /// Create a renderable entity with automatic GPU resource allocation
    ///
    /// **THIS IS THE CLEAN API**: Applications only call this method, never touch
    /// GraphicsEngine directly. SceneManager coordinates with GraphicsEngine internally.
    ///
    /// # Arguments
    /// * `world` - ECS world for entity creation
    /// * `graphics_engine` - Graphics engine for GPU resource allocation  
    /// * `mesh` - Mesh geometry data
    /// * `mesh_type` - Type of mesh (determines pool)
    /// * `material` - Material for rendering
    /// * `transform` - Initial world transform
    ///
    /// # Returns
    /// * Entity ID
    /// Create a renderable entity with transform, mesh, and material
    ///
    /// Creates an ECS entity with Transform and Renderable components.
    /// GPU resource allocation is deferred until render_entities_from_queue() is called.
    ///
    /// # Arguments
    /// * `world` - ECS world to create entity in
    /// * `mesh` - Mesh data for rendering
    /// * `mesh_type` - Type of mesh for pool system
    /// * `material` - Material properties
    /// * `transform` - Initial transform (position, rotation, scale)
    pub fn create_renderable_entity(
        &self,
        world: &mut World,
        mesh: Mesh,
        mesh_type: MeshType,
        material: Material,
        transform: Transform,
    ) -> Entity {
        use crate::render::resources::materials::PipelineType;
        
        // Create entity
        let entity = world.create_entity();
        
        // Add Transform component
        let transform_component = TransformComponent {
            position: transform.position,
            rotation: transform.rotation,
            scale: transform.scale,
        };
        world.add_component(entity, transform_component);
        
        // NOTE: GPU resource allocation is deferred until render_entities_from_queue() is called.
        // This prevents duplicate allocation and ensures proper entity→handle tracking.
        
        // Detect transparency from Material type
        let is_transparent = matches!(
            material.required_pipeline(),
            PipelineType::TransparentPBR | PipelineType::TransparentUnlit
        );
        
        // Add Renderable component with correct transparency
        let renderable = if is_transparent {
            RenderableComponent::new_transparent(material.clone(), mesh.clone(), mesh_type, 0)
        } else {
            RenderableComponent::new(material.clone(), mesh.clone(), mesh_type)
        };
        world.add_component(entity, renderable);
        
        // Mark dirty for next sync
        self.mark_entity_dirty(entity);
        
        entity
    }
    
    /// Create a renderable entity with specific render layer
    ///
    /// Same as create_renderable_entity but allows specifying render_layer.
    /// Use render_layer = 255 for skyboxes to render them LAST.
    ///
    /// # Arguments
    /// * `world` - ECS world to create entity in
    /// * `mesh` - Mesh data for rendering
    /// * `mesh_type` - Type of mesh for pool system
    /// * `material` - Material properties
    /// * `transform` - Initial transform (position, rotation, scale)
    /// * `render_layer` - Render layer (0-255, higher = renders later)
    pub fn create_renderable_entity_with_layer(
        &self,
        world: &mut World,
        mesh: Mesh,
        mesh_type: MeshType,
        material: Material,
        transform: Transform,
        render_layer: u8,
    ) -> Entity {
        use crate::render::resources::materials::PipelineType;
        
        // Create entity
        let entity = world.create_entity();
        
        // Add Transform component
        let transform_component = TransformComponent {
            position: transform.position,
            rotation: transform.rotation,
            scale: transform.scale,
        };
        world.add_component(entity, transform_component);
        
        // Detect transparency from Material
        let pipeline_type = material.required_pipeline();
        let _is_transparent = matches!(
            pipeline_type,
            PipelineType::TransparentPBR | PipelineType::TransparentUnlit
        );
        
        // Add Renderable component with specified render_layer
        let renderable = RenderableComponent::new_transparent(
            material,
            mesh,
            mesh_type,
            render_layer
        );
        world.add_component(entity, renderable);
        
        // Mark entity as dirty for next sync
        {
            let mut cache = self.renderable_cache.write().unwrap();
            if let Some(obj) = cache.get_mut(&entity) {
                obj.dirty = true;
            }
        }
        
        entity
    }
    
    /// Destroy an entity and remove it from the scene
    ///
    /// This implements two-phase destruction (Game Engine Architecture Ch. 16.6):
    /// 1. Mark components for removal (phase 1)
    /// 2. Release GPU resources via GraphicsEngine (phase 2, can fail gracefully)
    /// 3. Remove from scene graph and cache
    /// 
    /// If GPU resource cleanup fails, we log error but continue to prevent ECS corruption.
    /// Better to leak GPU memory than leave entity in invalid state.
    pub fn destroy_entity(
        &self,
        world: &mut World,
        graphics_engine: &mut GraphicsEngine,
        entity: Entity,
    ) {
        // Phase 1: Remove components to mark entity as inactive
        // This prevents it from being rendered even if cleanup fails
        world.remove_component::<TransformComponent>(entity);
        world.remove_component::<RenderableComponent>(entity);
        world.remove_component::<crate::ecs::components::LightComponent>(entity);
        
        // Phase 2: Release GPU resources via GraphicsEngine (can fail gracefully)
        // GraphicsEngine tracks entity→(mesh_type, handle) mapping internally
        if let Err(e) = graphics_engine.release_entity_gpu_resources(entity) {
            log::error!("Failed to release GPU resources for entity {:?}: {}", entity, e);
            log::warn!("Continuing with entity destruction despite GPU cleanup failure");
            // Continue anyway - better to leak GPU memory than corrupt ECS
        }
        
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
    use crate::render::resources::materials::Material;
    use crate::foundation::math::Vec3;
    
    fn create_test_material() -> Material {
        Material::unlit(crate::render::resources::materials::UnlitMaterialParams {
            color: Vec3::new(1.0, 1.0, 1.0),
            alpha: 1.0,
        })
    }
    
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
            create_test_material(),
            crate::render::primitives::Mesh::cube(),
            MeshType::Cube,
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
                create_test_material(),
                crate::render::primitives::Mesh::cube(),
                MeshType::Cube,
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
            create_test_material(),
            crate::render::primitives::Mesh::cube(),
            MeshType::Cube,
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
            create_test_material(),
            crate::render::primitives::Mesh::cube(),
            MeshType::Cube,
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
