//! Backend abstraction traits for the rendering system
//!
//! This module defines the traits that rendering backends must implement
//! to provide a consistent interface for the high-level renderer.

use crate::render::{Vertex, RenderError};
use crate::foundation::math::{Mat4, Vec3};

/// Result type for backend operations
pub type BackendResult<T> = Result<T, RenderError>;

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
    fn set_multi_light_environment(&mut self, multi_light_env: &crate::render::lighting::MultiLightEnvironment);
    
    /// Update camera UBO data (for UBO-based rendering)
    fn update_camera_ubo(&mut self, view_matrix: Mat4, projection_matrix: Mat4, view_projection_matrix: Mat4, camera_position: Vec3);
    
    /// Bind shared rendering resources for multiple object rendering
    fn bind_shared_resources(&mut self, shared: &crate::render::SharedRenderingResources) -> BackendResult<()>;
    
    /// Record draw call for an object using shared resources (new multiple object pattern)
    fn record_shared_object_draw(&mut self, shared: &crate::render::SharedRenderingResources, object_descriptor_sets: &[ash::vk::DescriptorSet]) -> BackendResult<()>;
    
    /// Draw a frame
    fn draw_frame(&mut self) -> BackendResult<()>;
    
    /// Wait for the device to be idle
    fn wait_idle(&self) -> BackendResult<()>;
    
    /// Recreate the swapchain (for window resizing)
    fn recreate_swapchain(&mut self, window_handle: &mut dyn WindowBackendAccess) -> BackendResult<()>;
    
    /// Downcast to concrete backend type for advanced operations
    /// This breaks abstraction but is needed for complex resource management
    fn as_any(&self) -> &dyn std::any::Any;
    
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
    
    /// Record dynamic object draws using instanced rendering
    ///
    /// Records all active dynamic objects into the command buffer using efficient
    /// instanced rendering (vkCmdDrawInstanced). This significantly reduces command
    /// buffer overhead compared to individual draw calls.
    ///
    /// # Arguments
    /// * `dynamic_objects` - HashMap of active dynamic objects to render
    /// * `shared_resources` - Shared rendering resources (mesh, textures, materials)
    ///
    /// # Returns
    /// Result indicating successful command recording
    fn record_dynamic_draws(&mut self, 
                           dynamic_objects: &std::collections::HashMap<crate::render::dynamic::DynamicObjectHandle, crate::render::dynamic::DynamicRenderData>,
                           shared_resources: &crate::render::SharedRenderingResources) 
                           -> BackendResult<()>;
}

/// Window backend access trait
///
/// This trait allows the renderer to access backend-specific window handles
/// for operations like swapchain recreation.
pub trait WindowBackendAccess {
    /// Get a mutable reference to the Vulkan window if available
    fn get_vulkan_window(&mut self) -> Option<&mut crate::render::vulkan::Window>;
}
