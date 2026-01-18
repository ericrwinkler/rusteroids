//! # Dynamic Object Graphics API
//!
//! High-level graphics API methods for dynamic object management.
//! These methods are called by GraphicsEngine and delegate to the dynamic object systems.

use crate::foundation::math::{Vec3, Mat4};
use crate::render::primitives::Mesh;
use crate::render::resources::materials::Material;
use crate::render::systems::dynamic::{
    MeshPoolManager, MeshType, DynamicObjectHandle, SpawnerError, PoolManagerStats,
};
use crate::render::api::RenderBackend;

/// Initialize the dynamic object pool system
///
/// Creates the MeshPoolManager and sets up initial pools for common mesh types.
/// This must be called before spawning any dynamic objects.
///
/// # Arguments
/// * `pool_manager` - Mutable reference to optional pool manager
/// * `backend` - Backend renderer for resource creation
/// * `max_objects_per_pool` - Maximum objects per individual mesh pool
///
/// # Returns
/// Result indicating successful initialization
pub fn initialize_dynamic_system(
    pool_manager: &mut Option<MeshPoolManager>,
    backend: &dyn RenderBackend,
    max_objects_per_pool: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    log::info!("Initializing mesh pool system with {} objects per pool", max_objects_per_pool);
    
    // Access VulkanContext through backend downcasting for pool system initialization
    if let Some(vulkan_backend) = backend.as_any().downcast_ref::<crate::render::backends::vulkan::VulkanRenderer>() {
        let context = vulkan_backend.get_context();
        
        // Create MeshPoolManager
        let mut mgr = MeshPoolManager::new();
        
        // Initialize the pool manager with Vulkan resources
        mgr.initialize_pools(context)?;
        *pool_manager = Some(mgr);
        
        log::info!("Mesh pool system initialized successfully");
        Ok(())
    } else {
        Err("Renderer is not using Vulkan backend".into())
    }
}

/// Create a mesh pool for a specific mesh type (MeshPoolManager system)
///
/// This method creates a pool for a specific mesh type using the MeshPoolManager.
/// Must be called after initialize_dynamic_system and before spawning objects of this type.
///
/// # Arguments
/// * `pool_manager` - Mutable reference to pool manager
/// * `backend` - Backend renderer for resource creation
/// * `mesh_type` - The type of mesh to create a pool for (Teapot, Sphere, Cube)
/// * `mesh` - The mesh data to create SharedRenderingResources from
/// * `materials` - Array of materials for this mesh type
/// * `max_objects` - Maximum number of objects this pool can hold
/// * `create_instanced_resources_fn` - Function to create instanced rendering resources
///
/// # Returns
/// Result indicating successful pool creation or error
pub fn create_mesh_pool<F>(
    pool_manager: &mut Option<MeshPoolManager>,
    backend: &mut dyn RenderBackend,
    mesh_type: MeshType,
    mesh: &Mesh,
    materials: &[Material],
    max_objects: usize,
    create_instanced_resources_fn: F,
) -> Result<(), Box<dyn std::error::Error>>
where
    F: FnOnce(&Mesh, &[Material]) -> Result<crate::render::resources::shared::SharedRenderingResources, Box<dyn std::error::Error>>,
{
    log::info!("Creating mesh pool for {:?} with capacity {}", mesh_type, max_objects);
    
    // Extract textures from first material (all instances in pool share these textures)
    let base_color_texture = materials.get(0)
        .and_then(|mat| mat.textures.base_color);
    let emission_texture = materials.get(0)
        .and_then(|mat| mat.textures.emission);
    
    log::info!("Pool {:?} base_color texture: {:?}, emission texture: {:?}", 
               mesh_type, base_color_texture, emission_texture);
    
    // Create SharedRenderingResources for this mesh using instanced rendering
    let shared_resources = create_instanced_resources_fn(mesh, materials)?;
    
    // Create the pool in the MeshPoolManager
    if let Some(ref mut mgr) = pool_manager {
        // Get VulkanRenderer from backend
        if let Some(vulkan_backend) = backend.as_any_mut().downcast_mut::<crate::render::backends::vulkan::VulkanRenderer>() {
            // Create per-pool descriptor set with texture binding(s)
            let material_descriptor_set = match (base_color_texture, emission_texture) {
                (Some(base_tex), Some(emission_tex)) => {
                    log::info!("Creating descriptor set for pool {:?} with base_color {:?} and emission {:?}", 
                              mesh_type, base_tex, emission_tex);
                    vulkan_backend.create_material_descriptor_set_with_textures(base_tex, emission_tex)?
                }
                (Some(base_tex), None) => {
                    log::info!("Creating descriptor set for pool {:?} with base_color {:?}", mesh_type, base_tex);
                    vulkan_backend.create_material_descriptor_set_with_texture(base_tex)?
                }
                _ => {
                    log::info!("No textures for pool {:?}, using default descriptor set", mesh_type);
                    vulkan_backend.get_default_material_descriptor_set()
                }
            };
            
            // Get context after descriptor set creation to avoid borrow conflicts
            let context = vulkan_backend.get_context();
            
            mgr.create_pool(
                mesh_type,
                shared_resources,
                max_objects,
                context,
                material_descriptor_set,
                base_color_texture,
            )?;
            log::info!("Successfully created pool for {:?}", mesh_type);
            Ok(())
        } else {
            Err("Backend is not a VulkanRenderer - cannot get VulkanContext".into())
        }
    } else {
        Err("Dynamic system not initialized. Call initialize_dynamic_system first.".into())
    }
}

/// Spawn a dynamic object with specified parameters
///
/// Creates a new dynamic object using the pooled resource system. Objects are
/// automatically managed by the pool and will be despawned when their lifetime expires.
///
/// # Arguments
/// * `pool_manager` - Mutable reference to pool manager
/// * `mesh_type` - Type of mesh to spawn
/// * `position` - World position for the object
/// * `rotation` - Rotation in radians (Euler angles)
/// * `scale` - Scale factors for each axis
/// * `transform` - Pre-computed model-to-world transformation matrix
/// * `material` - Material properties for rendering
///
/// # Returns
/// Handle to the spawned object for tracking/despawning
pub fn allocate_from_pool(
    pool_manager: &mut Option<MeshPoolManager>,
    mesh_type: MeshType,
    transform: Mat4,
    material: Material,
) -> Result<DynamicObjectHandle, SpawnerError> {
    // Ensure pool system is initialized
    let mgr = pool_manager.as_mut()
        .ok_or(SpawnerError::InvalidParameters {
            reason: "Pool system not initialized - call initialize_dynamic_system() first".to_string()
        })?;
    
    // Allocate from the appropriate mesh pool with the provided material
    let handle = mgr.allocate_instance(mesh_type, transform, material)
        .map_err(|e| SpawnerError::InvalidParameters {
            reason: format!("Pool manager allocation failed: {}", e)
        })?;
    
    log::debug!("Allocated {} object from pool", mesh_type);
    Ok(handle)
}

/// Free a pool instance and return it to the pool
///
/// Immediately removes the instance from rendering and returns
/// its GPU resources to the pool for reuse.
///
/// # Arguments
/// * `pool_manager` - Mutable reference to pool manager
/// * `mesh_type` - Type of mesh
/// * `handle` - Handle to the pool instance to free
///
/// # Returns
/// Result indicating success or failure
pub fn free_pool_instance(
    pool_manager: &mut Option<MeshPoolManager>,
    mesh_type: MeshType,
    handle: DynamicObjectHandle,
) -> Result<(), SpawnerError> {
    let mgr = pool_manager.as_mut()
        .ok_or(SpawnerError::InvalidParameters {
            reason: "Pool system not initialized".to_string()
        })?;
    
    mgr.free_instance(mesh_type, handle)
        .map_err(|e| SpawnerError::InvalidParameters {
            reason: format!("Pool manager free failed: {}", e)
        })?;
    log::debug!("Freed {} pool instance with handle generation {}", mesh_type, handle.generation);
    Ok(())
}

/// Update the transform matrix of a pool instance
///
/// # Arguments
/// * `pool_manager` - Mutable reference to pool manager
/// * `mesh_type` - The mesh type pool this instance belongs to
/// * `handle` - Handle to the pool instance to update
/// * `transform` - New pre-computed model-to-world transformation matrix
pub fn update_pool_instance(
    pool_manager: &mut Option<MeshPoolManager>,
    mesh_type: MeshType,
    handle: DynamicObjectHandle,
    transform: Mat4,
) -> Result<(), SpawnerError> {
    if let Some(ref mut mgr) = pool_manager {
        mgr.update_instance_transform(mesh_type, handle, transform)
            .map_err(|e| SpawnerError::InvalidParameters {
                reason: format!("Failed to update instance transform: {}", e),
            })
    } else {
        Err(SpawnerError::InvalidParameters {
            reason: "Pool manager not initialized".to_string(),
        })
    }
}

/// Update the material properties of a pool instance
///
/// Allows dynamic updates to material properties like alpha, colors, and emission.
/// Changes are automatically reflected in rendering on the next frame.
///
/// # Arguments
/// * `pool_manager` - Mutable reference to pool manager
/// * `mesh_type` - The mesh type pool this instance belongs to
/// * `handle` - Handle to the pool instance to update
/// * `material` - New material with updated properties
pub fn update_instance_material(
    pool_manager: &mut Option<MeshPoolManager>,
    mesh_type: MeshType,
    handle: DynamicObjectHandle,
    material: Material,
) -> Result<(), SpawnerError> {
    if let Some(ref mut mgr) = pool_manager {
        mgr.update_instance_material(mesh_type, handle, material)
            .map_err(|e| SpawnerError::InvalidParameters {
                reason: format!("Failed to update instance material: {}", e),
            })
    } else {
        Err(SpawnerError::InvalidParameters {
            reason: "Pool manager not initialized".to_string(),
        })
    }
}

/// Begin dynamic frame rendering setup
///
/// Prepares the dynamic rendering system for a new frame.
/// Processes cleanup of manually despawned objects.
///
/// # Arguments
/// * `pool_manager` - Mutable reference to pool manager
pub fn begin_dynamic_frame(
    pool_manager: &mut Option<MeshPoolManager>,
) {
    // Clear transient billboard pools from previous frame
    if let Some(mgr) = pool_manager {
        mgr.clear_all_billboard_pools();
    }
    
    // Update pool manager to process cleanup of manually despawned objects
    if let Some(mgr) = pool_manager {
        mgr.update_all_pools();
    }
}

/// Record dynamic object draw commands
///
/// Records all active dynamic objects into the command buffer using instanced
/// rendering for optimal performance.
///
/// # Arguments
/// * `pool_manager` - Mutable reference to pool manager
/// * `backend` - Backend renderer
/// * `camera_position` - Current camera position for depth sorting
/// * `camera_target` - Current camera target for depth sorting
///
/// # Returns
/// Result indicating successful command recording
pub fn record_dynamic_draws(
    pool_manager: &mut Option<MeshPoolManager>,
    backend: &mut dyn RenderBackend,
    camera_position: Vec3,
    camera_target: Vec3,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(ref mut mgr) = pool_manager {
        let stats = mgr.get_stats();
        
        if stats.total_active_objects > 0 {
            log::trace!("Recording draws for {} active dynamic objects across {} pools", 
                       stats.total_active_objects, stats.render_batches_per_frame);
            
            // Use MeshPoolManager's proper render method to avoid state corruption between mesh types
            mgr.render_all_pools_via_backend(backend, camera_position, camera_target)
                .map_err(|e| format!("Failed to render dynamic object pools: {}", e))?;
            
            log::debug!("Dynamic object rendering completed: {} total objects across {} mesh types", 
                       stats.total_active_objects, stats.render_batches_per_frame);
        } else {
            log::trace!("No active dynamic objects to render");
        }
    }
    
    Ok(())
}

/// Get dynamic system statistics
///
/// Returns statistics about the dynamic object system including pool utilization.
///
/// # Arguments
/// * `pool_manager` - Reference to pool manager
///
/// # Returns
/// Option containing spawner statistics, or None if system not initialized
pub fn get_dynamic_stats(pool_manager: &Option<MeshPoolManager>) -> Option<PoolManagerStats> {
    pool_manager.as_ref().map(|manager| manager.get_stats())
}


