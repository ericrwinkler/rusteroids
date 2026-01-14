//! Vulkan implementation of UIRenderBackend
//! 
//! Provides low-level Vulkan rendering for UI primitives.
//! All UI-specific Vulkan code is contained here to keep renderer.rs focused.

use crate::ui::backend::{UIRenderBackend, FontAtlasHandle};
use crate::render::systems::text::FontAtlas;
use crate::ui::rendering::{PanelVertex, UIVertex};
use crate::render::backends::vulkan::{VulkanRenderer, VulkanError, VulkanResult};
use crate::foundation::math::Vec4;
use crate::render::resources::materials::TextureHandle;
use ash::vk;

/// Public API to initialize UI text rendering system
/// 
/// Should be called once during initialization. Loads font, rasterizes glyphs,
/// uploads atlas to GPU, and creates all necessary pipelines and descriptor sets.
pub fn initialize_ui_text_system(renderer: &mut VulkanRenderer) -> VulkanResult<()> {
    // Initialize text pipeline
    ensure_ui_text_pipeline_initialized(renderer)?;
    
    // Initialize font atlas and upload to GPU (self-contained)
    ensure_font_atlas_initialized_internal(renderer)?;
    
    log::info!("UI text rendering system initialized successfully");
    Ok(())
}

impl UIRenderBackend for VulkanRenderer {
    fn begin_ui_pass(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // UI rendering happens within the active render pass
        // No explicit begin needed - just ensure pipelines are initialized
        ensure_ui_pipeline_initialized(self)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        ensure_ui_text_pipeline_initialized(self)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        Ok(())
    }
    
    fn render_panel_batch(
        &mut self,
        vertices: &[PanelVertex],
        draws: &[(usize, usize, Vec4)],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let recording_state = self.command_recording_state.as_ref()
            .ok_or("No active command recording")?;
        
        let command_buffer = recording_state.command_buffer;
        
        // Upload all vertices
        upload_ui_panel_vertices(self, command_buffer, vertices)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        
        // Issue draw calls for each range with different colors
        for (start, count, color) in draws {
            draw_ui_panel_range(self, command_buffer, *start, *count, *color)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        }
        
        Ok(())
    }
    
    fn render_text_batch(
        &mut self,
        vertices: &[UIVertex],
        draws: &[(usize, usize)],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let recording_state = self.command_recording_state.as_ref()
            .ok_or("No active command recording")?;
        
        let command_buffer = recording_state.command_buffer;
        
        // Convert UIVertex to flat f32 array
        let mut flat_vertices = Vec::with_capacity(vertices.len() * 4);
        for vertex in vertices {
            flat_vertices.push(vertex.position[0]);
            flat_vertices.push(vertex.position[1]);
            flat_vertices.push(vertex.uv[0]);
            flat_vertices.push(vertex.uv[1]);
        }
        
        // Upload and draw
        upload_text_vertices_batch(self, command_buffer, &flat_vertices, draws)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        
        Ok(())
    }
    
    fn end_ui_pass(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // UI rendering completes within the render pass
        // No explicit end needed
        Ok(())
    }
    
    fn upload_font_atlas(
        &mut self,
        _atlas: &FontAtlas,
    ) -> Result<FontAtlasHandle, Box<dyn std::error::Error>> {
        ensure_font_atlas_initialized_internal(self)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        Ok(FontAtlasHandle(0)) // Single font atlas for now
    }
    
    fn set_screen_size(&mut self, width: u32, height: u32) {
        // Screen size is tracked by swapchain extent
        // This is a no-op for Vulkan - size comes from swapchain
        let _ = (width, height);
    }
    
    fn get_screen_size(&self) -> (u32, u32) {
        self.get_swapchain_extent()
    }
}

// ============================================================================
// UI Pipeline Initialization
// ============================================================================

/// Initialize UI pipeline and resources (lazy initialization)
/// Creates a simple pipeline for rendering UI overlays with:
/// - Depth test DISABLED
/// - Alpha blending ENABLED  
/// - Push constants for color
fn ensure_ui_pipeline_initialized(renderer: &mut VulkanRenderer) -> VulkanResult<()> {
    // Already initialized?
    if renderer.ui_pipeline.is_some() {
        return Ok(());
    }
    
    log::info!("Initializing UI overlay pipeline...");
    
    let device = renderer.context.raw_device();
    
    // Load shader SPIR-V files from disk
    let vert_code = std::fs::read("target/shaders/ui_simple_vert.spv")
        .map_err(|e| VulkanError::InvalidOperation {
            reason: format!("Failed to load UI vertex shader: {}", e)
        })?;
    let frag_code = std::fs::read("target/shaders/ui_simple_frag.spv")
        .map_err(|e| VulkanError::InvalidOperation {
            reason: format!("Failed to load UI fragment shader: {}", e)
        })?;
    
    // Create shader modules
    let vert_module = unsafe {
        let create_info = vk::ShaderModuleCreateInfo::builder()
            .code(std::slice::from_raw_parts(
                vert_code.as_ptr() as *const u32,
                vert_code.len() / 4,
            ));
        device.create_shader_module(&create_info, None)
            .map_err(VulkanError::Api)?
    };
    
    let frag_module = unsafe {
        let create_info = vk::ShaderModuleCreateInfo::builder()
            .code(std::slice::from_raw_parts(
                frag_code.as_ptr() as *const u32,
                frag_code.len() / 4,
            ));
        device.create_shader_module(&create_info, None)
            .map_err(VulkanError::Api)?
    };
    
    let entry_point = std::ffi::CString::new("main").unwrap();
    let shader_stages = [
        vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(vert_module)
            .name(&entry_point)
            .build(),
        vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(frag_module)
            .name(&entry_point)
            .build(),
    ];
    
    // Vertex input: vec2 position
    let binding_description = vk::VertexInputBindingDescription::builder()
        .binding(0)
        .stride(8) // 2 * f32
        .input_rate(vk::VertexInputRate::VERTEX)
        .build();
    
    let attribute_description = vk::VertexInputAttributeDescription::builder()
        .location(0)
        .binding(0)
        .format(vk::Format::R32G32_SFLOAT)
        .offset(0)
        .build();
    
    let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::builder()
        .vertex_binding_descriptions(std::slice::from_ref(&binding_description))
        .vertex_attribute_descriptions(std::slice::from_ref(&attribute_description));
    
    let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::builder()
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
        .primitive_restart_enable(false);
    
    // Viewport and scissor (dynamic)
    let viewport_state = vk::PipelineViewportStateCreateInfo::builder()
        .viewport_count(1)
        .scissor_count(1);
    
    let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
    let dynamic_state = vk::PipelineDynamicStateCreateInfo::builder()
        .dynamic_states(&dynamic_states);
    
    // Rasterization
    let rasterizer = vk::PipelineRasterizationStateCreateInfo::builder()
        .depth_clamp_enable(false)
        .rasterizer_discard_enable(false)
        .polygon_mode(vk::PolygonMode::FILL)
        .line_width(1.0)
        .cull_mode(vk::CullModeFlags::NONE) // UI rendered from both sides
        .front_face(vk::FrontFace::COUNTER_CLOCKWISE);
    
    // Multisampling (disabled for UI)
    let multisampling = vk::PipelineMultisampleStateCreateInfo::builder()
        .sample_shading_enable(false)
        .rasterization_samples(vk::SampleCountFlags::TYPE_1);
    
    // Depth/stencil: DISABLED for UI overlays
    let depth_stencil = vk::PipelineDepthStencilStateCreateInfo::builder()
        .depth_test_enable(false)  // UI always on top
        .depth_write_enable(false) // Don't write to depth buffer
        .depth_compare_op(vk::CompareOp::ALWAYS);
    
    // Color blending: ENABLED with alpha
    let color_blend_attachment = vk::PipelineColorBlendAttachmentState::builder()
        .color_write_mask(vk::ColorComponentFlags::RGBA)
        .blend_enable(true)
        .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
        .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
        .color_blend_op(vk::BlendOp::ADD)
        .src_alpha_blend_factor(vk::BlendFactor::ONE)
        .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
        .alpha_blend_op(vk::BlendOp::ADD)
        .build();
    
    let color_blend_attachments = [color_blend_attachment];
    let color_blending = vk::PipelineColorBlendStateCreateInfo::builder()
        .logic_op_enable(false)
        .attachments(&color_blend_attachments);
    
    // Pipeline layout with push constants for color
    let push_constant_range = vk::PushConstantRange {
        stage_flags: vk::ShaderStageFlags::FRAGMENT,
        offset: 0,
        size: 16, // vec4 color = 4 * f32 = 16 bytes
    };
    
    let layout_info = vk::PipelineLayoutCreateInfo::builder()
        .push_constant_ranges(std::slice::from_ref(&push_constant_range));
    
    let layout = unsafe {
        device.create_pipeline_layout(&layout_info, None)
            .map_err(VulkanError::Api)?
    };
    
    // Create graphics pipeline
    let pipeline_info = vk::GraphicsPipelineCreateInfo::builder()
        .stages(&shader_stages)
        .vertex_input_state(&vertex_input_info)
        .input_assembly_state(&input_assembly)
        .viewport_state(&viewport_state)
        .rasterization_state(&rasterizer)
        .multisample_state(&multisampling)
        .depth_stencil_state(&depth_stencil)
        .color_blend_state(&color_blending)
        .dynamic_state(&dynamic_state)
        .layout(layout)
        .render_pass(renderer.get_render_pass())
        .subpass(0);
    
    let pipeline = unsafe {
        device.create_graphics_pipelines(
            vk::PipelineCache::null(),
            &[pipeline_info.build()],
            None
        ).map_err(|(_, err)| VulkanError::Api(err))?[0]
    };
    
    // Clean up shader modules
    unsafe {
        device.destroy_shader_module(vert_module, None);
        device.destroy_shader_module(frag_module, None);
    }
    
    // Create vertex buffer for dynamic UI rendering
    use crate::ui::rendering::UIVertex;
    let max_vertices = 600; // 100 quads * 6 vertices each
    let buffer_size = (std::mem::size_of::<UIVertex>() * max_vertices) as vk::DeviceSize;
    
    let buffer_info = vk::BufferCreateInfo::builder()
        .size(buffer_size)
        .usage(vk::BufferUsageFlags::VERTEX_BUFFER)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);
    
    let vertex_buffer = unsafe {
        device.create_buffer(&buffer_info, None)
            .map_err(VulkanError::Api)?
    };
    
    let mem_requirements = unsafe { device.get_buffer_memory_requirements(vertex_buffer) };
    
    // Find memory type (HOST_VISIBLE | HOST_COHERENT)
    let memory_properties = vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT;
    let mem_props = unsafe {
        renderer.context.instance().get_physical_device_memory_properties(renderer.context.physical_device().device)
    };
    
    let mut memory_type_index = None;
    for i in 0..mem_props.memory_type_count {
        if (mem_requirements.memory_type_bits & (1 << i)) != 0 
            && (mem_props.memory_types[i as usize].property_flags & memory_properties) == memory_properties 
        {
            memory_type_index = Some(i);
            break;
        }
    }
    
    let memory_type_index = memory_type_index.ok_or(VulkanError::NoSuitableMemoryType)?;
    
    let alloc_info = vk::MemoryAllocateInfo::builder()
        .allocation_size(mem_requirements.size)
        .memory_type_index(memory_type_index);
    
    let vertex_buffer_memory = unsafe {
        device.allocate_memory(&alloc_info, None)
            .map_err(VulkanError::Api)?
    };
    
    unsafe {
        device.bind_buffer_memory(vertex_buffer, vertex_buffer_memory, 0)
            .map_err(VulkanError::Api)?;
    }
    
    // Store in renderer
    renderer.ui_pipeline = Some(pipeline);
    renderer.ui_pipeline_layout = Some(layout);
    renderer.ui_vertex_buffer = Some(vertex_buffer);
    renderer.ui_vertex_buffer_memory = Some(vertex_buffer_memory);
    renderer.ui_vertex_buffer_size = buffer_size;
    
    log::debug!("UI overlay pipeline initialized");
    
    Ok(())
}

/// Initialize the UI text pipeline for screen-space text rendering
/// 
/// Similar to ensure_ui_pipeline_initialized() but with texture sampling support
/// for font atlas rendering. Vertex format: [x, y, u, v] (4 floats per vertex).
fn ensure_ui_text_pipeline_initialized(renderer: &mut VulkanRenderer) -> VulkanResult<()> {
    // Already initialized?
    if renderer.ui_text_pipeline.is_some() {
        return Ok(());
    }
    
    log::info!("Initializing UI text pipeline...");
    
    let device = renderer.context.raw_device();
    
    // Step 1: Create descriptor set layout for font atlas texture
    let sampler_binding = vk::DescriptorSetLayoutBinding::builder()
        .binding(0)
        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
        .descriptor_count(1)
        .stage_flags(vk::ShaderStageFlags::FRAGMENT)
        .build();
    
    let descriptor_set_layout_info = vk::DescriptorSetLayoutCreateInfo::builder()
        .bindings(std::slice::from_ref(&sampler_binding));
    
    let descriptor_set_layout = unsafe {
        device.create_descriptor_set_layout(&descriptor_set_layout_info, None)
            .map_err(VulkanError::Api)?
    };
    
    // Step 2: Load text shaders
    let vert_code = std::fs::read("target/shaders/ui_text_vert.spv")
        .map_err(|e| VulkanError::InvalidOperation {
            reason: format!("Failed to load UI text vertex shader: {}", e)
        })?;
    let frag_code = std::fs::read("target/shaders/ui_text_frag.spv")
        .map_err(|e| VulkanError::InvalidOperation {
            reason: format!("Failed to load UI text fragment shader: {}", e)
        })?;
    
    // Create shader modules
    let vert_module = unsafe {
        let create_info = vk::ShaderModuleCreateInfo::builder()
            .code(std::slice::from_raw_parts(
                vert_code.as_ptr() as *const u32,
                vert_code.len() / 4,
            ));
        device.create_shader_module(&create_info, None)
            .map_err(VulkanError::Api)?
    };
    
    let frag_module = unsafe {
        let create_info = vk::ShaderModuleCreateInfo::builder()
            .code(std::slice::from_raw_parts(
                frag_code.as_ptr() as *const u32,
                frag_code.len() / 4,
            ));
        device.create_shader_module(&create_info, None)
            .map_err(VulkanError::Api)?
    };
    
    let entry_point = std::ffi::CString::new("main").unwrap();
    let shader_stages = [
        vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(vert_module)
            .name(&entry_point)
            .build(),
        vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(frag_module)
            .name(&entry_point)
            .build(),
    ];
    
    // Step 3: Vertex input: vec2 position + vec2 texCoord (16 bytes stride)
    let binding_description = vk::VertexInputBindingDescription::builder()
        .binding(0)
        .stride(16) // 4 * f32 (x, y, u, v)
        .input_rate(vk::VertexInputRate::VERTEX)
        .build();
    
    let attribute_descriptions = [
        // Position attribute (location 0)
        vk::VertexInputAttributeDescription::builder()
            .location(0)
            .binding(0)
            .format(vk::Format::R32G32_SFLOAT)
            .offset(0)
            .build(),
        // TexCoord attribute (location 1)
        vk::VertexInputAttributeDescription::builder()
            .location(1)
            .binding(0)
            .format(vk::Format::R32G32_SFLOAT)
            .offset(8) // After position (2 * 4 bytes)
            .build(),
    ];
    
    let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::builder()
        .vertex_binding_descriptions(std::slice::from_ref(&binding_description))
        .vertex_attribute_descriptions(&attribute_descriptions);
    
    let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::builder()
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
        .primitive_restart_enable(false);
    
    // Viewport and scissor (dynamic)
    let viewport_state = vk::PipelineViewportStateCreateInfo::builder()
        .viewport_count(1)
        .scissor_count(1);
    
    let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
    let dynamic_state = vk::PipelineDynamicStateCreateInfo::builder()
        .dynamic_states(&dynamic_states);
    
    // Rasterization
    let rasterizer = vk::PipelineRasterizationStateCreateInfo::builder()
        .depth_clamp_enable(false)
        .rasterizer_discard_enable(false)
        .polygon_mode(vk::PolygonMode::FILL)
        .line_width(1.0)
        .cull_mode(vk::CullModeFlags::NONE)
        .front_face(vk::FrontFace::COUNTER_CLOCKWISE);
    
    // Multisampling (disabled)
    let multisampling = vk::PipelineMultisampleStateCreateInfo::builder()
        .sample_shading_enable(false)
        .rasterization_samples(vk::SampleCountFlags::TYPE_1);
    
    // Depth/stencil: DISABLED for UI text
    let depth_stencil = vk::PipelineDepthStencilStateCreateInfo::builder()
        .depth_test_enable(false)
        .depth_write_enable(false)
        .depth_compare_op(vk::CompareOp::ALWAYS);
    
    // Color blending: ENABLED with alpha (for text transparency)
    let color_blend_attachment = vk::PipelineColorBlendAttachmentState::builder()
        .color_write_mask(vk::ColorComponentFlags::RGBA)
        .blend_enable(true)
        .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
        .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
        .color_blend_op(vk::BlendOp::ADD)
        .src_alpha_blend_factor(vk::BlendFactor::ONE)
        .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
        .alpha_blend_op(vk::BlendOp::ADD)
        .build();
    
    let color_blend_attachments = [color_blend_attachment];
    let color_blending = vk::PipelineColorBlendStateCreateInfo::builder()
        .logic_op_enable(false)
        .attachments(&color_blend_attachments);
    
    // Step 4: Pipeline layout with descriptor set and push constants
    let push_constant_range = vk::PushConstantRange {
        stage_flags: vk::ShaderStageFlags::FRAGMENT,
        offset: 0,
        size: 16, // vec4 textColor = 4 * f32 = 16 bytes
    };
    
    let descriptor_set_layouts = [descriptor_set_layout];
    let layout_info = vk::PipelineLayoutCreateInfo::builder()
        .set_layouts(&descriptor_set_layouts)
        .push_constant_ranges(std::slice::from_ref(&push_constant_range));
    
    let layout = unsafe {
        device.create_pipeline_layout(&layout_info, None)
            .map_err(VulkanError::Api)?
    };
    
    // Create graphics pipeline
    let pipeline_info = vk::GraphicsPipelineCreateInfo::builder()
        .stages(&shader_stages)
        .vertex_input_state(&vertex_input_info)
        .input_assembly_state(&input_assembly)
        .viewport_state(&viewport_state)
        .rasterization_state(&rasterizer)
        .multisample_state(&multisampling)
        .depth_stencil_state(&depth_stencil)
        .color_blend_state(&color_blending)
        .dynamic_state(&dynamic_state)
        .layout(layout)
        .render_pass(renderer.get_render_pass())
        .subpass(0);
    
    let pipeline = unsafe {
        device.create_graphics_pipelines(
            vk::PipelineCache::null(),
            &[pipeline_info.build()],
            None
        ).map_err(|(_, err)| VulkanError::Api(err))?[0]
    };
    
    // Clean up shader modules
    unsafe {
        device.destroy_shader_module(vert_module, None);
        device.destroy_shader_module(frag_module, None);
    }
    
    // Store resources
    renderer.ui_text_pipeline = Some(pipeline);
    renderer.ui_text_pipeline_layout = Some(layout);
    renderer.ui_text_descriptor_set_layout = Some(descriptor_set_layout);
    
    log::info!("UI text pipeline initialized successfully");
    
    Ok(())
}

// ============================================================================
// Font Atlas Initialization
// ============================================================================

/// Initialize font atlas for text rendering (Step 3) - Internal version
/// 
/// Loads font, rasterizes glyphs, uploads atlas texture to GPU, and creates descriptor set.
/// This version is self-contained and doesn't require GraphicsEngine.
fn ensure_font_atlas_initialized_internal(renderer: &mut VulkanRenderer) -> VulkanResult<()> {
    if renderer.font_atlas.is_some() && renderer.ui_text_descriptor_set.is_some() {
        return Ok(());
    }
    
    log::info!("Initializing font atlas for UI text rendering...");
    
    // Load font from resources
    let font_path = "resources/fonts/default.ttf";
    let font_data = std::fs::read(font_path)
        .map_err(|e| VulkanError::InitializationFailed(format!("Failed to load font: {}", e)))?;
    
    // Create font atlas
    let mut font_atlas = crate::render::systems::text::FontAtlas::new(&font_data, 24.0)
        .map_err(|e| VulkanError::InitializationFailed(format!("Failed to create font atlas: {}", e)))?;
    
    // Rasterize ASCII glyphs
    font_atlas.rasterize_glyphs()
        .map_err(|e| VulkanError::InitializationFailed(format!("Failed to rasterize glyphs: {}", e)))?;
    
    // Upload font atlas to GPU directly using our internal upload method
    let image_data = font_atlas.get_atlas_image_data()
        .map_err(|e| VulkanError::InitializationFailed(format!("Failed to get atlas image data: {}", e)))?;
    
    let texture_handle = renderer.upload_texture_from_image_data(
        image_data,
        crate::render::resources::materials::TextureType::BaseColor,
    ).map_err(|e| VulkanError::InitializationFailed(format!("Failed to upload font atlas: {}", e)))?;
    
    log::info!("Font atlas uploaded to GPU: {:?}", texture_handle);
    
    // Store the texture handle in the font atlas
    font_atlas.set_texture_handle(texture_handle);
    
    renderer.font_atlas = Some(font_atlas);
    
    // Create descriptor set for font atlas texture
    create_text_descriptor_set(renderer, texture_handle)?;
    
    log::info!("Font atlas initialized successfully");
    Ok(())
}

/// Create descriptor set binding the font atlas texture
fn create_text_descriptor_set(renderer: &mut VulkanRenderer, texture_handle: TextureHandle) -> VulkanResult<()> {
    let device = renderer.context.raw_device();
    
    // Get descriptor set layout
    let descriptor_set_layout = renderer.ui_text_descriptor_set_layout
        .ok_or_else(|| VulkanError::InitializationFailed("Text descriptor set layout not initialized".to_string()))?;
    
    // Allocate descriptor set from pool
    let layouts = [descriptor_set_layout];
    let alloc_info = vk::DescriptorSetAllocateInfo::builder()
        .descriptor_pool(renderer.get_descriptor_pool())
        .set_layouts(&layouts);
    
    let descriptor_sets = unsafe {
        device.allocate_descriptor_sets(&alloc_info)
            .map_err(VulkanError::Api)?
    };
    
    let descriptor_set = descriptor_sets[0];
    
    // Get texture from resource manager
    let texture = renderer.resource_manager.get_loaded_texture(texture_handle.0 as usize)
        .ok_or_else(|| VulkanError::InitializationFailed(format!("Font atlas texture {:?} not found", texture_handle)))?;
    
    // Update descriptor set with font atlas texture
    let image_info = vk::DescriptorImageInfo::builder()
        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
        .image_view(texture.image_view())
        .sampler(texture.sampler())
        .build();
    
    let descriptor_write = vk::WriteDescriptorSet::builder()
        .dst_set(descriptor_set)
        .dst_binding(0)
        .dst_array_element(0)
        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
        .image_info(std::slice::from_ref(&image_info))
        .build();
    
    unsafe {
        device.update_descriptor_sets(&[descriptor_write], &[]);
    }
    
    renderer.ui_text_descriptor_set = Some(descriptor_set);
    
    log::info!("Text descriptor set created with font atlas texture");
    Ok(())
}

// ============================================================================
// Vertex Upload and Drawing
// ============================================================================

/// Upload all panel vertices to the vertex buffer at once
fn upload_ui_panel_vertices(
    renderer: &mut VulkanRenderer,
    _command_buffer: vk::CommandBuffer,
    vertices: &[PanelVertex],
) -> VulkanResult<()> {
    let device = renderer.context.raw_device();
    
    let vertex_buffer_memory = renderer.ui_vertex_buffer_memory
        .ok_or_else(|| VulkanError::InitializationFailed("UI vertex buffer memory not initialized".to_string()))?;
    
    // Ensure buffer is large enough
    let required_size = (std::mem::size_of::<PanelVertex>() * vertices.len()) as vk::DeviceSize;
    if required_size > renderer.ui_vertex_buffer_size {
        log::warn!("UI vertex buffer too small ({} bytes needed, {} allocated)", 
                   required_size, renderer.ui_vertex_buffer_size);
        return Err(VulkanError::InvalidOperation { 
            reason: format!("UI vertex buffer too small") 
        });
    }
    
    // Upload all vertices at once
    unsafe {
        let data_ptr = device.map_memory(
            vertex_buffer_memory,
            0,
            required_size,
            vk::MemoryMapFlags::empty(),
        ).map_err(VulkanError::Api)? as *mut PanelVertex;
        
        std::ptr::copy_nonoverlapping(vertices.as_ptr(), data_ptr, vertices.len());
        
        device.unmap_memory(vertex_buffer_memory);
    }
    
    Ok(())
}

/// Draw a range of panel vertices with the given color
fn draw_ui_panel_range(
    renderer: &VulkanRenderer,
    command_buffer: vk::CommandBuffer,
    start_vertex: usize,
    vertex_count: usize,
    color: Vec4,
) -> VulkanResult<()> {
    let device = renderer.context.raw_device();
    
    let vertex_buffer = renderer.ui_vertex_buffer
        .ok_or_else(|| VulkanError::InitializationFailed("UI vertex buffer not initialized".to_string()))?;
    let pipeline = renderer.ui_pipeline
        .ok_or_else(|| VulkanError::InitializationFailed("UI pipeline not initialized".to_string()))?;
    let pipeline_layout = renderer.ui_pipeline_layout
        .ok_or_else(|| VulkanError::InitializationFailed("UI pipeline layout not initialized".to_string()))?;
    
    unsafe {
        // Bind UI pipeline (may already be bound, but safe to rebind)
        device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, pipeline);
        
        // Bind vertex buffer
        device.cmd_bind_vertex_buffers(command_buffer, 0, &[vertex_buffer], &[0]);
        
        // Push color constant
        let color_array = [color.x, color.y, color.z, color.w];
        let color_bytes = std::slice::from_raw_parts(
            color_array.as_ptr() as *const u8,
            std::mem::size_of_val(&color_array),
        );
        device.cmd_push_constants(
            command_buffer,
            pipeline_layout,
            vk::ShaderStageFlags::FRAGMENT,
            0,
            color_bytes,
        );
        
        // Draw this range
        device.cmd_draw(command_buffer, vertex_count as u32, 1, start_vertex as u32, 0);
    }
    
    Ok(())
}

/// Upload all text vertices in a single batch and issue multiple draw calls
/// This prevents buffer overwriting when rendering multiple text items
fn upload_text_vertices_batch(
    renderer: &mut VulkanRenderer, 
    command_buffer: vk::CommandBuffer, 
    all_vertices: &[f32], 
    draw_ranges: &[(usize, usize)]
) -> VulkanResult<()> {
    if all_vertices.is_empty() || draw_ranges.is_empty() {
        return Ok(());
    }
    
    let device = renderer.context.raw_device();
    let buffer_size = (all_vertices.len() * std::mem::size_of::<f32>()) as vk::DeviceSize;
    
    // Create or resize vertex buffer if needed
    if renderer.ui_text_vertex_buffer.is_none() || renderer.ui_text_vertex_buffer_size < buffer_size {
        // Clean up old buffer if exists
        if let Some(old_buffer) = renderer.ui_text_vertex_buffer {
            unsafe {
                device.destroy_buffer(old_buffer, None);
            }
        }
        if let Some(old_memory) = renderer.ui_text_vertex_buffer_memory {
            unsafe {
                device.free_memory(old_memory, None);
            }
        }
        
        // Create new buffer (allocate 2x for headroom)
        let alloc_size = buffer_size * 2;
        
        let buffer_info = vk::BufferCreateInfo::builder()
            .size(alloc_size)
            .usage(vk::BufferUsageFlags::VERTEX_BUFFER)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
        
        let buffer = unsafe {
            device.create_buffer(&buffer_info, None)
                .map_err(VulkanError::Api)?
        };
        
        let mem_requirements = unsafe { device.get_buffer_memory_requirements(buffer) };
        
        // Find HOST_VISIBLE | HOST_COHERENT memory
        let memory_properties = vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT;
        let mem_props = unsafe {
            renderer.context.instance().get_physical_device_memory_properties(renderer.context.physical_device().device)
        };
        
        let mut memory_type_index = None;
        for i in 0..mem_props.memory_type_count {
            if (mem_requirements.memory_type_bits & (1 << i)) != 0 
                && (mem_props.memory_types[i as usize].property_flags & memory_properties) == memory_properties 
            {
                memory_type_index = Some(i);
                break;
            }
        }
        
        let memory_type_index = memory_type_index.ok_or(VulkanError::NoSuitableMemoryType)?;
        
        let alloc_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(mem_requirements.size)
            .memory_type_index(memory_type_index);
        
        let memory = unsafe {
            device.allocate_memory(&alloc_info, None)
                .map_err(VulkanError::Api)?
        };
        
        unsafe {
            device.bind_buffer_memory(buffer, memory, 0)
                .map_err(VulkanError::Api)?;
        }
        
        // Need to use unsafe to modify renderer fields
        let renderer_mut = unsafe { &mut *(renderer as *const VulkanRenderer as *mut VulkanRenderer) };
        renderer_mut.ui_text_vertex_buffer = Some(buffer);
        renderer_mut.ui_text_vertex_buffer_memory = Some(memory);
        renderer_mut.ui_text_vertex_buffer_size = alloc_size;
        
        log::debug!("Created text vertex buffer: {} bytes", alloc_size);
    }
    
    // Upload ALL vertex data at once
    let buffer = renderer.ui_text_vertex_buffer.unwrap();
    let memory = renderer.ui_text_vertex_buffer_memory.unwrap();
    
    unsafe {
        let data_ptr = device.map_memory(memory, 0, buffer_size, vk::MemoryMapFlags::empty())
            .map_err(VulkanError::Api)?;
        
        std::ptr::copy_nonoverlapping(
            all_vertices.as_ptr(),
            data_ptr as *mut f32,
            all_vertices.len(),
        );
        
        device.unmap_memory(memory);
    }
    
    // Get text rendering resources
    let pipeline = renderer.ui_text_pipeline
        .ok_or_else(|| VulkanError::InitializationFailed("Text pipeline not initialized".to_string()))?;
    let pipeline_layout = renderer.ui_text_pipeline_layout
        .ok_or_else(|| VulkanError::InitializationFailed("Text pipeline layout not initialized".to_string()))?;
    let descriptor_set = renderer.ui_text_descriptor_set
        .ok_or_else(|| VulkanError::InitializationFailed("Text descriptor set not initialized".to_string()))?;
    
    unsafe {
        // Bind text pipeline once
        device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, pipeline);
        
        // Bind descriptor set once (font atlas texture)
        device.cmd_bind_descriptor_sets(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            pipeline_layout,
            0,
            &[descriptor_set],
            &[],
        );
        
        // Push text color constant (white) - shared for all text
        let text_color = [1.0f32, 1.0f32, 1.0f32, 1.0f32];
        let color_bytes = std::slice::from_raw_parts(
            text_color.as_ptr() as *const u8,
            std::mem::size_of_val(&text_color),
        );
        device.cmd_push_constants(
            command_buffer,
            pipeline_layout,
            vk::ShaderStageFlags::FRAGMENT,
            0,
            color_bytes,
        );
        
        // Bind vertex buffer once
        device.cmd_bind_vertex_buffers(command_buffer, 0, &[buffer], &[0]);
        
        // Issue multiple draw calls from different ranges of the same buffer
        for (start_vertex, vertex_count) in draw_ranges {
            device.cmd_draw(command_buffer, *vertex_count as u32, 1, *start_vertex as u32, 0);
        }
    }
    
    Ok(())
}
