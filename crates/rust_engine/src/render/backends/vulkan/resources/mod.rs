//! Vulkan resource management
//! 
//! Contains types for managing GPU resources including buffers, textures,
//! uniform buffers, descriptors, and resource caching.

/// Buffer types (vertex, index, staging, generic)
pub mod buffer;

/// Texture management and loading
pub mod texture;

/// Uniform buffer objects (UBOs) for shader data
pub mod uniform_buffer;

/// Descriptor set management
pub mod descriptor_set;

/// High-level resource manager
pub mod resource_manager;

/// Texture resource manager
pub mod texture_manager;

/// UBO resource manager
pub mod ubo_manager;

/// Resource caching system
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
