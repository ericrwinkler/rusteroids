//! Pipeline manager for handling multiple graphics pipelines
//! 
//! Manages creation, storage, and retrieval of graphics pipelines for different
//! material types. Provides centralized pipeline management with automatic
//! resource cleanup.

use std::collections::HashMap;
use ash::vk;

use crate::render::vulkan::{
    VulkanContext, GraphicsPipeline, ShaderModule, VulkanResult, VulkanVertexLayout,
};
use crate::render::material::{MaterialType, PipelineType};
use super::pipeline_config::{PipelineConfig, CullMode};

/// Manages multiple graphics pipelines for different material types
pub struct PipelineManager {
    /// Map from pipeline type to graphics pipeline
    pipelines: HashMap<PipelineType, GraphicsPipeline>,
    /// Current active pipeline type
    active_pipeline: Option<PipelineType>,
}

impl PipelineManager {
    /// Create a new pipeline manager
    pub fn new() -> Self {
        Self {
            pipelines: HashMap::new(),
            active_pipeline: None,
        }
    }

    /// Initialize pipelines with standard configurations
    pub fn initialize_standard_pipelines(
        &mut self,
        context: &VulkanContext,
        render_pass: vk::RenderPass,
        descriptor_set_layouts: &[vk::DescriptorSetLayout],
    ) -> VulkanResult<()> {
        // Initialize Unlit pipeline first (simpler, should work with current setup)
        self.create_pipeline(
            context,
            render_pass,
            PipelineConfig::unlit(),
            Self::create_vertex_input_info(),
            descriptor_set_layouts,
        )?;

        // Set Unlit as default active pipeline for now
        self.active_pipeline = Some(PipelineType::Unlit);

        // TODO: Initialize other pipelines once material UBO descriptor set is properly configured
        // - StandardPBR pipeline
        // - TransparentPBR pipeline  
        // - TransparentUnlit pipeline

        Ok(())
    }

    /// Create a single pipeline from configuration
    fn create_pipeline(
        &mut self,
        context: &VulkanContext,
        render_pass: vk::RenderPass,
        config: PipelineConfig,
        vertex_input_info: vk::PipelineVertexInputStateCreateInfo,
        descriptor_set_layouts: &[vk::DescriptorSetLayout],
    ) -> VulkanResult<()> {
        log::debug!("Creating {:?} pipeline", config.pipeline_type);

        // Load shaders
        let vertex_shader = ShaderModule::from_file(
            context.raw_device(),
            &config.vertex_shader_path,
        )?;

        let fragment_shader = ShaderModule::from_file(
            context.raw_device(),
            &config.fragment_shader_path,
        )?;

        // Create pipeline with custom configuration
        let pipeline = Self::create_graphics_pipeline_with_config(
            context.raw_device(),
            render_pass,
            &vertex_shader,
            &fragment_shader,
            vertex_input_info,
            descriptor_set_layouts,
            &config,
        )?;

        self.pipelines.insert(config.pipeline_type, pipeline);
        log::debug!("{:?} pipeline created successfully", config.pipeline_type);

        Ok(())
    }

    /// Create graphics pipeline with custom configuration
    fn create_graphics_pipeline_with_config(
        device: ash::Device,
        render_pass: vk::RenderPass,
        vertex_shader: &ShaderModule,
        fragment_shader: &ShaderModule,
        vertex_input_info: vk::PipelineVertexInputStateCreateInfo,
        descriptor_set_layouts: &[vk::DescriptorSetLayout],
        config: &PipelineConfig,
    ) -> VulkanResult<GraphicsPipeline> {
        use crate::render::vulkan::VulkanError;

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
            
        // Viewport and scissor (set dynamically)
        let viewport_state = vk::PipelineViewportStateCreateInfo::builder()
            .viewport_count(1)
            .scissor_count(1);
        
        // Dynamic state
        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state = vk::PipelineDynamicStateCreateInfo::builder()
            .dynamic_states(&dynamic_states);
            
        // Rasterization with configurable cull mode
        let cull_mode_flags = match config.cull_mode {
            CullMode::None => vk::CullModeFlags::NONE,
            CullMode::Front => vk::CullModeFlags::FRONT,
            CullMode::Back => vk::CullModeFlags::BACK,
        };

        let rasterizer = vk::PipelineRasterizationStateCreateInfo::builder()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(cull_mode_flags)
            .front_face(vk::FrontFace::CLOCKWISE)
            .depth_bias_enable(false);
            
        // Multisampling
        let multisampling = vk::PipelineMultisampleStateCreateInfo::builder()
            .sample_shading_enable(false)
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);
            
        // Depth and stencil testing with configuration
        let depth_stencil = vk::PipelineDepthStencilStateCreateInfo::builder()
            .depth_test_enable(config.depth_test)
            .depth_write_enable(config.depth_write)
            .depth_compare_op(vk::CompareOp::LESS)
            .depth_bounds_test_enable(false)
            .stencil_test_enable(false);
            
        // Color blending with alpha blending configuration
        let color_blend_attachment = if config.alpha_blending {
            // Enable alpha blending for transparency
            vk::PipelineColorBlendAttachmentState::builder()
                .color_write_mask(vk::ColorComponentFlags::RGBA)
                .blend_enable(true)
                .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
                .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                .color_blend_op(vk::BlendOp::ADD)
                .src_alpha_blend_factor(vk::BlendFactor::ONE)
                .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
                .alpha_blend_op(vk::BlendOp::ADD)
                .build()
        } else {
            // No blending for opaque materials
            vk::PipelineColorBlendAttachmentState::builder()
                .color_write_mask(vk::ColorComponentFlags::RGBA)
                .blend_enable(false)
                .build()
        };
            
        let color_blend_attachments = [color_blend_attachment];
        let color_blending = vk::PipelineColorBlendStateCreateInfo::builder()
            .logic_op_enable(false)
            .attachments(&color_blend_attachments);
            
        // Pipeline layout with push constants
        let push_constant_range = vk::PushConstantRange {
            stage_flags: vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
            offset: 0,
            size: 112, // Model matrix + normal matrix (simplified, no material color)
        };
        
        let push_constant_ranges = [push_constant_range];
        let layout_info = vk::PipelineLayoutCreateInfo::builder()
            .set_layouts(descriptor_set_layouts)
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
            .dynamic_state(&dynamic_state)
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
        
        Ok(GraphicsPipeline::from_raw(device, pipeline, layout))
    }

    /// Create vertex input info for all pipelines
    fn create_vertex_input_info() -> vk::PipelineVertexInputStateCreateInfo {
        let (binding_desc, attr_desc) = VulkanVertexLayout::get_input_state();
        let binding_descriptions = [binding_desc];
        
        vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(&binding_descriptions)
            .vertex_attribute_descriptions(&attr_desc)
            .build()
    }

    /// Convert MaterialType to PipelineType
    fn material_type_to_pipeline_type(material_type: &MaterialType) -> PipelineType {
        match material_type {
            MaterialType::StandardPBR(_) => PipelineType::StandardPBR,
            MaterialType::Unlit(_) => PipelineType::Unlit,
            MaterialType::Transparent { base_material, .. } => {
                // Transparent materials use the same pipeline as base but with blending
                match **base_material {
                    MaterialType::StandardPBR(_) => PipelineType::TransparentPBR,
                    MaterialType::Unlit(_) => PipelineType::TransparentUnlit,
                    MaterialType::Transparent { .. } => {
                        // Nested transparent materials not supported, default to transparent PBR
                        PipelineType::TransparentPBR
                    }
                }
            }
        }
    }

    /// Get pipeline for a material type
    pub fn get_pipeline(&self, material_type: &MaterialType) -> Option<&GraphicsPipeline> {
        let pipeline_type = Self::material_type_to_pipeline_type(material_type);
        self.pipelines.get(&pipeline_type)
    }

    /// Get pipeline by pipeline type
    pub fn get_pipeline_by_type(&self, pipeline_type: PipelineType) -> Option<&GraphicsPipeline> {
        self.pipelines.get(&pipeline_type)
    }

    /// Set active pipeline type
    pub fn set_active_pipeline(&mut self, material_type: &MaterialType) {
        let pipeline_type = Self::material_type_to_pipeline_type(material_type);
        self.active_pipeline = Some(pipeline_type);
    }

    /// Get currently active pipeline
    pub fn get_active_pipeline(&self) -> Option<&GraphicsPipeline> {
        self.active_pipeline
            .and_then(|pipeline_type| self.pipelines.get(&pipeline_type))
    }

    /// Get active pipeline type
    pub fn get_active_pipeline_type(&self) -> Option<PipelineType> {
        self.active_pipeline
    }

    /// Check if a pipeline exists for the given material type
    pub fn has_pipeline(&self, material_type: &MaterialType) -> bool {
        let pipeline_type = Self::material_type_to_pipeline_type(material_type);
        self.pipelines.contains_key(&pipeline_type)
    }
}
