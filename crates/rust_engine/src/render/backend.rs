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
    
    /// Update camera UBO data (for UBO-based rendering)
    fn update_camera_ubo(&mut self, view_matrix: Mat4, projection_matrix: Mat4, view_projection_matrix: Mat4, camera_position: Vec3);
    
    /// Draw a frame
    fn draw_frame(&mut self) -> BackendResult<()>;
    
    /// Wait for the device to be idle
    fn wait_idle(&self) -> BackendResult<()>;
    
    /// Recreate the swapchain (for window resizing)
    fn recreate_swapchain(&mut self, window_handle: &mut dyn WindowBackendAccess) -> BackendResult<()>;
}

/// Window backend access trait
///
/// This trait allows the renderer to access backend-specific window handles
/// for operations like swapchain recreation.
pub trait WindowBackendAccess {
    /// Get a mutable reference to the Vulkan window if available
    fn get_vulkan_window(&mut self) -> Option<&mut crate::render::vulkan::Window>;
}
