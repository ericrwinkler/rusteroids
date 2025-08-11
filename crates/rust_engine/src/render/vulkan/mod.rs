//! Vulkan rendering backend
//! 
//! Low-level Vulkan implementation following the resource ownership
//! and safety rules defined in DESIGN.md

pub mod context;
pub mod window;
pub mod swapchain;
pub mod render_pass;
pub mod commands;
pub mod framebuffer;
pub mod sync;
pub mod buffer;
pub mod shader;
pub mod renderer;
pub mod uniform_buffer;
pub mod descriptor_set;

pub use context::{VulkanContext, VulkanResult, VulkanError, PhysicalDeviceInfo, LogicalDevice, VulkanInstance};
pub use window::{Window, WindowError};
pub use swapchain::Swapchain;
pub use render_pass::RenderPass;
pub use commands::{CommandPool, CommandRecorder};
pub use framebuffer::{Framebuffer, DepthBuffer};
pub use sync::{Semaphore, Fence, FrameSync};
pub use buffer::{Buffer, VertexBuffer, IndexBuffer, UniformBuffer as OldUniformBuffer};
pub use shader::{ShaderModule, GraphicsPipeline};
pub use renderer::VulkanRenderer;
pub use uniform_buffer::*;
pub use descriptor_set::*;
