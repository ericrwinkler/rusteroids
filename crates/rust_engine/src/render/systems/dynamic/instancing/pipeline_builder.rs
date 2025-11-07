//! # Instanced Pipeline Builder
//!
//! Creates Vulkan graphics pipelines optimized for instanced rendering of dynamic objects.
//! Handles pipeline configuration, shader loading, and descriptor set layout management.

use crate::render::backends::vulkan::{VulkanContext, VulkanVertexLayout, ShaderModule, GraphicsPipeline};
use ash::vk;

/// Create an instanced rendering pipeline for dynamic objects
///
/// This creates a graphics pipeline optimized for instanced rendering, where
/// vertex data is shared across multiple instances and per-instance data
/// (transforms, materials) is provided via instance attributes.
///
/// # Arguments
/// * `context` - Vulkan context with device and physical device
/// * `frame_layout` - Descriptor set layout for per-frame data (camera, lighting)
/// * `material_layout` - Descriptor set layout for per-object data (textures, materials)
/// * `render_pass` - Render pass handle for pipeline compatibility
///
/// # Returns
/// Tuple of (GraphicsPipeline, PipelineLayout) on success
///
/// # Pipeline Configuration
/// - Uses standard PBR shaders (standard_pbr_vert.spv, standard_pbr_frag.spv)
/// - Instanced vertex input (vertex data + per-instance transforms)
/// - Push constants: 128 bytes (model matrix 64 + normal matrix 48 + material color 16)
/// - Descriptor sets: Frame (set 0), Material (set 1)
pub fn create_instanced_pipeline(
    context: &VulkanContext,
    frame_layout: vk::DescriptorSetLayout,
    material_layout: vk::DescriptorSetLayout,
    render_pass: vk::RenderPass,
) -> Result<(GraphicsPipeline, vk::PipelineLayout), Box<dyn std::error::Error>> {
    // Validate push constants size against device limits (industry best practice)
    // Our optimized layout: model_matrix(64) + normal_matrix(48) + material_color(16) = 128 bytes
    const REQUIRED_PUSH_CONSTANTS_SIZE: u32 = 128;
    let max_push_constants_size = context.physical_device.properties.limits.max_push_constants_size;
    
    if max_push_constants_size < REQUIRED_PUSH_CONSTANTS_SIZE {
        return Err(format!(
            "Device push constants limit ({} bytes) is less than required ({} bytes). This violates Vulkan minimum spec!",
            max_push_constants_size, REQUIRED_PUSH_CONSTANTS_SIZE
        ).into());
    } else if max_push_constants_size == REQUIRED_PUSH_CONSTANTS_SIZE {
        log::warn!(
            "Device push constants limit ({} bytes) exactly matches our usage ({} bytes). Consider reducing push constants size for better compatibility.",
            max_push_constants_size, REQUIRED_PUSH_CONSTANTS_SIZE
        );
    } else {
        log::info!(
            "Push constants validation passed: device limit {} bytes, using {} bytes ({} bytes headroom)",
            max_push_constants_size, REQUIRED_PUSH_CONSTANTS_SIZE, max_push_constants_size - REQUIRED_PUSH_CONSTANTS_SIZE
        );
    }
    
    // Load shaders optimized for instanced rendering
    let vertex_shader = ShaderModule::from_file(
        context.raw_device(),
        "target/shaders/standard_pbr_vert.spv",
    )?;
    
    let fragment_shader = ShaderModule::from_file(
        context.raw_device(),
        "target/shaders/standard_pbr_frag.spv",
    )?;
    
    // Create INSTANCED vertex input state
    // This configures per-vertex attributes (position, normal, UV) + per-instance attributes (transforms)
    let (binding_descriptions, attribute_descriptions) = VulkanVertexLayout::get_instanced_input_state();
    let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::builder()
        .vertex_binding_descriptions(&binding_descriptions)
        .vertex_attribute_descriptions(&attribute_descriptions)
        .build();
    
    // Create descriptor set layout array for pipeline creation
    let descriptor_set_layouts = [frame_layout, material_layout];
    
    // Create instanced graphics pipeline
    let pipeline = GraphicsPipeline::new_with_descriptor_layouts(
        context.raw_device(),
        render_pass,
        &vertex_shader,
        &fragment_shader,
        vertex_input_info,
        &descriptor_set_layouts,
    )?;
    
    let pipeline_handle = pipeline.handle();
    
    if pipeline_handle == vk::Pipeline::null() {
        return Err("Pipeline creation failed - returned null handle".into());
    }
    
    // Get the pipeline layout that was created with the pipeline
    let pipeline_layout = pipeline.layout();
    
    log::info!("Instanced pipeline created successfully");
    
    Ok((pipeline, pipeline_layout))
}
