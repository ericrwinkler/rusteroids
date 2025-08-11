//! Shader management and compilation
//! 
//! SPIR-V shader loading and graphics pipeline management following RAII patterns

use ash::{vk, Device};
use std::path::Path;
use std::fs::File;
use std::io::Read;
use crate::render::vulkan::{VulkanResult, VulkanError};

/// Shader module wrapper with RAII cleanup
pub struct ShaderModule {
    device: Device,
    module: vk::ShaderModule,
}

impl ShaderModule {
    /// Create shader module from SPIR-V bytecode
    pub fn from_bytes(device: Device, bytes: &[u8]) -> VulkanResult<Self> {
        // Convert bytes to u32 slice (SPIR-V is u32-aligned)
        let (prefix, u32_slice, suffix) = unsafe { bytes.align_to::<u32>() };
        if !prefix.is_empty() || !suffix.is_empty() {
            return Err(VulkanError::InitializationFailed(
                "SPIR-V bytecode is not properly aligned".to_string()
            ));
        }
        
        let create_info = vk::ShaderModuleCreateInfo::builder()
            .code(u32_slice);
            
        let module = unsafe {
            device.create_shader_module(&create_info, None)
                .map_err(VulkanError::Api)?
        };
        
        Ok(Self { device, module })
    }
    
    /// Load shader from SPIR-V file
    pub fn from_file<P: AsRef<Path>>(device: Device, path: P) -> VulkanResult<Self> {
        let mut file = File::open(path)
            .map_err(|e| VulkanError::InitializationFailed(format!("Failed to open shader file: {}", e)))?;
            
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|e| VulkanError::InitializationFailed(format!("Failed to read shader file: {}", e)))?;
            
        Self::from_bytes(device, &bytes)
    }
    
    /// Get shader module handle
    pub fn handle(&self) -> vk::ShaderModule {
        self.module
    }
    
    /// Create shader stage create info
    pub fn create_stage_info(&self, stage: vk::ShaderStageFlags, entry_point: &std::ffi::CStr) -> vk::PipelineShaderStageCreateInfo {
        vk::PipelineShaderStageCreateInfo::builder()
            .stage(stage)
            .module(self.module)
            .name(entry_point)
            .build()
    }
}

impl Drop for ShaderModule {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_shader_module(self.module, None);
        }
    }
}

/// Graphics pipeline wrapper with RAII cleanup
pub struct GraphicsPipeline {
    device: Device,
    pipeline: vk::Pipeline,
    layout: vk::PipelineLayout,
}

impl GraphicsPipeline {
    /// Create graphics pipeline
    pub fn new(
        device: Device,
        render_pass: vk::RenderPass,
        vertex_shader: &ShaderModule,
        fragment_shader: &ShaderModule,
        vertex_input_info: vk::PipelineVertexInputStateCreateInfo,
        extent: vk::Extent2D,
    ) -> VulkanResult<Self> {
        // Shader stages
        let vertex_entry = std::ffi::CStr::from_bytes_with_nul(b"main\0").unwrap();
        let fragment_entry = std::ffi::CStr::from_bytes_with_nul(b"main\0").unwrap();
        
        let shader_stages = [
            vertex_shader.create_stage_info(vk::ShaderStageFlags::VERTEX, vertex_entry),
            fragment_shader.create_stage_info(vk::ShaderStageFlags::FRAGMENT, fragment_entry),
        ];
        
        // Input assembly
        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);
            
        // Viewport and scissor
        let viewport = vk::Viewport::builder()
            .x(0.0)
            .y(0.0)
            .width(extent.width as f32)
            .height(extent.height as f32)
            .min_depth(0.0)
            .max_depth(1.0)
            .build();
            
        let scissor = vk::Rect2D::builder()
            .offset(vk::Offset2D { x: 0, y: 0 })
            .extent(extent)
            .build();
            
        let viewports = [viewport];
        let scissors = [scissor];
        let viewport_state = vk::PipelineViewportStateCreateInfo::builder()
            .viewports(&viewports)
            .scissors(&scissors);
            
        // Rasterization
        let rasterizer = vk::PipelineRasterizationStateCreateInfo::builder()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(vk::CullModeFlags::BACK)  // Enable back-face culling
            .front_face(vk::FrontFace::CLOCKWISE)  // OBJ files converted to clockwise winding
            .depth_bias_enable(false);
            
        // Multisampling
        let multisampling = vk::PipelineMultisampleStateCreateInfo::builder()
            .sample_shading_enable(false)
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);
            
        // Depth and stencil testing
        let depth_stencil = vk::PipelineDepthStencilStateCreateInfo::builder()
            .depth_test_enable(true)
            .depth_write_enable(true)
            .depth_compare_op(vk::CompareOp::LESS) // More forgiving depth test
            .depth_bounds_test_enable(false)
            .stencil_test_enable(false);
            
        // Color blending
        let color_blend_attachment = vk::PipelineColorBlendAttachmentState::builder()
            .color_write_mask(vk::ColorComponentFlags::RGBA)
            .blend_enable(false)
            .build();
            
        let color_blend_attachments = [color_blend_attachment];
        let color_blending = vk::PipelineColorBlendStateCreateInfo::builder()
            .logic_op_enable(false)
            .attachments(&color_blend_attachments);
            
        // Pipeline layout with push constants for MVP matrix + material color + lighting
        let push_constant_range = vk::PushConstantRange {
            stage_flags: vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
            offset: 0,
            size: 112, // sizeof(mat4) + sizeof(vec4) + sizeof(vec4) + sizeof(vec4) = 64 + 16 + 16 + 16 = 112 bytes
        };
        
        let push_constant_ranges = [push_constant_range];
        let layout_info = vk::PipelineLayoutCreateInfo::builder()
            .push_constant_ranges(&push_constant_ranges);
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
            .layout(layout)
            .render_pass(render_pass)
            .subpass(0);
            
        let pipelines = unsafe {
            device.create_graphics_pipelines(
                vk::PipelineCache::null(),
                &[pipeline_info.build()],
                None
            ).map_err(|(_, err)| VulkanError::Api(err))?
        };
        
        let pipeline = pipelines[0];
        
        Ok(Self {
            device,
            pipeline,
            layout,
        })
    }
    
    /// Get pipeline handle
    pub fn handle(&self) -> vk::Pipeline {
        self.pipeline
    }
    
    /// Get layout handle
    pub fn layout(&self) -> vk::PipelineLayout {
        self.layout
    }
}

impl Drop for GraphicsPipeline {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_pipeline(self.pipeline, None);
            self.device.destroy_pipeline_layout(self.layout, None);
        }
    }
}
