//! Swapchain and framebuffer management for Vulkan renderer
//! 
//! Handles swapchain recreation and framebuffer lifecycle.

use ash::vk;
use crate::render::backends::vulkan::*;
use crate::render::backends::vulkan::state::DepthBuffer;

/// Manages swapchain-related resources like framebuffers and depth buffers
pub struct SwapchainManager {
    framebuffers: Vec<Framebuffer>,
    depth_buffers: Vec<DepthBuffer>,
}

impl SwapchainManager {
    /// Create a new swapchain manager for handling framebuffers and depth buffers
    pub fn new(context: &VulkanContext, render_pass: &RenderPass) -> VulkanResult<Self> {
        log::debug!("Creating SwapchainManager...");
        
        let mut framebuffers = Vec::new();
        let mut depth_buffers = Vec::new();
        
        // Create framebuffers and depth buffers for each swapchain image
        for image_view in context.swapchain().image_views() {
            let depth_buffer = DepthBuffer::new(
                context.raw_device(),
                &context.instance(),
                context.physical_device().device,
                context.swapchain().extent(),
            )?;
            
            let framebuffer = Framebuffer::new(
                context.raw_device(),
                render_pass.handle(),
                &[*image_view, depth_buffer.image_view()],
                context.swapchain().extent(),
            )?;
            
            depth_buffers.push(depth_buffer);
            framebuffers.push(framebuffer);
        }
        
        log::debug!("SwapchainManager created with {} framebuffers", framebuffers.len());
        Ok(Self {
            framebuffers,
            depth_buffers,
        })
    }
    
    /// Recreate framebuffers after swapchain recreation
    pub fn recreate_framebuffers(&mut self, context: &VulkanContext, render_pass: &RenderPass) -> VulkanResult<()> {
        log::debug!("Recreating framebuffers for new swapchain...");
        
    // Clear framebuffers and depth buffers
        self.framebuffers.clear();
        self.depth_buffers.clear();
        
        // Create new framebuffers and depth buffers
        for image_view in context.swapchain().image_views() {
            let depth_buffer = DepthBuffer::new(
                context.raw_device(),
                &context.instance(),
                context.physical_device().device,
                context.swapchain().extent(),
            )?;
            
            let framebuffer = Framebuffer::new(
                context.raw_device(),
                render_pass.handle(),
                &[*image_view, depth_buffer.image_view()],
                context.swapchain().extent(),
            )?;
            
            self.depth_buffers.push(depth_buffer);
            self.framebuffers.push(framebuffer);
        }
        
        log::debug!("Framebuffers recreated successfully");
        Ok(())
    }
    
    /// Get framebuffer for a specific swapchain image
    pub fn get_framebuffer(&self, image_index: usize) -> vk::Framebuffer {
        self.framebuffers[image_index].handle()
    }
    
    /// Get the number of framebuffers
    pub fn framebuffer_count(&self) -> usize {
        self.framebuffers.len()
    }
}
