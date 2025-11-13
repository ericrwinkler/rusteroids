//! Backend abstraction traits for the rendering system
//!
//! This module defines the traits that rendering backends must implement
//! to provide a consistent interface for the high-level renderer.

use crate::render::{Vertex, RenderError};
use crate::foundation::math::{Mat4, Vec3};

/// Result type for backend operations
pub type BackendResult<T> = Result<T, RenderError>;

/// Handle to a mesh resource stored in the backend
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MeshHandle(pub u64);

/// Handle to a material resource stored in the backend  
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MaterialHandle(pub u64);

/// Handle to per-object GPU resources (descriptor sets, uniform buffers, etc.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjectResourceHandle(pub u64);

/// Main rendering backend trait
///
/// This trait abstracts over different rendering backends (currently Vulkan)
/// and provides a consistent interface for the high-level renderer.
pub trait RenderBackend {
    /// Get the current swapchain extent (width, height)
    fn get_swapchain_extent(&self) -> (u32, u32);
    
    /// Update mesh data for rendering
    fn update_mesh(&mut self, vertices: &[Vertex], indices: &[u32]) -> BackendResult<()>;
    
    /// Set the MVP (Model-View-Projection) matrix
    fn set_mvp_matrix(&mut self, mvp: [[f32; 4]; 4]);
    
    /// Set the model matrix (for normal calculations)
    fn set_model_matrix(&mut self, model: [[f32; 4]; 4]);
    
    /// Set material color
    fn set_material_color(&mut self, color: [f32; 4]);
    
    /// Set directional light parameters
    fn set_directional_light(&mut self, direction: [f32; 3], intensity: f32, color: [f32; 3], ambient_intensity: f32);
    
    /// Set multi-light environment using UBO-based lighting
    fn set_multi_light_environment(&mut self, multi_light_env: &crate::render::systems::lighting::MultiLightEnvironment);
    
    /// Update camera UBO data (for UBO-based rendering)
    fn update_camera_ubo(&mut self, view_matrix: Mat4, projection_matrix: Mat4, view_projection_matrix: Mat4, camera_position: Vec3);
    
    // === NEW CLEAN API - Use these methods ===
    
    /// Create a mesh resource and return an opaque handle
    /// This replaces the need for applications to manage SharedRenderingResources
    fn create_mesh_resource(&mut self, mesh: &crate::render::Mesh, materials: &[crate::render::Material]) -> BackendResult<MeshHandle>;
    
    /// Render objects using cached mesh resources
    /// Applications only need to provide the mesh handle and transform data
    fn render_objects_with_mesh(&mut self, mesh_handle: MeshHandle, transforms: &[Mat4]) -> BackendResult<()>;
    
    /// Create per-object GPU resources (descriptor sets, uniform buffers, etc.)
    /// Returns an opaque handle for the object's GPU resources
    fn create_object_resources(&mut self, material: &crate::render::Material) -> BackendResult<ObjectResourceHandle>;
    
    /// Update per-object uniform data using the object resource handle
    fn update_object_uniforms(&mut self, handle: ObjectResourceHandle, model_matrix: Mat4, material_data: &[f32; 4]) -> BackendResult<()>;
    
    // === DEPRECATED API - These expose implementation details ===
    
    /// Bind shared rendering resources for multiple object rendering
    /// DEPRECATED: Use create_mesh_resource instead
    #[deprecated(note = "Use create_mesh_resource for cleaner abstraction")]
    fn bind_shared_resources(&mut self, shared: &crate::render::SharedRenderingResources) -> BackendResult<()>;
    
    /// Record draw call for an object using shared resources
    /// DEPRECATED: Use render_objects_with_mesh for cleaner abstraction
    #[deprecated(note = "Use render_objects_with_mesh for cleaner abstraction")]
    fn record_shared_object_draw(&mut self, shared: &crate::render::SharedRenderingResources, object_id: u32) -> BackendResult<()>;
    
    /// Draw a frame
    fn draw_frame(&mut self) -> BackendResult<()>;
    
    /// Wait for the device to be idle
    fn wait_idle(&self) -> BackendResult<()>;
    
    /// Recreate the swapchain (for window resizing)
    fn recreate_swapchain(&mut self, window_handle: &mut dyn WindowBackendAccess) -> BackendResult<()>;
    
    /// Downcast to concrete backend type for advanced operations
    /// This breaks abstraction but is needed for complex resource management
    fn as_any(&self) -> &dyn std::any::Any;
    
    /// Downcast to mutable concrete backend type for advanced operations
    /// This breaks abstraction but is needed for complex resource management
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
    
    /// Begin recording draw commands for multiple objects (proper Vulkan pattern)
    fn begin_render_pass(&mut self) -> BackendResult<()>;
    
    /// Record a draw command for a single object with its own uniform buffer
    fn draw_indexed(&mut self, index_count: u32, instance_count: u32, first_index: u32, vertex_offset: i32, first_instance: u32) -> BackendResult<()>;
    
    /// End recording and submit all commands
    fn end_render_pass_and_submit(&mut self) -> BackendResult<()>;
    
    /// Initialize dynamic object rendering system
    ///
    /// Sets up the instanced rendering infrastructure for efficient batch rendering
    /// of dynamic objects. This must be called before using any dynamic rendering features.
    ///
    /// # Arguments
    /// * `max_objects` - Maximum number of dynamic objects that can be rendered simultaneously
    ///
    /// # Returns
    /// Result indicating successful initialization
    fn initialize_dynamic_rendering(&mut self, max_objects: usize) -> BackendResult<()>;
    
    /// Initialize instanced rendering system for efficient batch rendering
    ///
    /// Sets up the instance renderer for efficient batch rendering of multiple objects
    /// with the same mesh using instanced rendering. This provides significant
    /// performance improvements over individual draw calls.
    ///
    /// # Arguments
    /// * `max_instances` - Maximum number of instances that can be rendered in a single batch
    ///
    /// # Returns
    /// Result indicating successful initialization
    fn initialize_instance_renderer(&mut self, max_instances: usize) -> BackendResult<()>;
    
    /// Record dynamic object draws using backend-agnostic interface
    ///
    /// Records all active dynamic objects into the command buffer using efficient
    /// rendering techniques appropriate for the backend. This abstracts away
    /// backend-specific details like Vulkan instanced rendering.
    ///
    /// # Arguments
    /// * `object_count` - Number of dynamic objects to render
    /// * `mesh_id` - Identifier for the mesh type to render
    ///
    /// # Returns
    /// Result indicating successful command recording
    fn record_dynamic_draws(&mut self, object_count: usize, mesh_id: u32) -> BackendResult<()>;
    
    /// Record a mesh draw command into the current command buffer
    ///
    /// This method records a draw without submitting the frame, allowing it to work
    /// within the dynamic frame lifecycle. Used for text rendering and other
    /// non-batched rendering.
    ///
    /// # Arguments
    /// * `mesh` - Mesh to render
    /// * `transform` - World transform matrix
    /// * `material` - Material for rendering
    ///
    /// # Returns
    /// Result indicating successful command recording
    fn record_mesh_draw(&mut self, mesh: &crate::render::Mesh, transform: &Mat4, material: &crate::render::Material) -> BackendResult<()>;
    
    /// Record an indexed draw command with specific index range
    ///
    /// Assumes mesh data is already uploaded. Records draw commands for a subset of the index buffer.
    ///
    /// # Arguments
    /// * `index_start` - Starting index in the index buffer
    /// * `index_count` - Number of indices to draw
    /// * `transform` - World transform matrix
    /// * `material` - Material for rendering
    ///
    /// # Returns
    /// Result indicating successful command recording
    fn record_indexed_draw(&mut self, index_start: u32, index_count: u32, transform: &Mat4, material: &crate::render::Material) -> BackendResult<()>;
    
    /// Record UI overlay rendering commands (Chapter 11.4.4)
    ///
    /// Records UI overlay drawing commands into the current command buffer while the render pass
    /// is still active. This implements the single render pass approach described in Chapter 11.4.4:
    /// "Overlays are generally rendered after the primary scene, with z-testing disabled."
    ///
    /// This method MUST be called while a render pass is active (after `begin_render_pass()` or
    /// during dynamic rendering, and before the render pass is ended).
    ///
    /// # Implementation Strategy
    /// - Switches to UI pipeline (depth test DISABLED, alpha blend ENABLED)
    /// - Records UI draw commands (quads, text, etc.)
    /// - Render pass STAYS OPEN (single render pass approach)
    ///
    /// # Parameters
    /// - `fps_text`: Optional FPS text to render (e.g., "FPS: 60.2")
    ///
    /// # Returns
    /// Result indicating successful command recording
    fn record_ui_overlay_test(&mut self, fps_text: Option<&str>) -> BackendResult<()>;
}

/// Window backend access trait
///
/// This trait allows the renderer to access backend-specific window handles
/// for operations like swapchain recreation while maintaining abstraction.
pub trait WindowBackendAccess {
    /// Get access to backend-specific window data for operations like swapchain recreation
    /// Returns a generic trait object that can be downcast by the specific backend
    fn get_backend_window(&mut self) -> &mut dyn std::any::Any;
}
