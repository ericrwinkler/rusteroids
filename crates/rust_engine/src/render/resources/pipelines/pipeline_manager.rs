//! Pipeline manager for handling multiple graphics pipelines
//! 
//! Manages creation, storage, and retrieval of graphics pipelines for different
//! material types. Provides centralized pipeline management with automatic
//! resource cleanup.

use std::collections::HashMap;
use ash::vk;

use crate::render::backends::vulkan::{
    VulkanContext, GraphicsPipeline, ShaderModule, VulkanResult, VulkanVertexLayout,
};
use crate::render::resources::materials::{MaterialType, PipelineType};
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
        // Initialize all opaque pipelines
        log::info!("Initializing rendering pipelines...");
        
        log::debug!("[PIPELINE] Creating StandardPBR config...");
        let pbr_config = PipelineConfig::standard_pbr();
        log::debug!("[PIPELINE] StandardPBR config created");
        
        log::debug!("[PIPELINE] Creating vertex input info...");
        let (binding_descs, attr_descs) = Self::create_vertex_input_info();
        log::debug!("[PIPELINE] Vertex input info created");
        
        log::debug!("[PIPELINE] Calling create_pipeline for StandardPBR...");
        // StandardPBR pipeline - opaque objects with full PBR lighting
        self.create_pipeline(
            context,
            render_pass,
            pbr_config,
            descriptor_set_layouts,
            binding_descs,
            attr_descs,
        )?;
        log::debug!("[PIPELINE] StandardPBR pipeline created successfully");

        // Unlit pipeline - opaque objects without lighting calculations
        let (binding_descs, attr_descs) = Self::create_vertex_input_info();
        self.create_pipeline(
            context,
            render_pass,
            PipelineConfig::unlit(),
            descriptor_set_layouts,
            binding_descs,
            attr_descs,
        )?;
        
        // Initialize transparent pipelines
        log::info!("Initializing transparent rendering pipelines...");
        
        // TransparentPBR pipeline - transparent objects with PBR lighting
        let (binding_descs, attr_descs) = Self::create_vertex_input_info();
        self.create_pipeline(
            context,
            render_pass,
            PipelineConfig::transparent_pbr(),
            descriptor_set_layouts,
            binding_descs,
            attr_descs,
        )?;
        
        // TransparentUnlit pipeline - transparent objects without lighting
        let (binding_descs, attr_descs) = Self::create_vertex_input_info();
        self.create_pipeline(
            context,
            render_pass,
            PipelineConfig::transparent_unlit(),
            descriptor_set_layouts,
            binding_descs,
            attr_descs,
        )?;

        // Set StandardPBR as default active pipeline to use material UBO system
        self.active_pipeline = Some(PipelineType::StandardPBR);
        
        log::info!("Successfully initialized 4 rendering pipelines: StandardPBR, Unlit, TransparentPBR, TransparentUnlit");

        Ok(())
    }

    /// Create a single pipeline from configuration
    fn create_pipeline(
        &mut self,
        context: &VulkanContext,
        render_pass: vk::RenderPass,
        config: PipelineConfig,
        descriptor_set_layouts: &[vk::DescriptorSetLayout],
        binding_descs: [vk::VertexInputBindingDescription; 2],
        attr_descs: [vk::VertexInputAttributeDescription; 16],
    ) -> VulkanResult<()> {
        log::info!("[PIPELINE] Creating pipeline");
        
        // Build vertex input info here where arrays are in final location
        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(&binding_descs)
            .vertex_attribute_descriptions(&attr_descs)
            .build();
        
        let device = context.raw_device();
        
        log::info!("[PIPELINE] Loading vertex shader from: {}", config.vertex_shader_path);
        
        // Load shaders
        let vertex_shader = ShaderModule::from_file(
            &device,
            &config.vertex_shader_path,
        )?;

        log::info!("[PIPELINE] Loading fragment shader from: {}", config.fragment_shader_path);
        let fragment_shader = ShaderModule::from_file(
            &device,
            &config.fragment_shader_path,
        )?;
        log::info!("[PIPELINE] Both shaders loaded successfully");

        log::info!("[PIPELINE] About to create graphics pipeline with config");
        log::info!("[PIPELINE] Parameters: device={:?}, render_pass={:?}", device.handle(), render_pass);
        log::info!("[PIPELINE] vertex_input_info sType: {:?}", vertex_input_info.s_type);
        log::info!("[PIPELINE] Calling create_graphics_pipeline_with_config NOW...");
        
        // Create pipeline with custom configuration
        let pipeline = Self::create_graphics_pipeline_with_config(
            &device,
            render_pass,
            &vertex_shader,
            &fragment_shader,
            vertex_input_info,
            descriptor_set_layouts,
            &config,
        )?;
        
        log::info!("[PIPELINE] RETURNED from create_graphics_pipeline_with_config");
        log::info!("[PIPELINE] Graphics pipeline created successfully");

        self.pipelines.insert(config.pipeline_type, pipeline);
        log::debug!("[PIPELINE] Pipeline inserted into manager");

        Ok(())
    }

    /// Create graphics pipeline with custom configuration
    fn create_graphics_pipeline_with_config(
        device: &ash::Device,
        render_pass: vk::RenderPass,
        vertex_shader: &ShaderModule,
        fragment_shader: &ShaderModule,
        vertex_input_info: vk::PipelineVertexInputStateCreateInfo,
        descriptor_set_layouts: &[vk::DescriptorSetLayout],
        config: &PipelineConfig,
    ) -> VulkanResult<GraphicsPipeline> {
        use crate::render::backends::vulkan::VulkanError;

        log::debug!("[PIPELINE] create_graphics_pipeline_with_config called");
        log::debug!("[PIPELINE] Creating shader stage infos...");
        
        // Shader stages
        let vertex_entry = std::ffi::CStr::from_bytes_with_nul(b"main\0").unwrap();
        let fragment_entry = std::ffi::CStr::from_bytes_with_nul(b"main\0").unwrap();
        
        let shader_stages = [
            vertex_shader.create_stage_info(vk::ShaderStageFlags::VERTEX, vertex_entry),
            fragment_shader.create_stage_info(vk::ShaderStageFlags::FRAGMENT, fragment_entry),
        ];
        
        log::debug!("[PIPELINE] Shader stages created, setting up input assembly...");
        
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
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE) // OBJ files use CCW winding (no conversion applied)
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
            
        // Pipeline layout WITH push constants (required for instanced rendering)
        // Push constants: model_matrix(64) + normal_matrix(48) + material_color(16) = 128 bytes
        const PUSH_CONSTANTS_SIZE: u32 = 128;
        
        let push_constant_range = vk::PushConstantRange::builder()
            .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
            .offset(0)
            .size(PUSH_CONSTANTS_SIZE)
            .build();
        
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

    /// Create vertex input info for all pipelines (unified instanced rendering)
    fn create_vertex_input_info() -> ([vk::VertexInputBindingDescription; 2], [vk::VertexInputAttributeDescription; 16]) {
        VulkanVertexLayout::get_instanced_input_state()
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

    // FIXME: LEGACY - Remove after modular pipeline system is complete
    // This "active pipeline" pattern assumes single pipeline for all objects
    // New system selects pipeline per-material using get_pipeline_by_type()
    /// Set active pipeline type (LEGACY - will be removed)
    pub fn set_active_pipeline(&mut self, material_type: &MaterialType) {
        let pipeline_type = Self::material_type_to_pipeline_type(material_type);
        self.active_pipeline = Some(pipeline_type);
    }

    // FIXME: LEGACY - Remove after modular pipeline system is complete
    /// Get currently active pipeline (LEGACY - will be removed)
    pub fn get_active_pipeline(&self) -> Option<&GraphicsPipeline> {
        self.active_pipeline
            .and_then(|pipeline_type| self.pipelines.get(&pipeline_type))
    }

    // FIXME: LEGACY - Remove after modular pipeline system is complete
    /// Get active pipeline type (LEGACY - will be removed)
    pub fn get_active_pipeline_type(&self) -> Option<PipelineType> {
        self.active_pipeline
    }

    /// Check if a pipeline exists for the given material type
    pub fn has_pipeline(&self, material_type: &MaterialType) -> bool {
        let pipeline_type = Self::material_type_to_pipeline_type(material_type);
        self.pipelines.contains_key(&pipeline_type)
    }
}
