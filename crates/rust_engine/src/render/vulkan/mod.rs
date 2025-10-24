//! Vulkan rendering backend
//! 
//! Low-level Vulkan implementation following the resource ownership
//! and safety rules defined in DESIGN.md

/// Core Vulkan wrappers and primitives
pub mod core;
/// High-level rendering system and orchestration
pub mod system;

// Re-export commonly used core types
pub use core::{
    context::{VulkanContext, VulkanResult, VulkanError, PhysicalDeviceInfo, LogicalDevice, VulkanInstance},
    window::{Window, WindowError},
    swapchain::Swapchain,
    render_pass::RenderPass,
    commands::{CommandPool, CommandRecorder},
    framebuffer::{Framebuffer, DepthBuffer},
    sync::{Semaphore, Fence, FrameSync, MemoryBarrierBuilder},
    buffer::{self, Buffer, VertexBuffer, IndexBuffer, StagingBuffer, UniformBuffer as OldUniformBuffer},
    shader::{self, ShaderModule, GraphicsPipeline},
    uniform_buffer::{self, CameraUniformData},
    descriptor_set::{self, DescriptorSetLayoutBuilder, DescriptorSetLayout, DescriptorPool},
    vertex_layout::VulkanVertexLayout,
    texture::Texture,
    surface::Surface,
};

// Re-export system types
pub use system::VulkanRenderer;
