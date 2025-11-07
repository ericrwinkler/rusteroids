// Vulkan resource management

pub mod buffer;
pub mod texture;
pub mod uniform_buffer;
pub mod descriptor_set;
pub mod resource_manager;
pub mod texture_manager;
pub mod ubo_manager;
pub mod cache_manager;

// Re-export buffer types (except UniformBuffer which conflicts)
pub use buffer::{Buffer, VertexBuffer, IndexBuffer, StagingBuffer};
pub use texture::Texture;
// Use the uniform_buffer module's UniformBuffer as the canonical one
pub use uniform_buffer::{UniformBuffer, CameraUniformData};
pub use descriptor_set::{DescriptorPool, DescriptorSetLayout, DescriptorSetLayoutBuilder};
pub use resource_manager::ResourceManager;
pub use texture_manager::TextureManager;
pub use ubo_manager::UboManager;
pub use cache_manager::CacheManager;
