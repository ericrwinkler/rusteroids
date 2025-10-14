//! Dynamic Instance Data Structures
//!
//! This module defines the core data structures for the dynamic rendering system,
//! providing GPU-optimized layouts and high-level object management structures.
//!
//! # Architecture
//!
//! The dynamic rendering system uses three key data structures:
//!
//! 1. **DynamicInstanceData**: GPU-optimized data layout for shader consumption
//! 2. **DynamicRenderObject**: High-level object state management
//! 3. **MaterialInstance**: Efficient material variation system
//!
//! # GPU Memory Layout
//!
//! All GPU data structures use 256-byte alignment for optimal performance
//! across different GPU architectures. This ensures efficient memory access
//! patterns and consistent performance characteristics.
//!
//! # Usage
//!
//! ```rust
//! // Create instance data for GPU upload
//! let instance_data = DynamicInstanceData::new(
//!     transform.get_model_matrix(),
//!     material_instance,
//!     object_flags
//! );
//!
//! // Manage high-level object state
//! let render_object = DynamicRenderObject::new(
//!     entity_id,
//!     transform,
//!     material_instance,
//!     pool_handle
//! );
//! ```

use crate::foundation::math::{Mat4, Mat4Ext, Mat3, Vec3};
use crate::render::dynamic::{DynamicObjectHandle, ResourceState};
use crate::render::dynamic::resource_pool::{BufferHandle, DescriptorSetHandle, MaterialInstanceHandle, MaterialProperties};
use crate::ecs::Entity;
use std::time::Instant;

/// GPU-optimized instance data structure
///
/// This structure is uploaded to the GPU for each dynamic object instance.
/// It uses 256-byte alignment for optimal GPU memory access patterns and
/// must match the shader layout exactly.
///
/// # Memory Layout
///
/// - 64 bytes: Model transformation matrix
/// - 64 bytes: Normal transformation matrix (padded from Mat3)
/// - 64 bytes: Material instance data
/// - 64 bytes: Object instance data and padding
/// Total: 256 bytes (aligned)
#[repr(C, align(256))]
#[derive(Debug, Clone, Copy)]
pub struct DynamicInstanceData {
    /// Model transformation matrix (world space)
    pub model_matrix: [[f32; 4]; 4],
    
    /// Normal transformation matrix (inverse transpose of model matrix upper 3x3)
    /// Padded to 4x4 for alignment requirements
    pub normal_matrix: [[f32; 4]; 4],
    
    /// Material instance data for this object
    pub material_data: MaterialInstanceData,
    
    /// Object-specific instance data
    pub object_data: ObjectInstanceData,
}

/// Material instance data for GPU consumption
///
/// Contains material properties that can vary per instance while
/// sharing the same base material and textures.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MaterialInstanceData {
    /// Base color (albedo) with alpha
    pub base_color: [f32; 4],
    
    /// Material properties: [metallic, roughness, emission_strength, alpha]
    pub material_factors: [f32; 4],
    
    /// Emission color (RGB) + material_index
    pub emission_and_index: [f32; 4],
    
    /// Material flags and padding: [flags, padding, padding, padding]
    pub flags_and_padding: [u32; 4],
}

/// Object-specific instance data
///
/// Contains per-object data that affects rendering behavior
/// but isn't part of the material system.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ObjectInstanceData {
    /// Object position in world space (for distance calculations)
    pub world_position: [f32; 4],
    
    /// Object scale factors (for LOD decisions)
    pub scale_factors: [f32; 4],
    
    /// Rendering flags: [visible, cast_shadows, receive_shadows, use_lod]
    pub render_flags: [u32; 4],
    
    /// Timing data: [spawn_time, lifetime, age, reserved]
    pub timing_data: [f32; 4],
}

/// Material instance identifier
///
/// References to base materials and instance-specific overrides.
/// This allows efficient material variation without full material duplication.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MaterialId {
    /// Index into material manager's material array
    pub index: u32,
    /// Generation counter for validation
    pub generation: u32,
}

/// Material instance with property overrides
///
/// Represents a variation of a base material with runtime property modifications.
/// This enables visual variety for dynamic objects without allocating full materials.
#[derive(Debug, Clone)]
pub struct MaterialInstance {
    /// Base material reference
    pub base_material: MaterialId,
    
    /// Instance-specific property overrides
    pub instance_properties: MaterialProperties,
    
    /// Handle to descriptor set in pool
    pub descriptor_set_handle: DescriptorSetHandle,
    
    /// Material instance handle for pool management
    pub instance_handle: MaterialInstanceHandle,
    
    /// Flags indicating which properties are overridden
    pub property_overrides: MaterialOverrideFlags,
}

/// Flags indicating which material properties are overridden
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaterialOverrideFlags {
    /// Override base color
    pub base_color: bool,
    /// Override metallic factor
    pub metallic: bool,
    /// Override roughness factor
    pub roughness: bool,
    /// Override emission color
    pub emission: bool,
    /// Override alpha/transparency
    pub alpha: bool,
}

/// High-level dynamic render object
///
/// Manages the complete state of a dynamic object including entity linking,
/// transform data, material instances, and pool resource handles.
#[derive(Debug)]
pub struct DynamicRenderObject {
    /// Optional entity ID for ECS integration
    pub entity_id: Option<Entity>,
    
    /// Transform data for this object
    pub transform: ObjectTransform,
    
    /// Material instance for rendering
    pub material_instance: MaterialInstance,
    
    /// Handle to this object in the dynamic object pool
    pub pool_handle: DynamicObjectHandle,
    
    /// Resource handles for GPU resources
    pub resource_handles: ResourceHandleSet,
    
    /// Object lifecycle state
    pub lifecycle: ObjectLifecycle,
    
    /// Rendering state and flags
    pub render_state: ObjectRenderState,
}

/// Transform data for dynamic objects
#[derive(Debug, Clone)]
pub struct ObjectTransform {
    /// Position in world space
    pub position: Vec3,
    /// Rotation (Euler angles in radians)
    pub rotation: Vec3,
    /// Scale factors
    pub scale: Vec3,
    /// Cached model matrix (updated when transform changes)
    pub model_matrix: Mat4,
    /// Cached normal matrix (updated when transform changes)
    pub normal_matrix: Mat3,
    /// Flag indicating if matrices need recalculation
    pub matrices_dirty: bool,
}

/// Set of resource handles for dynamic object GPU resources
#[derive(Debug, Clone)]
pub struct ResourceHandleSet {
    /// Handle to uniform buffer in pool
    pub buffer_handle: Option<BufferHandle>,
    /// Handle to descriptor set in pool
    pub descriptor_set_handle: Option<DescriptorSetHandle>,
    /// Handle to material instance in pool
    pub material_handle: Option<MaterialInstanceHandle>,
}

/// Object lifecycle management
#[derive(Debug, Clone)]
pub struct ObjectLifecycle {
    /// Time when object was spawned
    pub spawn_time: Instant,
    /// How long this object should live (seconds)
    pub lifetime: f32,
    /// Current lifecycle state
    pub state: ResourceState,
    /// Generation counter for handle validation
    pub generation: u32,
}

/// Rendering state and configuration
#[derive(Debug, Clone)]
pub struct ObjectRenderState {
    /// Is this object currently visible
    pub visible: bool,
    /// Should this object cast shadows
    pub cast_shadows: bool,
    /// Should this object receive shadows
    pub receive_shadows: bool,
    /// Use level-of-detail for this object
    pub use_lod: bool,
    /// Current LOD level (0 = highest detail)
    pub lod_level: u32,
    /// Distance from camera (for LOD calculations)
    pub camera_distance: f32,
}

impl DynamicInstanceData {
    /// Create instance data from render object
    pub fn from_render_object(render_object: &DynamicRenderObject) -> Self {
        let transform = &render_object.transform;
        let material = &render_object.material_instance;
        let render_state = &render_object.render_state;
        let lifecycle = &render_object.lifecycle;
        
        // Ensure matrices are up to date
        let model_matrix = if transform.matrices_dirty {
            transform.calculate_model_matrix()
        } else {
            transform.model_matrix
        };
        
        let normal_matrix = if transform.matrices_dirty {
            transform.calculate_normal_matrix()
        } else {
            transform.normal_matrix
        };
        
        Self {
            model_matrix: model_matrix.into(),
            normal_matrix: Self::mat3_to_padded_mat4(normal_matrix),
            material_data: MaterialInstanceData::from_material_instance(material),
            object_data: ObjectInstanceData::from_render_state(render_state, lifecycle, &transform.position, &transform.scale),
        }
    }
    
    /// Create instance data from individual components
    pub fn new(
        model_matrix: Mat4,
        material_instance: &MaterialInstance,
        object_flags: ObjectRenderFlags,
    ) -> Self {
        let normal_matrix = Self::calculate_normal_matrix_from_model(&model_matrix);
        
        Self {
            model_matrix: model_matrix.into(),
            normal_matrix: Self::mat3_to_padded_mat4(normal_matrix),
            material_data: MaterialInstanceData::from_material_instance(material_instance),
            object_data: ObjectInstanceData::from_flags(object_flags),
        }
    }
    
    /// Calculate normal matrix from model matrix
    fn calculate_normal_matrix_from_model(model_matrix: &Mat4) -> Mat3 {
        // Extract upper 3x3 rotation/scale part
        let mat3 = model_matrix.fixed_view::<3, 3>(0, 0);
        
        // Calculate inverse transpose for normal transformation
        mat3.try_inverse()
            .unwrap_or_else(|| mat3.clone_owned())
            .transpose()
    }
    
    /// Convert Mat3 to padded 4x4 for GPU alignment
    fn mat3_to_padded_mat4(mat3: Mat3) -> [[f32; 4]; 4] {
        [
            [mat3[(0, 0)], mat3[(0, 1)], mat3[(0, 2)], 0.0],
            [mat3[(1, 0)], mat3[(1, 1)], mat3[(1, 2)], 0.0],
            [mat3[(2, 0)], mat3[(2, 1)], mat3[(2, 2)], 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ]
    }
    
    /// Get the size of this structure in bytes
    pub const fn size_bytes() -> usize {
        std::mem::size_of::<Self>()
    }
    
    /// Validate that this structure meets GPU alignment requirements
    pub const fn validate_alignment() -> bool {
        // Must be exactly 256 bytes and properly aligned
        Self::size_bytes() == 256 && std::mem::align_of::<Self>() >= 256
    }
}

impl MaterialInstanceData {
    /// Create from material instance
    pub fn from_material_instance(material_instance: &MaterialInstance) -> Self {
        let props = &material_instance.instance_properties;
        
        Self {
            base_color: props.base_color,
            material_factors: [props.metallic, props.roughness, 0.0, props.alpha],
            emission_and_index: [props.emission[0], props.emission[1], props.emission[2], 0.0],
            flags_and_padding: [0, 0, 0, 0], // TODO: Add material flags
        }
    }
    
    /// Create from material properties
    pub fn from_material_properties(props: &MaterialProperties, material_index: u32) -> Self {
        Self {
            base_color: props.base_color,
            material_factors: [props.metallic, props.roughness, 0.0, props.alpha],
            emission_and_index: [props.emission[0], props.emission[1], props.emission[2], material_index as f32],
            flags_and_padding: [0, 0, 0, 0],
        }
    }
}

/// Simplified object render flags for basic usage
#[derive(Debug, Clone)]
pub struct ObjectRenderFlags {
    /// Whether this object should be rendered
    pub visible: bool,
    /// Whether this object casts shadows
    pub cast_shadows: bool,
    /// Whether this object receives shadows
    pub receive_shadows: bool,
    /// Whether to use level-of-detail for this object
    pub use_lod: bool,
}

impl ObjectInstanceData {
    /// Create from render state and lifecycle
    pub fn from_render_state(
        render_state: &ObjectRenderState,
        lifecycle: &ObjectLifecycle,
        position: &Vec3,
        scale: &Vec3,
    ) -> Self {
        let age = lifecycle.spawn_time.elapsed().as_secs_f32();
        
        Self {
            world_position: [position.x, position.y, position.z, 1.0],
            scale_factors: [scale.x, scale.y, scale.z, 1.0],
            render_flags: [
                render_state.visible as u32,
                render_state.cast_shadows as u32,
                render_state.receive_shadows as u32,
                render_state.use_lod as u32,
            ],
            timing_data: [
                lifecycle.spawn_time.elapsed().as_secs_f32(),
                lifecycle.lifetime,
                age,
                0.0, // Reserved
            ],
        }
    }
    
    /// Create from simplified flags
    pub fn from_flags(flags: ObjectRenderFlags) -> Self {
        Self {
            world_position: [0.0, 0.0, 0.0, 1.0],
            scale_factors: [1.0, 1.0, 1.0, 1.0],
            render_flags: [
                flags.visible as u32,
                flags.cast_shadows as u32,
                flags.receive_shadows as u32,
                flags.use_lod as u32,
            ],
            timing_data: [0.0, 0.0, 0.0, 0.0],
        }
    }
}

impl ObjectTransform {
    /// Create new transform
    pub fn new(position: Vec3, rotation: Vec3, scale: Vec3) -> Self {
        let mut transform = Self {
            position,
            rotation,
            scale,
            model_matrix: Mat4::identity(),
            normal_matrix: Mat3::identity(),
            matrices_dirty: true,
        };
        
        transform.update_matrices();
        transform
    }
    
    /// Update the transform and mark matrices as dirty
    pub fn set_position(&mut self, position: Vec3) {
        self.position = position;
        self.matrices_dirty = true;
    }
    
    /// Update the rotation and mark matrices as dirty
    pub fn set_rotation(&mut self, rotation: Vec3) {
        self.rotation = rotation;
        self.matrices_dirty = true;
    }
    
    /// Update the scale and mark matrices as dirty
    pub fn set_scale(&mut self, scale: Vec3) {
        self.scale = scale;
        self.matrices_dirty = true;
    }
    
    /// Update cached matrices if dirty
    pub fn update_matrices(&mut self) {
        if self.matrices_dirty {
            self.model_matrix = self.calculate_model_matrix();
            self.normal_matrix = self.calculate_normal_matrix();
            self.matrices_dirty = false;
        }
    }
    
    /// Calculate model matrix from transform components
    pub fn calculate_model_matrix(&self) -> Mat4 {
        let translation = Mat4::new_translation(&self.position);
        let rotation_x = Mat4::rotation_x(self.rotation.x);
        let rotation_y = Mat4::rotation_y(self.rotation.y);
        let rotation_z = Mat4::rotation_z(self.rotation.z);
        let scale_matrix = Mat4::new_nonuniform_scaling(&self.scale);
        
        // Apply transforms: scale -> rotation -> translation
        translation * rotation_z * rotation_y * rotation_x * scale_matrix
    }
    
    /// Calculate normal matrix from current transform
    pub fn calculate_normal_matrix(&self) -> Mat3 {
        let model_matrix = if self.matrices_dirty {
            self.calculate_model_matrix()
        } else {
            self.model_matrix
        };
        
        // Extract upper 3x3 and calculate inverse transpose
        let mat3 = model_matrix.fixed_view::<3, 3>(0, 0);
        mat3.try_inverse()
            .unwrap_or_else(|| mat3.clone_owned())
            .transpose()
    }
}

impl MaterialInstance {
    /// Create new material instance
    pub fn new(
        base_material: MaterialId,
        instance_properties: MaterialProperties,
        descriptor_set_handle: DescriptorSetHandle,
        instance_handle: MaterialInstanceHandle,
    ) -> Self {
        Self {
            base_material,
            instance_properties,
            descriptor_set_handle,
            instance_handle,
            property_overrides: MaterialOverrideFlags::none(),
        }
    }
    
    /// Create with property overrides
    pub fn with_overrides(
        base_material: MaterialId,
        instance_properties: MaterialProperties,
        descriptor_set_handle: DescriptorSetHandle,
        instance_handle: MaterialInstanceHandle,
        overrides: MaterialOverrideFlags,
    ) -> Self {
        Self {
            base_material,
            instance_properties,
            descriptor_set_handle,
            instance_handle,
            property_overrides: overrides,
        }
    }
}

impl MaterialOverrideFlags {
    /// No properties overridden
    pub fn none() -> Self {
        Self {
            base_color: false,
            metallic: false,
            roughness: false,
            emission: false,
            alpha: false,
        }
    }
    
    /// All properties overridden
    pub fn all() -> Self {
        Self {
            base_color: true,
            metallic: true,
            roughness: true,
            emission: true,
            alpha: true,
        }
    }
    
    /// Check if any properties are overridden
    pub fn has_overrides(&self) -> bool {
        self.base_color || self.metallic || self.roughness || self.emission || self.alpha
    }
}

impl MaterialId {
    /// Create new material ID
    pub fn new(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }
    
    /// Invalid material ID
    pub fn invalid() -> Self {
        Self { index: u32::MAX, generation: 0 }
    }
    
    /// Check if this ID is valid
    pub fn is_valid(&self) -> bool {
        self.index != u32::MAX
    }
}

impl ResourceHandleSet {
    /// Create new empty handle set
    pub fn new() -> Self {
        Self {
            buffer_handle: None,
            descriptor_set_handle: None,
            material_handle: None,
        }
    }
    
    /// Check if all handles are assigned
    pub fn is_complete(&self) -> bool {
        self.buffer_handle.is_some() 
            && self.descriptor_set_handle.is_some() 
            && self.material_handle.is_some()
    }
    
    /// Clear all handles
    pub fn clear(&mut self) {
        self.buffer_handle = None;
        self.descriptor_set_handle = None;
        self.material_handle = None;
    }
}

impl Default for ObjectRenderFlags {
    fn default() -> Self {
        Self {
            visible: true,
            cast_shadows: true,
            receive_shadows: true,
            use_lod: true,
        }
    }
}

impl Default for MaterialOverrideFlags {
    fn default() -> Self {
        Self::none()
    }
}

impl Default for ObjectRenderState {
    fn default() -> Self {
        Self {
            visible: true,
            cast_shadows: true,
            receive_shadows: true,
            use_lod: true,
            lod_level: 0,
            camera_distance: 0.0,
        }
    }
}

// Ensure our instance data meets GPU requirements at compile time
const _: () = assert!(DynamicInstanceData::validate_alignment(), "DynamicInstanceData must be exactly 256 bytes and properly aligned");

// Additional compile-time validations
const _: () = assert!(std::mem::size_of::<MaterialInstanceData>() == 64, "MaterialInstanceData must be exactly 64 bytes");
const _: () = assert!(std::mem::size_of::<ObjectInstanceData>() == 64, "ObjectInstanceData must be exactly 64 bytes");

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_dynamic_instance_data_size() {
        assert_eq!(std::mem::size_of::<DynamicInstanceData>(), 256);
        assert_eq!(std::mem::align_of::<DynamicInstanceData>(), 256);
        assert!(DynamicInstanceData::validate_alignment());
    }
    
    #[test]
    fn test_material_instance_data_size() {
        assert_eq!(std::mem::size_of::<MaterialInstanceData>(), 64);
    }
    
    #[test]
    fn test_object_instance_data_size() {
        assert_eq!(std::mem::size_of::<ObjectInstanceData>(), 64);
    }
    
    #[test]
    fn test_object_transform_matrix_calculation() {
        let mut transform = ObjectTransform::new(
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(2.0, 2.0, 2.0),
        );
        
        assert!(!transform.matrices_dirty);
        
        transform.set_position(Vec3::new(5.0, 6.0, 7.0));
        assert!(transform.matrices_dirty);
        
        transform.update_matrices();
        assert!(!transform.matrices_dirty);
        
        // Check that position is correctly applied
        let pos_vec = transform.model_matrix.transform_point(&nalgebra::Point3::origin());
        assert!((pos_vec.x - 5.0).abs() < 0.001);
        assert!((pos_vec.y - 6.0).abs() < 0.001);
        assert!((pos_vec.z - 7.0).abs() < 0.001);
    }
    
    #[test]
    fn test_material_override_flags() {
        let flags = MaterialOverrideFlags::none();
        assert!(!flags.has_overrides());
        
        let flags = MaterialOverrideFlags::all();
        assert!(flags.has_overrides());
        
        let mut flags = MaterialOverrideFlags::none();
        flags.base_color = true;
        assert!(flags.has_overrides());
    }
    
    #[test]
    fn test_resource_handle_set() {
        let mut handles = ResourceHandleSet::new();
        assert!(!handles.is_complete());
        
        handles.buffer_handle = Some(BufferHandle::new(0, 1, 0));
        handles.descriptor_set_handle = Some(DescriptorSetHandle::new(0, 1, 0));
        handles.material_handle = Some(MaterialInstanceHandle::new(0, 1));
        
        assert!(handles.is_complete());
        
        handles.clear();
        assert!(!handles.is_complete());
    }
    
    #[test]
    fn test_material_id_validation() {
        let valid_id = MaterialId::new(5, 10);
        assert!(valid_id.is_valid());
        
        let invalid_id = MaterialId::invalid();
        assert!(!invalid_id.is_valid());
    }
}