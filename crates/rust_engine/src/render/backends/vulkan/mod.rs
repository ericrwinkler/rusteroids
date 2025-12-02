//! Vulkan backend implementation
//! 
//! Organized into initialization, resources, rendering, state, and core modules.

/// Vulkan initialization types (context, surface, window)
pub mod initialization;

/// Vulkan resource management (buffers, textures, descriptors)
pub mod resources;

/// Vulkan rendering operations (shaders, pipelines, render passes, commands)
pub mod rendering;

/// Vulkan state management
pub mod state;

/// Core Vulkan types and utilities
pub mod core;

/// Per-object uniform buffer data structures
pub mod object_data;

/// Main Vulkan renderer implementation
pub mod renderer;

// Re-export main renderer
pub use renderer::VulkanRenderer;

// Re-export core initialization types
pub use initialization::context::{VulkanContext, VulkanError, VulkanResult, PhysicalDeviceInfo};
pub use initialization::surface::Surface;
pub use initialization::window::Window;

// Re-export resource types
pub use resources::buffer::Buffer;
pub use resources::texture::Texture;
pub use resources::uniform_buffer::{UniformBuffer, CameraUniformData};
pub use resources::descriptor_set::{DescriptorPool, DescriptorSetLayout, DescriptorSetLayoutBuilder};
pub use resources::resource_manager::ResourceManager;
pub use resources::texture_manager::TextureManager;
pub use resources::ubo_manager::UboManager;

// Re-export rendering types
pub use rendering::shader::{ShaderModule, GraphicsPipeline};
pub use rendering::render_pass::RenderPass;
pub use rendering::commands::{CommandPool, CommandRecorder as VulkanCommandRecorder, ActiveRenderPass};
pub use rendering::command_recorder::CommandRecorder;
pub use rendering::vertex_layout::VulkanVertexLayout;

// Re-export object data structures
pub use object_data::ObjectUBO;

// Re-export state types
pub use state::framebuffer::Framebuffer;
pub use state::swapchain::Swapchain;
pub use state::sync::{Fence, Semaphore, FrameSync};
pub use state::swapchain_manager::SwapchainManager;
pub use state::sync_manager::SyncManager;
