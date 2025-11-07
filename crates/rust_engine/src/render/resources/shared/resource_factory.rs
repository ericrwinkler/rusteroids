//! # Shared Rendering Resource Factory
//!
//! Factory methods for creating SharedRenderingResources with proper Vulkan buffer
//! and pipeline initialization. These are internal helpers used during resource setup.

use crate::render::primitives::{Mesh, Vertex};
use crate::render::resources::materials::Material;
use crate::render::resources::shared::SharedRenderingResources;
use crate::render::backends::vulkan::{VulkanRenderer, Buffer};
use ash::vk;

/// Create shared rendering resources for standard (non-instanced) rendering
///
/// Creates Vulkan buffers and pipeline resources for a mesh that will be rendered
/// multiple times with different transforms. Uses the standard graphics pipeline.
///
/// # Arguments
/// * `vulkan_backend` - Vulkan renderer backend
/// * `mesh` - Mesh data (vertices and indices)
///
/// # Returns
/// SharedRenderingResources with buffers and pipeline configured
#[deprecated(since = "0.1.0", note = "Use load_mesh API instead")]
pub fn create_shared_rendering_resources(
    vulkan_backend: &VulkanRenderer,
    mesh: &Mesh,
    _materials: &[Material],
) -> Result<SharedRenderingResources, Box<dyn std::error::Error>> {
    log::info!("Creating SharedRenderingResources with {} vertices and {} indices", 
              mesh.vertices.len(), mesh.indices.len());
    
    // Get required components from VulkanRenderer
    let context = vulkan_backend.get_context();
    let frame_descriptor_set_layout = vulkan_backend.get_frame_descriptor_set_layout();
    let material_descriptor_set_layout = vulkan_backend.get_object_descriptor_set_layout();
    
    // Create shared vertex buffer
    let vertex_buffer = Buffer::new(
        context.raw_device(),
        context.instance().clone(),
        context.physical_device().device,
        (mesh.vertices.len() * std::mem::size_of::<Vertex>()) as vk::DeviceSize,
        vk::BufferUsageFlags::VERTEX_BUFFER,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    )?;
    
    // Upload vertex data
    vertex_buffer.write_data(&mesh.vertices)?;
    
    // Create shared index buffer
    let index_buffer = Buffer::new(
        context.raw_device(),
        context.instance().clone(),
        context.physical_device().device,
        (mesh.indices.len() * std::mem::size_of::<u32>()) as vk::DeviceSize,
        vk::BufferUsageFlags::INDEX_BUFFER,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    )?;
    
    // Upload index data
    index_buffer.write_data(&mesh.indices)?;
    
    // Get pipeline and layouts (these should already exist in the VulkanRenderer)
    let pipeline = vulkan_backend.get_graphics_pipeline();
    let pipeline_layout = vulkan_backend.get_pipeline_layout();
    
    let shared_resources = SharedRenderingResources::new(
        vertex_buffer,
        index_buffer,
        pipeline,
        pipeline_layout,
        material_descriptor_set_layout,
        frame_descriptor_set_layout,
        mesh.indices.len() as u32,
        mesh.vertices.len() as u32,
    );
    
    log::info!("SharedRenderingResources created successfully");
    Ok(shared_resources)
}

/// Create shared rendering resources for instanced rendering
///
/// Creates Vulkan buffers and an instanced graphics pipeline for a mesh that will
/// be rendered using instancing for optimal performance.
///
/// # Arguments
/// * `vulkan_backend` - Vulkan renderer backend
/// * `mesh` - Mesh data (vertices and indices)
/// * `create_instanced_pipeline_fn` - Function to create instanced pipeline
///
/// # Returns
/// SharedRenderingResources configured for instanced rendering
#[deprecated(since = "0.1.0", note = "Use load_mesh API instead")]
pub fn create_instanced_rendering_resources<F>(
    vulkan_backend: &VulkanRenderer,
    mesh: &Mesh,
    _materials: &[Material],
    create_instanced_pipeline_fn: F,
) -> Result<SharedRenderingResources, Box<dyn std::error::Error>>
where
    F: FnOnce(&crate::render::backends::vulkan::VulkanContext, vk::DescriptorSetLayout, vk::DescriptorSetLayout) 
        -> Result<(crate::render::backends::vulkan::GraphicsPipeline, vk::PipelineLayout), Box<dyn std::error::Error>>,
{
    log::info!("Creating InstancedRenderingResources with {} vertices and {} indices", 
        mesh.vertices.len(), mesh.indices.len());
        
    // Get required components from VulkanRenderer
    let context = vulkan_backend.get_context();
    let frame_descriptor_set_layout = vulkan_backend.get_frame_descriptor_set_layout();
    let material_descriptor_set_layout = vulkan_backend.get_object_descriptor_set_layout();
    
    // Create shared vertex buffer (same as regular)
    let vertex_buffer = Buffer::new(
        context.raw_device(),
        context.instance().clone(),
        context.physical_device().device,
        (mesh.vertices.len() * std::mem::size_of::<Vertex>()) as vk::DeviceSize,
        vk::BufferUsageFlags::VERTEX_BUFFER,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    )?;
    
    // Upload vertex data
    vertex_buffer.write_data(&mesh.vertices)?;
    
    // Create shared index buffer (same as regular)
    let index_buffer = Buffer::new(
        context.raw_device(),
        context.instance().clone(),
        context.physical_device().device,
        (mesh.indices.len() * std::mem::size_of::<u32>()) as vk::DeviceSize,
        vk::BufferUsageFlags::INDEX_BUFFER,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    )?;
    
    // Upload index data
    index_buffer.write_data(&mesh.indices)?;
    
    // Create INSTANCED pipeline (different from regular)
    let (graphics_pipeline, pipeline_layout) = create_instanced_pipeline_fn(
        context,
        frame_descriptor_set_layout,
        material_descriptor_set_layout,
    )?;
    
    // Debug log the pipeline handle
    log::info!("Created instanced pipeline: {:?}", graphics_pipeline.handle());
    
    // Build SharedRenderingResources with instanced pipeline
    let shared_resources = SharedRenderingResources::new_with_graphics_pipeline(
        vertex_buffer,
        index_buffer,
        graphics_pipeline,
        pipeline_layout,
        material_descriptor_set_layout,
        frame_descriptor_set_layout,
        mesh.indices.len() as u32,
        mesh.vertices.len() as u32,
    );
    
    log::info!("InstancedRenderingResources created successfully");
    Ok(shared_resources)
}
