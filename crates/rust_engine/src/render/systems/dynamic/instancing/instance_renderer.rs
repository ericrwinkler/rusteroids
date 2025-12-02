//! Instance Renderer for Dynamic Objects
//!
//! Implements efficient Vulkan instanced rendering for dynamic objects using vkCmdDrawInstanced.
//! This system replaces individual draw calls with a single instanced draw, significantly
//! reducing command buffer overhead and improving performance.
//!
//! # Architecture
//!
//! ```text
//! DynamicObjectManager  → InstanceRenderer → Vulkan Command Buffer
//!         |                      |
//!   Active Objects        Instance Buffer
//!                              |
//!                        vkCmdDrawInstanced
//! ```
//!
//! # Usage
//!
//! ```rust
//! let mut renderer = InstanceRenderer::new(context, 100)?;
//! 
//! // Begin frame
//! renderer.begin_frame(command_buffer)?;
//! 
//! // Upload instance data
//! renderer.upload_instance_data(&active_objects)?;
//! 
//! // Record draw calls  
//! renderer.record_instanced_draw(&shared_resources, context, frame_descriptor_set, material_descriptor_set)?;
//! 
//! // End frame
//! renderer.end_frame()?;
//! ```

use crate::render::backends::vulkan::{VulkanContext, VulkanResult, VulkanError, Buffer};
// Note: PushConstants no longer used - true instancing uses vertex attributes
use crate::render::SharedRenderingResources;
use crate::render::systems::dynamic::object_manager::{DynamicRenderData, DynamicObjectHandle};
use crate::foundation::math::Mat4;
use ash::vk;
use std::collections::HashMap;

/// Maximum number of instances that can be rendered in a single draw call
pub const MAX_INSTANCES_PER_DRAW: usize = 100;

/// Instance data structure for GPU upload
/// Must match shader layout exactly
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct InstanceData {
    /// Model transformation matrix (4x4)
    pub model_matrix: [[f32; 4]; 4],
    /// Normal transformation matrix (3x4, padded to 4x4 for alignment)
    pub normal_matrix: [[f32; 4]; 4],
    /// Material color override
    pub material_color: [f32; 4],
    /// Emission color (RGB) + emission strength (A)
    pub emission_color: [f32; 4],
    /// Texture enable flags: [base_color, normal, unused, unused]
    pub texture_enable_flags: [u32; 4],
    /// Material index for texture/material lookup
    pub material_index: u32,
    /// Padding for proper alignment
    pub _padding: [u32; 3],
}

unsafe impl bytemuck::Pod for InstanceData {}
unsafe impl bytemuck::Zeroable for InstanceData {}

impl InstanceData {
    /// Create instance data from dynamic render data
    pub fn from_render_data(render_data: &DynamicRenderData) -> Self {
        let model_matrix = render_data.get_model_matrix();
        let normal_matrix = Self::calculate_normal_matrix(&model_matrix);
        
        // Extract material properties from the DynamicRenderData
        let (material_color, material_index) = Self::extract_material_properties(&render_data.material);
        let emission_color = Self::extract_emission_properties(&render_data.material);
        let texture_enable_flags = Self::extract_texture_flags(&render_data.material);
        
        Self {
            model_matrix: model_matrix.into(),
            normal_matrix,
            material_color,
            emission_color,
            texture_enable_flags,
            material_index,
            _padding: [0; 3],
        }
    }
    
    /// Extract emission color (RGB + strength) from Material
    fn extract_emission_properties(material: &crate::render::resources::materials::Material) -> [f32; 4] {
        use crate::render::resources::materials::MaterialType;
        
        match &material.material_type {
            MaterialType::StandardPBR(params) => {
                // Use emission from PBR parameters
                [params.emission.x, params.emission.y, params.emission.z, params.emission_strength]
            }
            MaterialType::Unlit(_params) => {
                // Unlit materials don't have emission
                [0.0, 0.0, 0.0, 0.0]
            }
            MaterialType::Transparent { base_material, .. } => {
                // Handle transparent materials by extracting from base material
                match base_material.as_ref() {
                    MaterialType::StandardPBR(params) => {
                        [params.emission.x, params.emission.y, params.emission.z, params.emission_strength]
                    }
                    _ => {
                        [0.0, 0.0, 0.0, 0.0]
                    }
                }
            }
        }
    }
    
    /// Extract texture enable flags from Material
    fn extract_texture_flags(material: &crate::render::resources::materials::Material) -> [u32; 4] {
        use crate::render::resources::materials::MaterialType;
        
        match &material.material_type {
            MaterialType::StandardPBR(params) => {
                // Extract texture flags from PBR parameters
                // [base_color, normal, unused, unused]
                [
                    params.base_color_texture_enabled as u32,
                    params.normal_texture_enabled as u32,
                    0,
                    0,
                ]
            }
            MaterialType::Unlit(_params) => {
                // Unlit materials don't use textures
                [0, 0, 0, 0]
            }
            MaterialType::Transparent { base_material, .. } => {
                // Handle transparent materials by extracting from base material
                match base_material.as_ref() {
                    MaterialType::StandardPBR(params) => {
                        [
                            params.base_color_texture_enabled as u32,
                            params.normal_texture_enabled as u32,
                            0,
                            0,
                        ]
                    }
                    _ => {
                        [0, 0, 0, 0]
                    }
                }
            }
        }
    }
    
    /// Extract material color and index from Material
    fn extract_material_properties(material: &crate::render::resources::materials::Material) -> ([f32; 4], u32) {
        use crate::render::resources::materials::MaterialType;
        
        match &material.material_type {
            MaterialType::StandardPBR(params) => {
                // Use base color from PBR parameters
                ([params.base_color.x, params.base_color.y, params.base_color.z, params.alpha], 0)
            }
            MaterialType::Unlit(params) => {
                // Use color from unlit parameters
                ([params.color.x, params.color.y, params.color.z, params.alpha], 1)
            }
            MaterialType::Transparent { base_material, .. } => {
                // Handle transparent materials by extracting from base material
                match base_material.as_ref() {
                    MaterialType::StandardPBR(params) => {
                        ([params.base_color.x, params.base_color.y, params.base_color.z, params.alpha], 0)
                    }
                    MaterialType::Unlit(params) => {
                        ([params.color.x, params.color.y, params.color.z, params.alpha], 1)
                    }
                    MaterialType::Transparent { .. } => {
                        // Nested transparent materials, use default white
                        ([1.0, 1.0, 1.0, 1.0], 0)
                    }
                }
            }
        }
    }
    
    /// Calculate normal matrix from model matrix
    fn calculate_normal_matrix(model_matrix: &Mat4) -> [[f32; 4]; 4] {
        // Extract 3x3 rotation/scale part and invert transpose for normals
        let mat3 = model_matrix.fixed_view::<3, 3>(0, 0);
        let normal_mat3 = mat3.try_inverse().unwrap_or_else(|| mat3.clone_owned()).transpose();
        
        // Convert to 4x4 with padding
        [
            [normal_mat3[(0, 0)], normal_mat3[(0, 1)], normal_mat3[(0, 2)], 0.0],
            [normal_mat3[(1, 0)], normal_mat3[(1, 1)], normal_mat3[(1, 2)], 0.0],
            [normal_mat3[(2, 0)], normal_mat3[(2, 1)], normal_mat3[(2, 2)], 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ]
    }
}

/// Batch of instances to be rendered together
#[derive(Debug)]
pub struct InstanceBatch {
    /// Instance data for this batch
    pub instances: Vec<InstanceData>,
    /// Shared mesh data (vertex/index buffers)
    pub mesh_handle: u32, // TODO: Replace with proper mesh handle
    /// Material type for this batch
    pub material_type: u32,
}

impl InstanceBatch {
    /// Create a new empty instance batch
    pub fn new(mesh_handle: u32, material_type: u32) -> Self {
        Self {
            instances: Vec::with_capacity(MAX_INSTANCES_PER_DRAW),
            mesh_handle,
            material_type,
        }
    }
    
    /// Add an instance to this batch
    pub fn add_instance(&mut self, instance_data: InstanceData) -> Result<(), &'static str> {
        if self.instances.len() >= MAX_INSTANCES_PER_DRAW {
            return Err("Batch is full");
        }
        
        self.instances.push(instance_data);
        Ok(())
    }
    
    /// Check if batch is full
    pub fn is_full(&self) -> bool {
        self.instances.len() >= MAX_INSTANCES_PER_DRAW
    }
    
    /// Get number of instances in this batch
    pub fn instance_count(&self) -> u32 {
        self.instances.len() as u32
    }
}

/// Vulkan instanced rendering pipeline for dynamic objects
pub struct InstanceRenderer {
    /// Instance buffer for GPU upload
    instance_buffer: Buffer,
    
    /// Mapped memory for instance buffer
    instance_buffer_mapped: *mut u8,
    
    /// Current frame index
    current_frame: usize,
    
    /// Maximum number of instances supported
    max_instances: usize,
    
    /// Batches organized by mesh and material
    batches: Vec<InstanceBatch>,
    
    /// Current active command buffer
    active_command_buffer: Option<vk::CommandBuffer>,
    
    /// Performance statistics
    stats: RenderStats,
}

/// Performance statistics for instance rendering
#[derive(Debug, Default)]
pub struct RenderStats {
    /// Number of instances rendered this frame
    pub instances_rendered: u32,
    /// Number of draw calls issued this frame
    pub draw_calls: u32,
    /// Number of batches processed this frame
    pub batches_processed: u32,
    /// GPU memory used for instance data (bytes)
    pub gpu_memory_used: u64,
}

impl InstanceRenderer {
    /// Create a new instance renderer
    pub fn new(context: &VulkanContext, max_instances: usize) -> VulkanResult<Self> {
        log::info!("Creating InstanceRenderer with {} max instances", max_instances);
        
        // Calculate buffer size (instances * size per instance * frames in flight)
        let frames_in_flight = 2; // TODO: Get from context
        let instance_size = std::mem::size_of::<InstanceData>();
        let buffer_size = max_instances * instance_size * frames_in_flight;
        
        // Create instance buffer with HOST_VISIBLE | HOST_COHERENT for direct mapping
        let instance_buffer = Buffer::new(
            context.raw_device().clone(),
            context.instance().clone(),
            context.physical_device().device,
            buffer_size as u64,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;
        
        // Map buffer memory persistently
        let instance_buffer_mapped = instance_buffer.map_memory()? as *mut u8;
        
        Ok(Self {
            instance_buffer,
            instance_buffer_mapped,
            current_frame: 0,
            max_instances,
            batches: Vec::new(),
            active_command_buffer: None,
            stats: RenderStats::default(),
        })
    }
    
    /// Begin frame rendering
    pub fn begin_frame(&mut self, command_buffer: vk::CommandBuffer, frame_index: usize) -> VulkanResult<()> {
        self.current_frame = frame_index;
        self.active_command_buffer = Some(command_buffer);
        
        // Clear batches from previous frame
        self.batches.clear();
        
        // Reset statistics
        self.stats = RenderStats::default();
        
        log::trace!("InstanceRenderer began frame {}", frame_index);
        Ok(())
    }
    
    /// Set active command buffer without frame lifecycle management
    ///
    /// This is used when the command buffer is already in RECORDING state
    /// and we just need to record additional commands into it.
    ///
    /// # Arguments
    /// * `command_buffer` - Already active Vulkan command buffer
    /// * `frame_index` - Current frame index
    pub fn set_active_command_buffer(&mut self, command_buffer: vk::CommandBuffer, frame_index: usize) {
        self.current_frame = frame_index;
        self.active_command_buffer = Some(command_buffer);
        
        // Always clear batches - each mesh type gets a fresh start
        self.batches.clear();
        
        // Reset statistics for this mesh type
        self.stats = RenderStats::default();
        
        log::trace!("InstanceRenderer set active command buffer for frame {} (reset for mesh type)", frame_index);
    }
    
    /// Upload instance data from dynamic objects
    pub fn upload_instance_data(
        &mut self, 
        dynamic_objects: &HashMap<DynamicObjectHandle, DynamicRenderData>
    ) -> VulkanResult<()> {
        if dynamic_objects.is_empty() {
            return Ok(());
        }
        
        // Group objects by mesh and material to create batches
        self.create_batches(dynamic_objects)?;
        
        // Upload all batch data to GPU
        self.upload_batches_to_gpu()?;
        
        self.stats.instances_rendered = dynamic_objects.len() as u32;
        self.stats.batches_processed = self.batches.len() as u32;
        
        log::trace!("Uploaded {} instances in {} batches", 
                   self.stats.instances_rendered, self.stats.batches_processed);
        
        Ok(())
    }
    
    /// Create batches from dynamic objects
    fn create_batches(
        &mut self,
        dynamic_objects: &HashMap<DynamicObjectHandle, DynamicRenderData>
    ) -> VulkanResult<()> {
        // CRITICAL: Clear previous batches to avoid accumulation across pipeline groups
        self.batches.clear();
        
        // Validate object count doesn't exceed our allocated capacity
        if dynamic_objects.len() > self.max_instances {
            return Err(VulkanError::InitializationFailed(
                format!("Too many dynamic objects: {} > max {}", dynamic_objects.len(), self.max_instances)
            ));
        }
        
        // For now, put all objects in a single batch with default mesh/material
        // TODO: Group by actual mesh and material types
        let mut batch = InstanceBatch::new(0, 0); // Default mesh and material
        
        for (_handle, render_data) in dynamic_objects {
            // Validate render data before processing - extract position from transform
            let position = render_data.transform.column(3).xyz();
            if position.x.is_nan() || position.y.is_nan() || position.z.is_nan() {
                log::warn!("Skipping object with NaN position: {:?}", position);
                continue;
            }
            
            let instance_data = InstanceData::from_render_data(render_data);
            
            // Add to current batch or create new batch if full
            if batch.is_full() {
                self.batches.push(batch);
                batch = InstanceBatch::new(0, 0);
            }
            
            if let Err(e) = batch.add_instance(instance_data) {
                return Err(VulkanError::InitializationFailed(
                    format!("Failed to add instance to batch: {}", e)
                ));
            }
        }
        
        // Add final batch if not empty
        if !batch.instances.is_empty() {
            self.batches.push(batch);
        }
        
        Ok(())
    }
    
    /// Upload batch data to GPU instance buffer
    fn upload_batches_to_gpu(&mut self) -> VulkanResult<()> {
        let instance_size = std::mem::size_of::<InstanceData>();
        let frame_offset = self.current_frame * self.max_instances * instance_size;
        
        // Validate mapped buffer is not null
        if self.instance_buffer_mapped.is_null() {
            return Err(VulkanError::InitializationFailed(
                "Instance buffer not mapped".to_string()
            ));
        }
        
        let mut current_offset = frame_offset;
        
        for batch in &self.batches {
            let batch_size = batch.instances.len() * instance_size;
            
            // Check bounds
            if current_offset + batch_size > self.instance_buffer.size() as usize {
                return Err(VulkanError::InitializationFailed(
                    "Instance buffer overflow".to_string()
                ));
            }
            
            // Copy batch data to mapped buffer
            log::trace!("Copying batch of {} bytes at offset {}", batch_size, current_offset);
            unsafe {
                let dst_ptr = self.instance_buffer_mapped.add(current_offset);
                let src_ptr = batch.instances.as_ptr() as *const u8;
                
                // Validate pointers before copy
                if dst_ptr.is_null() || src_ptr.is_null() {
                    return Err(VulkanError::InitializationFailed(
                        "Null pointer in batch copy".to_string()
                    ));
                }
                
                std::ptr::copy_nonoverlapping(src_ptr, dst_ptr, batch_size);
            }
            log::trace!("Batch copy completed successfully");
            
            current_offset += batch_size;
        }
        
        self.stats.gpu_memory_used = current_offset as u64;
        
        log::trace!("Uploaded {} bytes to GPU instance buffer", current_offset);
        Ok(())
    }
    
    /// Record multiple individual draw calls (one per object)
    /// 
    /// This converts from instanced rendering to multiple individual draw calls,
    /// which works with existing shaders that expect push constants for per-object data.
    pub fn record_instanced_draw(
        &mut self,
        shared_resources: &SharedRenderingResources,
        context: &VulkanContext,
        frame_descriptor_set: vk::DescriptorSet,
        material_descriptor_set: vk::DescriptorSet,
    ) -> VulkanResult<()> {
        let command_buffer = self.active_command_buffer
            .ok_or_else(|| VulkanError::InvalidOperation {
                reason: "No active command buffer".to_string()
            })?;
        
        if self.batches.is_empty() {
            log::trace!("No batches to render");
            return Ok(());
        }
        
        let device = context.raw_device();
        
        // Validate shared resources
        if shared_resources.index_count == 0 {
            return Err(VulkanError::InvalidOperation {
                reason: "No indices to render".to_string()
            });
        }
        
        log::trace!("Recording multiple individual draw calls...");
        
        // Validate critical handles before unsafe operations
        let vertex_buffer_handle = shared_resources.vertex_buffer.handle();
        
        if vertex_buffer_handle == vk::Buffer::null() {
            return Err(VulkanError::InvalidOperation {
                reason: "Vertex buffer handle is null".to_string()
            });
        }
        
        if command_buffer == vk::CommandBuffer::null() {
            return Err(VulkanError::InvalidOperation {
                reason: "Command buffer is null".to_string()
            });
        }
        
        log::trace!("All handles validated - proceeding with Vulkan calls");
        
        unsafe {
            log::trace!("Binding graphics pipeline...");
            
            // FIXME: LEGACY - Remove after modular pipeline system is complete
            // This binds the pipeline from SharedRenderingResources (single pipeline approach)
            // New system will pass pipeline as explicit parameter based on material type
            // Check if pipeline is null and return error instead of crashing
            if shared_resources.pipeline == vk::Pipeline::null() {
                return Err(VulkanError::InvalidOperation {
                    reason: "Pipeline handle is null - SharedRenderingResources was created with invalid pipeline".to_string()
                });
            }
            
            // CRITICAL: Bind graphics pipeline before any draw commands
            device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                shared_resources.pipeline,
            );
            
            log::trace!("Binding vertex buffers (mesh data only - no instance data)...");
            // Bind ONLY vertex buffers (mesh data) - no instance data needed
            device.cmd_bind_vertex_buffers(
                command_buffer,
                0, // binding 0: vertex data
                &[vertex_buffer_handle],
                &[0],
            );
            
            log::trace!("Binding index buffer...");
            // Bind index buffer
            device.cmd_bind_index_buffer(
                command_buffer,
                shared_resources.index_buffer.handle(),
                0,
                vk::IndexType::UINT32,
            );
            
            // FIXME: LEGACY - Remove after modular pipeline system is complete
            // This uses pipeline_layout from SharedRenderingResources
            // New system will get pipeline_layout from the explicit pipeline parameter
            // Bind both descriptor sets in a single call (set 0: frame data, set 1: material data)
            log::trace!("Binding descriptor sets...");
            device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                shared_resources.pipeline_layout,
                0, // Starting from set 0
                &[frame_descriptor_set, material_descriptor_set], // Both sets in order
                &[],
            );
            
            // Set dynamic viewport (required by pipeline)
            log::trace!("Setting dynamic viewport...");
            let extent = context.swapchain().extent();
            let viewport = vk::Viewport::builder()
                .x(0.0)
                .y(0.0)
                .width(extent.width as f32)
                .height(extent.height as f32)
                .min_depth(0.0)
                .max_depth(1.0)
                .build();
            device.cmd_set_viewport(command_buffer, 0, &[viewport]);
            
            // Set dynamic scissor (required by pipeline)
            log::trace!("Setting dynamic scissor...");
            let scissor = vk::Rect2D::builder()
                .offset(vk::Offset2D { x: 0, y: 0 })
                .extent(extent)
                .build();
            device.cmd_set_scissor(command_buffer, 0, &[scissor]);
            
            // TRUE VULKAN INSTANCING: Instance data already uploaded in upload_batches_to_gpu()
            let mut total_objects = 0;
            
            log::trace!("Recording true instanced draws for {} batches", self.batches.len());
            
            // Bind both vertex buffer and instance buffer with frame offset
            let instance_size = std::mem::size_of::<InstanceData>();
            let frame_offset = (self.current_frame * self.max_instances * instance_size) as vk::DeviceSize;
            let vertex_buffers = [shared_resources.vertex_buffer.handle(), self.instance_buffer.handle()];
            let offsets = [0, frame_offset]; // Use frame offset for instance buffer to prevent race conditions
            device.cmd_bind_vertex_buffers(command_buffer, 0, &vertex_buffers, &offsets);
            
            // Record one instanced draw call per batch
            let mut current_instance_offset = 0u32;
            for (batch_idx, batch) in self.batches.iter().enumerate() {
                let instance_count = batch.instance_count();
                if instance_count == 0 {
                    continue;
                }
                
                log::trace!("Recording instanced draw for batch {} with {} instances", batch_idx, instance_count);
                
                // One draw call for all instances in this batch
                device.cmd_draw_indexed(
                    command_buffer,
                    shared_resources.index_count,
                    instance_count,           // TRUE INSTANCING: All instances in one call
                    0,                        // first index
                    0,                        // vertex offset  
                    current_instance_offset,  // first instance offset in buffer
                );
                
                log::trace!("Drew {} instances successfully", instance_count);
                
                self.stats.draw_calls += 1;
                total_objects += instance_count;
                current_instance_offset += instance_count;
            }
            
            log::trace!("Completed {} individual draw calls for {} objects total", self.stats.draw_calls, total_objects);
        }
        
        log::trace!("Recorded {} individual draw calls", self.stats.draw_calls);
        Ok(())
    }
    
    /// Record instanced draw with explicit pipeline for specific instance indices
    /// 
    /// This version allows drawing only a subset of uploaded instances by their indices.
    /// Used for modular pipeline rendering where all objects are uploaded once, then
    /// drawn in multiple passes with different pipelines.
    pub fn record_instanced_draw_with_explicit_pipeline_and_range(
        &mut self,
        shared_resources: &SharedRenderingResources,
        context: &VulkanContext,
        frame_descriptor_set: vk::DescriptorSet,
        material_descriptor_set: vk::DescriptorSet,
        pipeline: vk::Pipeline,
        pipeline_layout: vk::PipelineLayout,
        instance_indices: &[u32],
    ) -> VulkanResult<()> {
        let command_buffer = self.active_command_buffer
            .ok_or_else(|| VulkanError::InvalidOperation {
                reason: "No active command buffer".to_string()
            })?;
        
        if instance_indices.is_empty() {
            log::trace!("No instances to render for this pipeline");
            return Ok(());
        }
        
        let device = context.raw_device();
        
        // Validate shared resources
        if shared_resources.index_count == 0 {
            return Err(VulkanError::InvalidOperation {
                reason: "No indices to render".to_string()
            });
        }
        
        log::trace!("Recording draw for {} instances with explicit pipeline...", instance_indices.len());
        
        // Validate critical handles
        let vertex_buffer_handle = shared_resources.vertex_buffer.handle();
        
        if vertex_buffer_handle == vk::Buffer::null() {
            return Err(VulkanError::InvalidOperation {
                reason: "Vertex buffer handle is null".to_string()
            });
        }
        
        if command_buffer == vk::CommandBuffer::null() {
            return Err(VulkanError::InvalidOperation {
                reason: "Command buffer is null".to_string()
            });
        }
        
        if pipeline == vk::Pipeline::null() {
            return Err(VulkanError::InvalidOperation {
                reason: "Pipeline handle is null".to_string()
            });
        }
        
        unsafe {
            // Bind the explicit pipeline
            device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline,
            );
            
            // Bind descriptor sets
            device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline_layout,
                0,
                &[frame_descriptor_set, material_descriptor_set],
                &[],
            );
            
            // Set viewport and scissor
            let extent = context.swapchain().extent();
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
            
            device.cmd_set_viewport(command_buffer, 0, &[viewport]);
            device.cmd_set_scissor(command_buffer, 0, &[scissor]);
            
            // Bind vertex and instance buffers
            let instance_size = std::mem::size_of::<InstanceData>();
            let frame_offset = (self.current_frame * self.max_instances * instance_size) as vk::DeviceSize;
            let vertex_buffers = [vertex_buffer_handle, self.instance_buffer.handle()];
            let offsets = [0, frame_offset];
            device.cmd_bind_vertex_buffers(command_buffer, 0, &vertex_buffers, &offsets);
            
            // Bind index buffer
            device.cmd_bind_index_buffer(
                command_buffer,
                shared_resources.index_buffer.handle(),
                0,
                vk::IndexType::UINT32,
            );
            
            // Draw the specified instances
            // Since instances may not be contiguous in the buffer (different pipelines interleaved),
            // we need to draw them individually or in contiguous chunks
            // For simplicity, draw each instance separately for now
            for &instance_index in instance_indices {
                log::trace!("Drawing instance at index {}", instance_index);
                
                device.cmd_draw_indexed(
                    command_buffer,
                    shared_resources.index_count,
                    1,  // instance count
                    0,  // first index
                    0,  // vertex offset
                    instance_index,  // first instance
                );
                
                self.stats.draw_calls += 1;
            }
        }
        
        Ok(())
    }
    
    /// Record multiple individual draw calls with an explicit pipeline (MODULAR PIPELINE VERSION)
    /// 
    /// This version accepts explicit pipeline and pipeline_layout parameters instead of
    /// reading them from SharedRenderingResources. This allows switching pipelines based
    /// on material requirements.
    pub fn record_instanced_draw_with_explicit_pipeline(
        &mut self,
        shared_resources: &SharedRenderingResources,
        context: &VulkanContext,
        frame_descriptor_set: vk::DescriptorSet,
        material_descriptor_set: vk::DescriptorSet,
        pipeline: vk::Pipeline,
        pipeline_layout: vk::PipelineLayout,
    ) -> VulkanResult<()> {
        let command_buffer = self.active_command_buffer
            .ok_or_else(|| VulkanError::InvalidOperation {
                reason: "No active command buffer".to_string()
            })?;
        
        if self.batches.is_empty() {
            log::trace!("No batches to render");
            return Ok(());
        }
        
        let device = context.raw_device();
        
        // Validate shared resources
        if shared_resources.index_count == 0 {
            return Err(VulkanError::InvalidOperation {
                reason: "No indices to render".to_string()
            });
        }
        
        log::trace!("Recording draw calls with explicit pipeline...");
        
        // Validate critical handles before unsafe operations
        let vertex_buffer_handle = shared_resources.vertex_buffer.handle();
        
        if vertex_buffer_handle == vk::Buffer::null() {
            return Err(VulkanError::InvalidOperation {
                reason: "Vertex buffer handle is null".to_string()
            });
        }
        
        if command_buffer == vk::CommandBuffer::null() {
            return Err(VulkanError::InvalidOperation {
                reason: "Command buffer is null".to_string()
            });
        }
        
        if pipeline == vk::Pipeline::null() {
            return Err(VulkanError::InvalidOperation {
                reason: "Pipeline handle is null".to_string()
            });
        }
        
        unsafe {
            // Bind the explicit pipeline
            device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline,
            );
            
            // Bind descriptor sets using explicit pipeline layout
            device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline_layout,
                0,
                &[frame_descriptor_set, material_descriptor_set],
                &[],
            );
            
            // Set dynamic viewport
            let extent = context.swapchain().extent();
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
            
            device.cmd_set_viewport(command_buffer, 0, &[viewport]);
            device.cmd_set_scissor(command_buffer, 0, &[scissor]);
            
            // Bind BOTH vertex buffer (binding 0) and instance buffer (binding 1)
            let instance_size = std::mem::size_of::<InstanceData>();
            let frame_offset = (self.current_frame * self.max_instances * instance_size) as vk::DeviceSize;
            let vertex_buffers = [vertex_buffer_handle, self.instance_buffer.handle()];
            let offsets = [0, frame_offset];
            device.cmd_bind_vertex_buffers(command_buffer, 0, &vertex_buffers, &offsets);
            
            // Bind index buffer
            device.cmd_bind_index_buffer(
                command_buffer,
                shared_resources.index_buffer.handle(),
                0,
                vk::IndexType::UINT32,
            );
            
            // Draw all batches
            let mut current_instance_offset = 0;
            let mut total_objects = 0;
            
            for (batch_idx, batch) in self.batches.iter().enumerate() {
                let instance_count = batch.instance_count() as u32;
                
                if instance_count == 0 {
                    continue;
                }
                
                log::trace!("Recording instanced draw for batch {} with {} instances", batch_idx, instance_count);
                
                device.cmd_draw_indexed(
                    command_buffer,
                    shared_resources.index_count,
                    instance_count,
                    0,
                    0,
                    current_instance_offset,
                );
                
                self.stats.draw_calls += 1;
                total_objects += instance_count;
                current_instance_offset += instance_count;
            }
            
            log::trace!("Completed {} draw calls for {} objects with explicit pipeline", self.stats.draw_calls, total_objects);
        }
        
        Ok(())
    }
    
    /// End frame rendering
    pub fn end_frame(&mut self) -> VulkanResult<()> {
        self.active_command_buffer = None;
        
        log::trace!("InstanceRenderer ended frame {} - {} instances, {} draws, {} batches",
                   self.current_frame,
                   self.stats.instances_rendered,
                   self.stats.draw_calls,
                   self.stats.batches_processed);
        
        // Update frame counter for next frame (assuming max 2 frames in flight)
        self.current_frame = (self.current_frame + 1) % 2;
        
        Ok(())
    }
    
    /// Get rendering statistics
    pub fn get_stats(&self) -> &RenderStats {
        &self.stats
    }
    
    /// Get maximum supported instances
    pub fn max_instances(&self) -> usize {
        self.max_instances
    }
    
    /// Initialize Vulkan resources
    pub fn initialize_resources(&mut self, _context: &VulkanContext) -> VulkanResult<()> {
        // Resources already initialized in constructor
        log::debug!("InstanceRenderer resources initialized");
        Ok(())
    }
    
    /// Cleanup Vulkan resources
    pub fn cleanup(&mut self) {
        log::debug!("Cleaning up InstanceRenderer resources");
        
        // Unmap buffer memory
        if !self.instance_buffer_mapped.is_null() {
            // Buffer cleanup will handle unmapping
            self.instance_buffer_mapped = std::ptr::null_mut();
        }
        
        // Buffer cleanup happens automatically when dropped
    }
}

impl Drop for InstanceRenderer {
    fn drop(&mut self) {
        self.cleanup();
    }
}

/// Error types specific to instance rendering
#[derive(Debug)]
pub enum InstanceRenderError {
    /// Buffer overflow - too many instances
    BufferOverflow,
    /// Invalid batch configuration
    InvalidBatch(String),
    /// GPU upload failed
    UploadFailed(String),
    /// Command recording failed
    CommandRecordingFailed(String),
}

impl std::fmt::Display for InstanceRenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BufferOverflow => write!(f, "Instance buffer overflow"),
            Self::InvalidBatch(msg) => write!(f, "Invalid batch configuration: {}", msg),
            Self::UploadFailed(msg) => write!(f, "GPU upload failed: {}", msg),
            Self::CommandRecordingFailed(msg) => write!(f, "Command recording failed: {}", msg),
        }
    }
}

impl std::error::Error for InstanceRenderError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::math::Vec3;
    
    #[test]
    fn test_instance_data_creation() {
        use crate::render::resources::materials::{Material, UnlitMaterialParams};
        use std::time::Instant;
        
        // Build transform matrix
        let transform = Mat4::new_translation(&Vec3::new(1.0, 2.0, 3.0))
            * Mat4::from_euler_angles(0.1, 0.2, 0.3)
            * Mat4::new_nonuniform_scaling(&Vec3::new(1.0, 1.0, 1.0));
        
        let render_data = DynamicRenderData {
            transform,
            material: Material::unlit(UnlitMaterialParams {
                color: Vec3::new(1.0, 1.0, 1.0),
                alpha: 1.0,
            }),
            spawn_time: Instant::now(),
            generation: 0,
            state: crate::render::systems::dynamic::object_manager::ResourceState::Active,
        };
        
        let instance_data = InstanceData::from_render_data(&render_data);
        
        // Check that position is correctly encoded in model matrix
        assert_eq!(instance_data.model_matrix[3][0], 1.0); // x translation
        assert_eq!(instance_data.model_matrix[3][1], 2.0); // y translation
        assert_eq!(instance_data.model_matrix[3][2], 3.0); // z translation
        assert_eq!(instance_data.model_matrix[3][3], 1.0); // w component
        
        // Check material defaults
        assert_eq!(instance_data.material_color, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(instance_data.material_index, 0);
        assert_eq!(instance_data.emission_color, [0.0, 0.0, 0.0, 0.0]);
        assert_eq!(instance_data.texture_enable_flags, [0, 0, 0, 0]);
    }
    
    #[test]
    fn test_instance_batch_management() {
        let mut batch = InstanceBatch::new(0, 0);
        assert_eq!(batch.instance_count(), 0);
        assert!(!batch.is_full());
        
        // Fill batch to capacity
        for i in 0..MAX_INSTANCES_PER_DRAW {
            let instance_data = InstanceData {
                model_matrix: [[0.0; 4]; 4],
                normal_matrix: [[0.0; 4]; 4],
                material_color: [1.0, 1.0, 1.0, 1.0],
                emission_color: [0.0, 0.0, 0.0, 0.0],
                texture_enable_flags: [0, 0, 0, 0],
                material_index: i as u32,
                _padding: [0; 3],
            };
            
            assert!(batch.add_instance(instance_data).is_ok());
        }
        
        assert!(batch.is_full());
        assert_eq!(batch.instance_count(), MAX_INSTANCES_PER_DRAW as u32);
        
        // Adding one more should fail
        let extra_instance = InstanceData {
            model_matrix: [[0.0; 4]; 4],
            normal_matrix: [[0.0; 4]; 4],
            material_color: [1.0, 1.0, 1.0, 1.0],
            emission_color: [0.0, 0.0, 0.0, 0.0],
            texture_enable_flags: [0, 0, 0, 0],
            material_index: 999,
            _padding: [0; 3],
        };
        
        assert!(batch.add_instance(extra_instance).is_err());
    }
    
    #[test]
    fn test_normal_matrix_calculation() {
        // Test identity matrix
        let identity = Mat4::identity();
        let normal_matrix = InstanceData::calculate_normal_matrix(&identity);
        
        // Identity matrix should produce identity normal matrix
        assert_eq!(normal_matrix[0][0], 1.0);
        assert_eq!(normal_matrix[1][1], 1.0);
        assert_eq!(normal_matrix[2][2], 1.0);
        assert_eq!(normal_matrix[3][3], 1.0);
        
        // Off-diagonal elements should be zero
        assert_eq!(normal_matrix[0][1], 0.0);
        assert_eq!(normal_matrix[1][0], 0.0);
    }
}
