//! # Render Backend Abstraction
//!
//! This module provides a clean abstraction layer between the high-level renderer
//! and specific graphics API implementations. It defines the `RenderBackend` trait
//! that graphics backends (Vulkan, etc.) must implement.
//!
//! ## Design Goals
//!
//! - **API Agnostic**: Support multiple graphics backends through a common interface
//! - **Library-First**: Usable as a standalone rendering library
//! - **Performance**: Minimal overhead abstraction over native API calls
//! - **Type Safety**: Strong typing with backend-specific error handling
//!
//! ## Architecture
//!
//! The backend abstraction follows these principles:
//! - **Stateful**: Backends manage their own internal state
//! - **Resource Management**: Backends handle GPU resource lifecycle
//! - **Error Propagation**: Clean error handling through Result types
//! - **Extensible**: Easy to add new backends without changing high-level code

use crate::render::{Vertex, RenderError};
use crate::foundation::math::Mat4;

/// Result type for backend operations
pub type BackendResult<T> = Result<T, RenderError>;

/// # Render Backend Trait
///
/// Defines the interface that all graphics backends must implement.
/// This abstraction allows the high-level renderer to work with different
/// graphics APIs (Vulkan, OpenGL, etc.) through a common interface.
///
/// ## Implementation Notes
///
/// Backends should:
/// - Manage their own GPU resources and state
/// - Handle coordinate system conversions internally
/// - Provide efficient resource updates
/// - Support frame-in-flight rendering patterns
pub trait RenderBackend {
    /// Get the current swapchain/framebuffer dimensions
    ///
    /// # Returns
    /// Tuple of (width, height) in pixels
    fn get_swapchain_extent(&self) -> (u32, u32);
    
    /// Update mesh data for rendering
    ///
    /// Uploads vertex and index data to GPU buffers for subsequent draw calls.
    ///
    /// # Arguments
    /// * `vertices` - Array of vertex data with position, normal, and texture coordinates
    /// * `indices` - Array of triangle indices for indexed drawing
    ///
    /// # Returns
    /// Result indicating success or backend-specific error
    fn update_mesh(&mut self, vertices: &[Vertex], indices: &[u32]) -> BackendResult<()>;
    
    /// Set the model-view-projection matrix for vertex transformation
    ///
    /// # Arguments
    /// * `mvp` - 4x4 transformation matrix in column-major order
    fn set_mvp_matrix(&mut self, mvp: [[f32; 4]; 4]);
    
    /// Set the model matrix for normal transformation and lighting
    ///
    /// # Arguments
    /// * `model` - 4x4 model transformation matrix in column-major order
    fn set_model_matrix(&mut self, model: [[f32; 4]; 4]);
    
    /// Set material color for the current mesh
    ///
    /// # Arguments
    /// * `color` - RGBA color values in range [0.0, 1.0]
    fn set_material_color(&mut self, color: [f32; 4]);
    
    /// Configure directional lighting parameters
    ///
    /// # Arguments
    /// * `direction` - Light direction vector (will be normalized by backend)
    /// * `intensity` - Light intensity multiplier
    /// * `color` - RGB light color values in range [0.0, 1.0]
    /// * `ambient_intensity` - Ambient light contribution
    fn set_directional_light(&mut self, direction: [f32; 3], intensity: f32, color: [f32; 3], ambient_intensity: f32);
    
    /// Render the current frame
    ///
    /// Executes the rendering pipeline with currently bound resources.
    /// This typically includes:
    /// - Acquiring the next swapchain image
    /// - Recording and submitting command buffers
    /// - Presenting the rendered result
    ///
    /// # Returns
    /// Result indicating success or backend-specific rendering error
    fn draw_frame(&mut self) -> BackendResult<()>;
    
    /// Wait for all pending GPU operations to complete
    ///
    /// Blocks the CPU until the GPU has finished all queued work.
    /// Useful for cleanup and synchronization points.
    ///
    /// # Returns
    /// Result indicating success or backend-specific error
    fn wait_idle(&self) -> BackendResult<()>;
    
    /// Recreate the swapchain for window resize or surface changes
    ///
    /// This method handles backend-specific swapchain recreation when the
    /// window surface changes (resize, fullscreen, etc.).
    ///
    /// # Arguments
    /// * `window_handle` - Handle to window backend access for swapchain operations
    ///
    /// # Returns
    /// Result indicating success or backend-specific error
    fn recreate_swapchain(&mut self, window_handle: &mut dyn WindowBackendAccess) -> BackendResult<()>;
}

/// Helper trait for backend-specific window operations
///
/// Provides access to backend-specific window management functionality
/// that may be needed for operations like swapchain recreation.
pub trait WindowBackendAccess {
    /// Get backend-specific window handle for swapchain operations
    ///
    /// # Returns
    /// Reference to backend-specific window type, or None if not available
    fn get_vulkan_window(&mut self) -> Option<&mut crate::backend::vulkan::Window>;
}
