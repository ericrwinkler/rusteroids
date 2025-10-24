pub mod buffer;
pub mod commands;
pub mod context;
pub mod descriptor_set;
pub mod framebuffer;
pub mod render_pass;
pub mod shader;
pub mod surface;
pub mod swapchain;
pub mod sync;
pub mod texture;
/// Uniform buffer objects for shader constants
pub mod uniform_buffer;
pub mod vertex_layout;
pub mod window;

// Re-export commonly used types
pub use context::{VulkanContext, VulkanResult, VulkanError, PhysicalDeviceInfo, LogicalDevice, VulkanInstance};
pub use window::{Window, WindowError};
pub use swapchain::Swapchain;
pub use render_pass::RenderPass;
pub use commands::{CommandPool, CommandRecorder};
pub use framebuffer::{Framebuffer, DepthBuffer};
pub use sync::{Semaphore, Fence, FrameSync, MemoryBarrierBuilder};
pub use buffer::{Buffer, VertexBuffer, IndexBuffer, StagingBuffer, UniformBuffer as OldUniformBuffer};
pub use shader::{ShaderModule, GraphicsPipeline};
pub use uniform_buffer::*;
pub use descriptor_set::{DescriptorSetLayoutBuilder, DescriptorSetLayout, DescriptorPool};
pub use vertex_layout::VulkanVertexLayout;
pub use texture::Texture;
pub use surface::Surface;
