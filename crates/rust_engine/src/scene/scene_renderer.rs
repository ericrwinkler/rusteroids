//! Scene Renderer - bridges Scene Manager with Vulkan backend
//!
//! This module provides a high-level rendering interface that uses the
//! Scene Manager for culling and batching, then delegates to VulkanRenderer
//! for actual GPU command recording.
//!
//! Following Game Engine Architecture Chapter 11 - The Rendering Engine

use crate::scene::{SceneManager, RenderQueue, Frustum};
use crate::render::backends::vulkan::VulkanRenderer;
use crate::render::{RenderError, BackendResult};
use crate::foundation::math::Mat4;
use crate::ecs::World;

/// High-level scene renderer configuration
#[derive(Debug, Clone)]
pub struct SceneRendererConfig {
    /// Enable frustum culling
    pub enable_frustum_culling: bool,
    
    /// Enable automatic dirty tracking and GPU sync
    pub enable_auto_sync: bool,
    
    /// Maximum objects to render per frame (for performance limits)
    pub max_objects_per_frame: usize,
}

impl Default for SceneRendererConfig {
    fn default() -> Self {
        Self {
            enable_frustum_culling: true,
            enable_auto_sync: true,
            max_objects_per_frame: 10000,
        }
    }
}

/// Scene renderer that coordinates Scene Manager and Vulkan backend
///
/// This is the main rendering interface for the engine. It:
/// 1. Syncs ECS changes to Scene Manager (via automatic change detection)
/// 2. Builds optimized render queue (material batching, transparency sorting)
/// 3. Performs frustum culling (optional)
/// 4. Delegates GPU command recording to VulkanRenderer
/// 5. Manages GPU buffer updates for dirty objects
pub struct SceneRenderer {
    /// The Vulkan backend renderer
    renderer: VulkanRenderer,
    
    /// Configuration
    config: SceneRendererConfig,
    
    /// Current view-projection matrix for culling
    view_projection: Mat4,
}

impl SceneRenderer {
    /// Create a new scene renderer wrapping a Vulkan renderer
    pub fn new(renderer: VulkanRenderer, config: SceneRendererConfig) -> Self {
        Self {
            renderer,
            config,
            view_projection: Mat4::identity(),
        }
    }
    
    /// Create with default configuration
    pub fn with_defaults(renderer: VulkanRenderer) -> Self {
        Self::new(renderer, SceneRendererConfig::default())
    }
    
    /// Update the view-projection matrix for culling
    ///
    /// This should be called whenever the camera moves or the projection changes.
    /// The frustum is extracted from this matrix for culling.
    pub fn set_view_projection(&mut self, view_projection: Mat4) {
        self.view_projection = view_projection;
    }
    
    /// Get the underlying VulkanRenderer (for direct access if needed)
    pub fn renderer(&self) -> &VulkanRenderer {
        &self.renderer
    }
    
    /// Get mutable access to the underlying VulkanRenderer
    pub fn renderer_mut(&mut self) -> &mut VulkanRenderer {
        &mut self.renderer
    }
    
    /// Render a frame using the Scene Manager
    ///
    /// This is the main rendering entry point. It:
    /// 1. Syncs ECS world to Scene Manager (if auto_sync enabled)
    /// 2. Builds optimized render queue from Scene Manager
    /// 3. Performs frustum culling (if enabled)
    /// 4. Records GPU commands via VulkanRenderer
    /// 5. Submits and presents the frame
    ///
    /// # Arguments
    /// * `world` - The ECS world to sync from
    /// * `scene_manager` - The scene manager containing renderable objects
    ///
    /// # Returns
    /// Result indicating success or rendering error
    pub fn render_frame(
        &mut self,
        world: &mut World,
        scene_manager: &mut SceneManager,
    ) -> BackendResult<()> {
        // Step 1: Sync ECS changes to Scene Manager
        if self.config.enable_auto_sync {
            scene_manager.sync_from_world(world);
        }
        
        // Step 2: Build optimized render queue
        let render_queue = scene_manager.build_render_queue();
        
        // Step 3: Perform frustum culling (if enabled)
        let culled_queue = if self.config.enable_frustum_culling {
            self.cull_render_queue(&render_queue)
        } else {
            render_queue
        };
        
        // Step 4: Limit object count for performance
        let limited_queue = self.limit_render_queue(culled_queue);
        
        // Step 5: Record and submit GPU commands
        self.execute_render_queue(&limited_queue)?;
        
        // Step 6: Clear dirty flags after successful render
        self.clear_dirty_flags(scene_manager);
        
        Ok(())
    }
    
    /// Perform frustum culling on the render queue
    ///
    /// Removes objects outside the view frustum to avoid rendering
    /// geometry that won't be visible.
    fn cull_render_queue(&self, _queue: &RenderQueue) -> RenderQueue {
        // Extract frustum from view-projection matrix
        let _frustum = Frustum::from_matrix(&self.view_projection);
        
        // TODO: Implement actual culling by testing object bounds against frustum
        // For now, return unmodified queue
        // In a full implementation:
        // 1. Get AABB for each object from scene graph
        // 2. Test AABB against frustum planes
        // 3. Keep only objects that pass the test
        
        // Placeholder: Just clone the queue
        RenderQueue::new() // Simplified for Phase 3
    }
    
    /// Limit render queue to max_objects_per_frame
    fn limit_render_queue(&self, queue: RenderQueue) -> RenderQueue {
        // TODO: Implement proper limiting by truncating batches
        // For now, just return the queue as-is
        queue
    }
    
    /// Execute the render queue using VulkanRenderer
    ///
    /// This translates the high-level render queue into actual
    /// Vulkan command buffer recording.
    fn execute_render_queue(&mut self, _queue: &RenderQueue) -> BackendResult<()> {
        // TODO: Implement actual rendering
        // For Phase 3, we'll use the existing VulkanRenderer::draw_frame()
        // In Phase 4, we'll record batched draw calls from the queue
        
        self.renderer.draw_frame()
            .map_err(|e| RenderError::BackendError(e.to_string()))
    }
    
    /// Clear dirty flags on rendered objects
    fn clear_dirty_flags(&self, _scene_manager: &mut SceneManager) {
        // TODO: Iterate through rendered objects and call clear_dirty()
        // This tells the Scene Manager that GPU buffers are up-to-date
    }
    
    /// Recreate swapchain (delegates to renderer)
    pub fn recreate_swapchain(&mut self, window: &crate::render::backends::vulkan::Window) -> BackendResult<()> {
        self.renderer.recreate_swapchain(window)
            .map_err(|e| RenderError::BackendError(e.to_string()))
    }
    
    /// Get swapchain extent (delegates to renderer)
    pub fn get_swapchain_extent(&self) -> (u32, u32) {
        self.renderer.get_swapchain_extent()
    }
    
    /// Wait for device idle (delegates to renderer)
    pub fn wait_idle(&self) -> BackendResult<()> {
        self.renderer.wait_idle()
            .map_err(|e| RenderError::BackendError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // Note: Full integration tests require Vulkan initialization
    // These tests focus on configuration and basic structure
    
    #[test]
    fn test_scene_renderer_config_default() {
        let config = SceneRendererConfig::default();
        assert!(config.enable_frustum_culling);
        assert!(config.enable_auto_sync);
        assert_eq!(config.max_objects_per_frame, 10000);
    }
    
    #[test]
    fn test_scene_renderer_config_custom() {
        let config = SceneRendererConfig {
            enable_frustum_culling: false,
            enable_auto_sync: false,
            max_objects_per_frame: 5000,
        };
        assert!(!config.enable_frustum_culling);
        assert!(!config.enable_auto_sync);
        assert_eq!(config.max_objects_per_frame, 5000);
    }
}
