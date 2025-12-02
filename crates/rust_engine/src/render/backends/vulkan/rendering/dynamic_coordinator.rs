//! Dynamic rendering orchestration for instanced rendering
//! 
//! Coordinates batch rendering of dynamic objects using Vulkan instanced rendering.
//! Handles pipeline grouping, depth sorting, and efficient GPU upload.

use crate::render::backends::vulkan::*;
use crate::foundation::math::Vec3;
use ash::vk;
use std::collections::HashMap;

/// Coordinates dynamic object rendering with pipeline grouping and depth sorting
pub struct DynamicCoordinator;

impl DynamicCoordinator {
    /// Record dynamic draws using a dedicated InstanceRenderer (new architecture)
    ///
    /// This method provides the Vulkan-specific parameters needed for dedicated InstanceRenderer
    /// per mesh pool, preventing state corruption between different mesh types.
    /// 
    /// MODULAR PIPELINE VERSION: Groups objects by required pipeline type and renders each group separately
    pub fn record_dynamic_draws_with_dedicated_renderer(
        context: &VulkanContext,
        pipeline_manager: &crate::render::PipelineManager,
        instance_renderer: &mut crate::render::systems::dynamic::InstanceRenderer,
        dynamic_objects: &HashMap<crate::render::systems::dynamic::DynamicObjectHandle, crate::render::systems::dynamic::DynamicRenderData>,
        shared_resources: &crate::render::SharedRenderingResources,
        frame_descriptor_set: vk::DescriptorSet,
        pool_material_descriptor_set: vk::DescriptorSet,
        camera_position: Vec3,
        camera_target: Vec3,
    ) -> crate::render::BackendResult<()> {
        use crate::render::resources::materials::PipelineType;
        
        // Group objects by their required pipeline type
        let mut pipeline_groups: HashMap<PipelineType, Vec<(crate::render::systems::dynamic::DynamicObjectHandle, &crate::render::systems::dynamic::DynamicRenderData)>> = HashMap::new();
        
        for (handle, render_data) in dynamic_objects.iter() {
            let pipeline_type = render_data.material.required_pipeline();
            pipeline_groups.entry(pipeline_type)
                .or_insert_with(Vec::new)
                .push((*handle, render_data));
        }
        
        log::debug!("Rendering {} objects grouped into {} pipeline types", dynamic_objects.len(), pipeline_groups.len());
        
        // CRITICAL FIX: Upload ALL objects ONCE to contiguous buffer regions
        // Then draw each pipeline group from its specific region
        instance_renderer.upload_instance_data(dynamic_objects)
            .map_err(|e| crate::render::RenderError::BackendError(e.to_string()))?;
        
        // Track which objects are at which indices in the uploaded buffer
        let mut object_indices: HashMap<crate::render::systems::dynamic::DynamicObjectHandle, u32> = HashMap::new();
        let mut current_index = 0u32;
        for (handle, _) in dynamic_objects.iter() {
            object_indices.insert(*handle, current_index);
            current_index += 1;
        }
        
        // CRITICAL: Render in correct order for transparency
        // 1. Render opaque objects front-to-back (performance - early Z rejection)
        // 2. Then render transparent objects back-to-front (correctness - proper alpha blending)
        
        let opaque_pipeline_types = [PipelineType::StandardPBR, PipelineType::Unlit];
        let transparent_pipeline_types = [PipelineType::TransparentPBR, PipelineType::TransparentUnlit];
        
        // Calculate camera forward direction for view-space Z sorting (industry standard)
        let camera_forward = (camera_target - camera_position).normalize();
        
        // Render opaque objects front-to-back
        for pipeline_type in &opaque_pipeline_types {
            if let Some(objects) = pipeline_groups.get(pipeline_type) {
                // Sort front-to-back using view-space Z (dot product with camera forward)
                let mut sorted_objects = objects.clone();
                sorted_objects.sort_by(|a, b| {
                    // Extract positions from transform matrices
                    let pos_a = a.1.transform.column(3).xyz();
                    let pos_b = b.1.transform.column(3).xyz();
                    let to_object_a = pos_a - camera_position;
                    let to_object_b = pos_b - camera_position;
                    
                    let depth_a = to_object_a.dot(&camera_forward);
                    let depth_b = to_object_b.dot(&camera_forward);
                    
                    depth_a.partial_cmp(&depth_b).unwrap_or(std::cmp::Ordering::Equal)
                });
                
                Self::render_pipeline_group(
                    context,
                    pipeline_manager,
                    instance_renderer,
                    shared_resources,
                    frame_descriptor_set,
                    pool_material_descriptor_set,
                    *pipeline_type,
                    &sorted_objects,
                    &object_indices
                )?;
            }
        }
        
        // Render transparent objects back-to-front
        // CRITICAL: Merge all transparent pipeline types and sort together for correct depth ordering
        let mut all_transparent_objects = Vec::new();
        for pipeline_type in &transparent_pipeline_types {
            if let Some(objects) = pipeline_groups.get(pipeline_type) {
                for obj in objects {
                    all_transparent_objects.push((*pipeline_type, *obj));
                }
            }
        }
        
        // Sort ALL transparent objects back-to-front using view-space Z
        // This ensures correct alpha blending across different material types
        all_transparent_objects.sort_by(|a, b| {
            // Extract positions from transform matrices
            let pos_a = a.1.1.transform.column(3).xyz();
            let pos_b = b.1.1.transform.column(3).xyz();
            let to_object_a = pos_a - camera_position;
            let to_object_b = pos_b - camera_position;
            
            let depth_a = to_object_a.dot(&camera_forward);
            let depth_b = to_object_b.dot(&camera_forward);
            
            depth_b.partial_cmp(&depth_a).unwrap_or(std::cmp::Ordering::Equal) // Reversed for back-to-front
        });
        
        log::debug!("Sorted {} transparent objects for rendering", all_transparent_objects.len());
        
        // Render sorted transparent objects (may require pipeline switches)
        let mut current_pipeline: Option<PipelineType> = None;
        let mut batch_objects = Vec::new();
        
        for (pipeline_type, object) in all_transparent_objects {
            if current_pipeline != Some(pipeline_type) {
                // Pipeline change - render previous batch
                if let Some(prev_pipeline) = current_pipeline {
                    Self::render_pipeline_group(
                        context,
                        pipeline_manager,
                        instance_renderer,
                        shared_resources,
                        frame_descriptor_set,
                        pool_material_descriptor_set,
                        prev_pipeline,
                        &batch_objects,
                        &object_indices
                    )?;
                    batch_objects.clear();
                }
                current_pipeline = Some(pipeline_type);
            }
            batch_objects.push(object);
        }
        
        // Render final batch
        if let Some(pipeline_type) = current_pipeline {
            Self::render_pipeline_group(
                context,
                pipeline_manager,
                instance_renderer,
                shared_resources,
                frame_descriptor_set,
                pool_material_descriptor_set,
                pipeline_type,
                &batch_objects,
                &object_indices
            )?;
        }
        
        log::trace!("Recorded dynamic draws for {} total objects across {} pipelines", dynamic_objects.len(), pipeline_groups.len());
        Ok(())
    }
    
    /// Upload instance data for a pool without rendering (Phase 1 of split architecture)
    ///
    /// This uploads ALL objects to the GPU buffer and returns the index mapping.
    /// The indices can then be used for multiple draw calls without re-uploading.
    pub fn upload_pool_instance_data(
        instance_renderer: &mut crate::render::systems::dynamic::InstanceRenderer,
        dynamic_objects: &HashMap<crate::render::systems::dynamic::DynamicObjectHandle, crate::render::systems::dynamic::DynamicRenderData>,
    ) -> crate::render::BackendResult<HashMap<crate::render::systems::dynamic::DynamicObjectHandle, u32>> {
        // Upload ALL objects ONCE
        instance_renderer.upload_instance_data(dynamic_objects)
            .map_err(|e| crate::render::RenderError::BackendError(e.to_string()))?;
        
        // Build and return index map
        let mut object_indices = HashMap::new();
        let mut current_index = 0u32;
        for (handle, _) in dynamic_objects.iter() {
            object_indices.insert(*handle, current_index);
            current_index += 1;
        }
        
        Ok(object_indices)
    }
    
    /// Render a subset of already-uploaded objects (Phase 2 of split architecture)
    ///
    /// This renders specific objects using indices from upload_pool_instance_data().
    /// Does NOT upload data - assumes upload_pool_instance_data() was already called.
    pub fn render_uploaded_objects_subset(
        context: &VulkanContext,
        pipeline_manager: &crate::render::PipelineManager,
        instance_renderer: &mut crate::render::systems::dynamic::InstanceRenderer,
        shared_resources: &crate::render::SharedRenderingResources,
        frame_descriptor_set: vk::DescriptorSet,
        material_descriptor_set: vk::DescriptorSet,
        objects: &[(crate::render::systems::dynamic::DynamicObjectHandle, crate::render::systems::dynamic::DynamicRenderData)],
        object_indices: &HashMap<crate::render::systems::dynamic::DynamicObjectHandle, u32>,
    ) -> crate::render::BackendResult<()> {
        use crate::render::resources::materials::PipelineType;
        
        // Group objects by pipeline type
        let mut pipeline_groups: HashMap<PipelineType, Vec<(crate::render::systems::dynamic::DynamicObjectHandle, &crate::render::systems::dynamic::DynamicRenderData)>> = HashMap::new();
        
        for (handle, render_data) in objects.iter() {
            let pipeline_type = render_data.material.required_pipeline();
            pipeline_groups.entry(pipeline_type)
                .or_insert_with(Vec::new)
                .push((*handle, render_data));
        }
        
        // Render each pipeline group
        for (pipeline_type, group_objects) in pipeline_groups {
            Self::render_pipeline_group(
                context,
                pipeline_manager,
                instance_renderer,
                shared_resources,
                frame_descriptor_set,
                material_descriptor_set,
                pipeline_type,
                &group_objects,
                object_indices
            )?;
        }
        
        Ok(())
    }
    
    /// Helper method to render a single pipeline group
    fn render_pipeline_group(
        context: &VulkanContext,
        pipeline_manager: &crate::render::PipelineManager,
        instance_renderer: &mut crate::render::systems::dynamic::InstanceRenderer,
        shared_resources: &crate::render::SharedRenderingResources,
        frame_descriptor_set: vk::DescriptorSet,
        material_descriptor_set: vk::DescriptorSet,
        pipeline_type: crate::render::resources::materials::PipelineType,
        objects: &[(crate::render::systems::dynamic::DynamicObjectHandle, &crate::render::systems::dynamic::DynamicRenderData)],
        object_indices: &HashMap<crate::render::systems::dynamic::DynamicObjectHandle, u32>
    ) -> crate::render::BackendResult<()> {
        // Get the pipeline for this material type
        let pipeline = pipeline_manager.get_pipeline_by_type(pipeline_type)
            .ok_or_else(|| crate::render::RenderError::BackendError(
                format!("No pipeline found for type: {:?}", pipeline_type)
            ))?;
        
        log::trace!("Rendering {} objects with pipeline {:?}", objects.len(), pipeline_type);
        
        // Collect instance indices for this pipeline group
        let instance_indices: Vec<u32> = objects.iter()
            .filter_map(|(handle, _)| object_indices.get(handle).copied())
            .collect();
        
        // Record draw commands with the specific pipeline and instance range
        instance_renderer.record_instanced_draw_with_explicit_pipeline_and_range(
            shared_resources,
            context,
            frame_descriptor_set,
            material_descriptor_set,
            pipeline.handle(),
            pipeline.layout(),
            &instance_indices
        ).map_err(|e| crate::render::RenderError::BackendError(e.to_string()))?;
        
        Ok(())
    }
}
