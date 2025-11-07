// Vulkan backend implementation
// Organized into initialization, resources, rendering, state, and core modules

pub mod initialization;
pub mod resources;
pub mod rendering;
pub mod state;
pub mod core;
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

// Re-export state types
pub use state::framebuffer::Framebuffer;
pub use state::swapchain::Swapchain;
pub use state::sync::{Fence, Semaphore, FrameSync};
pub use state::swapchain_manager::SwapchainManager;
pub use state::sync_manager::SyncManager;
