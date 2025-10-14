//! Dynamic Object Spawner Interface
//!
//! High-level API for spawning and managing dynamic objects with automatic lifecycle management.
//! This provides a clean abstraction over the lower-level pool systems, making it easy for
//! applications to create temporary objects without worrying about memory management.
//!
//! # Architecture
//!
//! ```text
//! Application → DynamicObjectSpawner → DynamicObjectManager → Resource Pools
//!      |              |                        |                    |
//!   spawn()      validate params          pool allocation      GPU resources
//!      |              |                        |                    |
//!   Handle       error handling          handle generation     buffer management
//! ```
//!
//! # Usage
//!
//! ```rust
//! // Spawn a dynamic object (particle, projectile, etc.)
//! let handle = spawner.spawn_dynamic_object(DynamicSpawnParams {
//!     position: Vec3::new(0.0, 5.0, 0.0),
//!     rotation: Vec3::zeros(),
//!     scale: Vec3::new(1.0, 1.0, 1.0),
//!     material: MaterialProperties::default(),
//!     lifetime: 5.0, // 5 seconds
//! })?;
//! 
//! // Object automatically despawns after lifetime expires
//! // Or despawn manually
//! spawner.despawn_dynamic_object(handle)?;
//! ```
//!
//! # Error Handling
//!
//! The spawner provides comprehensive error handling for common failure scenarios:
//! - Pool exhaustion (no available slots)
//! - Invalid parameters (negative lifetime, invalid scale)
//! - Handle validation errors
//! - Resource allocation failures

use crate::foundation::math::Vec3;
use crate::render::dynamic::{
    DynamicObjectManager, DynamicObjectHandle, DynamicSpawnParams, DynamicObjectError
};
use crate::render::dynamic::resource_pool::MaterialProperties;
use crate::render::vulkan::{VulkanContext, VulkanResult};
use std::time::Instant;

/// Errors that can occur during dynamic object spawning
#[derive(Debug)]
pub enum SpawnerError {
    /// Pool is full, cannot spawn more objects
    PoolExhausted {
        /// Current active object count
        active_count: usize,
        /// Maximum pool capacity
        max_capacity: usize,
    },
    /// Invalid spawn parameters
    InvalidParameters {
        /// Description of the parameter issue
        reason: String,
    },
    /// Handle is invalid or stale
    InvalidHandle {
        /// The problematic handle
        handle: DynamicObjectHandle,
        /// Reason for invalidity
        reason: String,
    },
    /// Underlying object manager error
    ObjectManagerError(DynamicObjectError),
    /// Resource allocation failed
    ResourceAllocationFailed(String),
}

impl std::fmt::Display for SpawnerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PoolExhausted { active_count, max_capacity } => {
                write!(f, "Pool exhausted: {}/{} objects active", active_count, max_capacity)
            }
            Self::InvalidParameters { reason } => {
                write!(f, "Invalid parameters: {}", reason)
            }
            Self::InvalidHandle { handle, reason } => {
                write!(f, "Invalid handle {:?}: {}", handle, reason)
            }
            Self::ObjectManagerError(e) => {
                write!(f, "Object manager error: {}", e)
            }
            Self::ResourceAllocationFailed(msg) => {
                write!(f, "Resource allocation failed: {}", msg)
            }
        }
    }
}

impl std::error::Error for SpawnerError {}

impl From<DynamicObjectError> for SpawnerError {
    fn from(error: DynamicObjectError) -> Self {
        Self::ObjectManagerError(error)
    }
}

/// Parameters for spawning a dynamic object with validation
#[derive(Debug, Clone)]
pub struct ValidatedSpawnParams {
    /// World position
    pub position: Vec3,
    /// Rotation in radians (Euler angles)
    pub rotation: Vec3,
    /// Scale factors (must be positive)
    pub scale: Vec3,
    /// Material properties for rendering
    pub material: MaterialProperties,
    /// Optional entity ID for ECS integration
    pub entity_id: Option<u64>,
    /// Render flags for special rendering modes
    pub render_flags: u32,
}

impl ValidatedSpawnParams {
    /// Create new spawn parameters with validation
    pub fn new(
        position: Vec3, 
        rotation: Vec3, 
        scale: Vec3, 
        material: MaterialProperties
    ) -> Result<Self, SpawnerError> {
        Self::validate_parameters(&position, &rotation, &scale)?;
        
        Ok(Self {
            position,
            rotation,
            scale,
            material,
            entity_id: None,
            render_flags: 0,
        })
    }
    
    /// Create spawn parameters with entity ID
    pub fn with_entity_id(mut self, entity_id: u64) -> Self {
        self.entity_id = Some(entity_id);
        self
    }
    
    /// Create spawn parameters with render flags
    pub fn with_render_flags(mut self, flags: u32) -> Self {
        self.render_flags = flags;
        self
    }
    
    /// Validate spawn parameters
    fn validate_parameters(
        position: &Vec3,
        rotation: &Vec3,
        scale: &Vec3
    ) -> Result<(), SpawnerError> {
        // Check for NaN or infinite values
        if !position.iter().all(|x| x.is_finite()) {
            return Err(SpawnerError::InvalidParameters {
                reason: format!("Position contains invalid values: {:?}", position),
            });
        }
        
        if !rotation.iter().all(|x| x.is_finite()) {
            return Err(SpawnerError::InvalidParameters {
                reason: format!("Rotation contains invalid values: {:?}", rotation),
            });
        }
        
        if !scale.iter().all(|x| x.is_finite() && *x > 0.0) {
            return Err(SpawnerError::InvalidParameters {
                reason: format!("Scale must be positive and finite: {:?}", scale),
            });
        }
        
        // Check reasonable bounds
        const MAX_POSITION: f32 = 1000000.0;
        if position.iter().any(|x| x.abs() > MAX_POSITION) {
            return Err(SpawnerError::InvalidParameters {
                reason: format!("Position values too large: {:?}", position),
            });
        }
        
        const MAX_SCALE: f32 = 1000.0;
        if scale.iter().any(|x| *x > MAX_SCALE) {
            return Err(SpawnerError::InvalidParameters {
                reason: format!("Scale values too large: {:?}", scale),
            });
        }
        
        Ok(())
    }
}

/// Trait for spawning dynamic objects with automatic lifecycle management
pub trait DynamicObjectSpawner {
    /// Spawn a new dynamic object
    /// 
    /// # Arguments
    /// 
    /// * `params` - Validated spawn parameters
    /// 
    /// # Returns
    /// 
    /// * `Ok(DynamicObjectHandle)` - Handle to the spawned object
    /// * `Err(SpawnerError)` - Error if spawning failed
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// let params = ValidatedSpawnParams::new(
    ///     Vec3::new(0.0, 5.0, 0.0),
    ///     Vec3::zeros(),
    ///     Vec3::new(1.0, 1.0, 1.0),
    ///     MaterialProperties::default(),
    ///     5.0
    /// )?;
    /// 
    /// let handle = spawner.spawn_dynamic_object(params)?;
    /// ```
    fn spawn_dynamic_object(&mut self, params: ValidatedSpawnParams) -> Result<DynamicObjectHandle, SpawnerError>;
    
    /// Despawn a dynamic object immediately
    /// 
    /// # Arguments
    /// 
    /// * `handle` - Handle to the object to despawn
    /// 
    /// # Returns
    /// 
    /// * `Ok(())` - Object successfully despawned
    /// * `Err(SpawnerError)` - Error if despawning failed
    fn despawn_dynamic_object(&mut self, handle: DynamicObjectHandle) -> Result<(), SpawnerError>;
    
    /// Get information about a spawned object
    /// 
    /// # Arguments
    /// 
    /// * `handle` - Handle to the object
    /// 
    /// # Returns
    /// 
    /// * `Some(ObjectInfo)` - Information about the object
    /// * `None` - Object not found or handle invalid
    fn get_object_info(&self, handle: DynamicObjectHandle) -> Option<ObjectInfo>;
    
    /// Get current spawner statistics
    fn get_spawner_stats(&self) -> SpawnerStats;
    
    /// Update object lifecycle (call once per frame)
    /// 
    /// This automatically despawns objects that have exceeded their lifetime
    /// and handles any pending cleanup operations.
    /// 
    /// # Arguments
    /// 
    /// * `delta_time` - Time elapsed since last update in seconds
    fn update_lifecycle(&mut self, delta_time: f32);
}

/// Information about a spawned dynamic object
#[derive(Debug, Clone)]
pub struct ObjectInfo {
    /// Object handle
    pub handle: DynamicObjectHandle,
    /// Current world position
    pub position: Vec3,
    /// Current rotation
    pub rotation: Vec3,
    /// Current scale
    pub scale: Vec3,
    /// When the object was spawned
    pub spawn_time: Instant,
    /// Associated entity ID (if any)
    pub entity_id: Option<u64>,
    /// Current render flags
    pub render_flags: u32,
}

/// Statistics about the dynamic object spawner
#[derive(Debug, Clone, Default)]
pub struct SpawnerStats {
    /// Current number of active objects
    pub active_objects: usize,
    /// Maximum pool capacity
    pub max_capacity: usize,
    /// Total objects spawned since creation
    pub total_spawned: u64,
    /// Total objects despawned since creation
    pub total_despawned: u64,
    /// Objects automatically despawned due to lifetime expiry
    pub auto_despawned: u64,
    /// Objects manually despawned
    pub manual_despawned: u64,
    /// Pool utilization percentage (0.0 to 1.0)
    pub utilization: f32,
    /// Peak number of objects active simultaneously
    pub peak_active: usize,
}

impl SpawnerStats {
    /// Update utilization percentage
    pub fn update_utilization(&mut self) {
        if self.max_capacity > 0 {
            self.utilization = self.active_objects as f32 / self.max_capacity as f32;
        } else {
            self.utilization = 0.0;
        }
    }
    
    /// Check if pool is nearly full (>90% utilization)
    pub fn is_nearly_full(&self) -> bool {
        self.utilization > 0.9
    }
    
    /// Check if pool is critically full (>95% utilization)
    pub fn is_critically_full(&self) -> bool {
        self.utilization > 0.95
    }
}

/// Default implementation of DynamicObjectSpawner using DynamicObjectManager
pub struct DefaultDynamicSpawner {
    /// Underlying object manager
    object_manager: DynamicObjectManager,
    /// Spawner statistics
    stats: SpawnerStats,
}

impl DefaultDynamicSpawner {
    /// Create a new default dynamic spawner
    pub fn new(context: &VulkanContext, max_objects: usize) -> VulkanResult<Self> {
        let mut object_manager = DynamicObjectManager::new(max_objects);
        object_manager.initialize_resources(context)?;
        
        let mut stats = SpawnerStats::default();
        stats.max_capacity = max_objects;
        stats.update_utilization();
        
        Ok(Self {
            object_manager,
            stats,
        })
    }
    
    /// Get reference to underlying object manager
    pub fn object_manager(&self) -> &DynamicObjectManager {
        &self.object_manager
    }
    
    /// Get mutable reference to underlying object manager
    pub fn object_manager_mut(&mut self) -> &mut DynamicObjectManager {
        &mut self.object_manager
    }
    
    /// Get active objects for rendering
    ///
    /// Returns a HashMap containing all currently active dynamic objects.
    /// This is used by the rendering system to access object data for batch rendering operations.
    /// 
    /// NOTE: This creates a HashMap and clones render data every call - should be optimized
    ///
    /// # Returns
    /// HashMap of active dynamic objects ready for rendering
    pub fn get_active_objects(&self) -> std::collections::HashMap<DynamicObjectHandle, crate::render::dynamic::DynamicRenderData> {
        self.object_manager.get_active_objects_map()
    }
    
    /// Get objects grouped by mesh type for multi-mesh rendering
    ///
    /// Returns objects organized by MeshType for efficient batch rendering.
    /// Iterate over active objects without cloning (more efficient)
    ///
    /// Provides direct access to active objects for rendering without the overhead
    /// of creating a HashMap and cloning all render data.
    ///
    /// # Arguments
    /// * `callback` - Function to call for each active object
    pub fn for_each_active_object<F>(&self, callback: F) 
    where 
        F: FnMut(DynamicObjectHandle, &crate::render::dynamic::DynamicRenderData),
    {
        self.object_manager.for_each_active_object(callback);
    }
}

impl DynamicObjectSpawner for DefaultDynamicSpawner {
    fn spawn_dynamic_object(&mut self, params: ValidatedSpawnParams) -> Result<DynamicObjectHandle, SpawnerError> {
        // Check pool availability
        let active_count = self.object_manager.active_count();
        if active_count >= self.stats.max_capacity {
            return Err(SpawnerError::PoolExhausted {
                active_count,
                max_capacity: self.stats.max_capacity,
            });
        }
        
        // Convert to DynamicSpawnParams (no mesh_type - single mesh pool)
        let spawn_params = DynamicSpawnParams {
            position: params.position,
            rotation: params.rotation,
            scale: params.scale,
            material: crate::render::material::Material::standard_pbr(
                crate::render::material::StandardMaterialParams {
                    base_color: Vec3::new(
                        params.material.base_color[0],
                        params.material.base_color[1],
                        params.material.base_color[2]
                    ),
                    alpha: params.material.base_color[3],
                    metallic: params.material.metallic,
                    roughness: params.material.roughness,
                    emission: Vec3::new(
                        params.material.emission[0],
                        params.material.emission[1],
                        params.material.emission[2]
                    ),
                    ..Default::default()
                }
            ),
        };
        
        // Spawn object through manager
        let handle = self.object_manager.spawn_object(spawn_params)?;
        
        // Update statistics
        self.stats.total_spawned += 1;
        self.stats.active_objects = self.object_manager.active_count();
        if self.stats.active_objects > self.stats.peak_active {
            self.stats.peak_active = self.stats.active_objects;
        }
        self.stats.update_utilization();
        
        Ok(handle)
    }
    
    fn despawn_dynamic_object(&mut self, handle: DynamicObjectHandle) -> Result<(), SpawnerError> {
        // Validate handle first (using object_pool method)
        if self.object_manager.get_object(handle).is_err() {
            return Err(SpawnerError::InvalidHandle {
                handle,
                reason: "Handle is invalid or stale".to_string(),
            });
        }
        
        // Despawn through manager
        self.object_manager.despawn_object(handle)?;
        
        // Update statistics
        self.stats.total_despawned += 1;
        self.stats.manual_despawned += 1;
        self.stats.active_objects = self.object_manager.active_count();
        self.stats.update_utilization();
        
        Ok(())
    }
    
    fn get_object_info(&self, handle: DynamicObjectHandle) -> Option<ObjectInfo> {
        let render_data = self.object_manager.get_object(handle).ok()?;
        
        Some(ObjectInfo {
            handle,
            position: render_data.position,
            rotation: render_data.rotation,
            scale: render_data.scale,
            spawn_time: render_data.spawn_time,
            entity_id: None, // TODO: Add entity_id to DynamicRenderData
            render_flags: 0, // TODO: Add render_flags to DynamicRenderData
        })
    }
    
    fn get_spawner_stats(&self) -> SpawnerStats {
        let mut stats = self.stats.clone();
        stats.active_objects = self.object_manager.active_count();
        stats.update_utilization();
        stats
    }
    
    fn update_lifecycle(&mut self, delta_time: f32) {
        let initial_count = self.object_manager.active_count();
        
        // Update object manager (handles automatic despawning)
        self.object_manager.update(delta_time);
        
        let final_count = self.object_manager.active_count();
        let auto_despawned = initial_count.saturating_sub(final_count);
        
        // Update statistics
        self.stats.auto_despawned += auto_despawned as u64;
        self.stats.total_despawned += auto_despawned as u64;
        self.stats.active_objects = final_count;
        self.stats.update_utilization();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::math::Vec3;
    
    #[test]
    fn test_validated_spawn_params_creation() {
        let params = ValidatedSpawnParams::new(
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(0.1, 0.2, 0.3),
            Vec3::new(1.0, 1.0, 1.0),
            MaterialProperties::default(),
        );
        
        assert!(params.is_ok());
        let params = params.unwrap();
        assert_eq!(params.position, Vec3::new(1.0, 2.0, 3.0));
    }
    
    #[test]
    fn test_invalid_scale_validation() {
        let result = ValidatedSpawnParams::new(
            Vec3::zeros(),
            Vec3::zeros(),
            Vec3::new(0.0, 1.0, 1.0), // Zero scale is invalid
            MaterialProperties::default(),
        );
        
        assert!(result.is_err());
        match result.unwrap_err() {
            SpawnerError::InvalidParameters { reason } => {
                assert!(reason.contains("Scale must be positive"));
            }
            _ => panic!("Expected InvalidParameters error"),
        }
    }
    
    #[test]
    fn test_spawner_stats_utilization() {
        let mut stats = SpawnerStats {
            active_objects: 50,
            max_capacity: 100,
            ..Default::default()
        };
        
        stats.update_utilization();
        assert_eq!(stats.utilization, 0.5);
        
        assert!(!stats.is_nearly_full());
        assert!(!stats.is_critically_full());
        
        stats.active_objects = 95;
        stats.update_utilization();
        assert!(stats.is_nearly_full());
        assert!(!stats.is_critically_full()); // 95% is nearly full but not critically full
        
        stats.active_objects = 96;
        stats.update_utilization();
        assert!(stats.is_critically_full()); // 96% is critically full (>95%)
    }
    
    #[test]
    fn test_spawner_error_display() {
        let error = SpawnerError::PoolExhausted {
            active_count: 100,
            max_capacity: 100,
        };
        
        let display = format!("{}", error);
        assert!(display.contains("Pool exhausted: 100/100"));
    }
    
    #[test]
    fn test_parameter_bounds_validation() {
        // Test extremely large position
        let result = ValidatedSpawnParams::new(
            Vec3::new(2000000.0, 0.0, 0.0),
            Vec3::zeros(),
            Vec3::new(1.0, 1.0, 1.0),
            MaterialProperties::default(),
        );
        
        assert!(result.is_err());
        
        // Test extremely large scale
        let result = ValidatedSpawnParams::new(
            Vec3::zeros(),
            Vec3::zeros(),
            Vec3::new(2000.0, 1.0, 1.0),
            MaterialProperties::default(),
        );
        
        assert!(result.is_err());
    }
}