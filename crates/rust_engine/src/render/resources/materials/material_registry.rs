//! Material Manager for efficient material resource management
//!
//! This module provides a centralized system for managing materials,
//! their GPU resources, and descriptor set bindings.

use std::collections::HashMap;
use crate::render::backends::vulkan::{Buffer, VulkanError, VulkanResult};
use super::{Material, MaterialId, MaterialType, MaterialUBO, StandardMaterialUBO, UnlitMaterialUBO};

/// Central manager for material resources
pub struct MaterialManager {
    /// All registered materials
    materials: HashMap<MaterialId, Material>,
    /// Material UBO buffers (one per material)
    material_ubo_buffers: HashMap<MaterialId, Buffer>,
    /// Next available material ID
    next_id: u32,
    /// Maximum number of materials that can be loaded
    max_materials: usize,
}

impl MaterialManager {
    /// Create a new material manager
    pub fn new(max_materials: usize) -> Self {
        Self {
            materials: HashMap::with_capacity(max_materials),
            material_ubo_buffers: HashMap::with_capacity(max_materials),
            next_id: 1, // Start from 1, reserve 0 for "no material"
            max_materials,
        }
    }

    /// Register a new material and create its GPU resources
    pub fn register_material(
        &mut self,
        mut material: Material,
        context: &crate::render::backends::vulkan::VulkanContext,
    ) -> VulkanResult<MaterialId> {
        if self.materials.len() >= self.max_materials {
            return Err(VulkanError::InitializationFailed(
                "Maximum number of materials reached".to_string(),
            ));
        }

        // Assign unique ID
        let id = MaterialId(self.next_id);
        self.next_id += 1;
        material.id = id;

        // Create UBO for this material
        let material_ubo = self.create_material_ubo(&material)?;
        let ubo_buffer = self.create_ubo_buffer(context, &material_ubo)?;

        // Store material and its resources
        self.materials.insert(id, material);
        self.material_ubo_buffers.insert(id, ubo_buffer);

        log::debug!("Registered material {:?} with UBO size: {} bytes", id, material_ubo.size());

        Ok(id)
    }

    /// Get a material by ID
    pub fn get_material(&self, id: MaterialId) -> Option<&Material> {
        self.materials.get(&id)
    }

    /// Get the UBO buffer for a material
    pub fn get_material_ubo_buffer(&self, id: MaterialId) -> Option<&Buffer> {
        self.material_ubo_buffers.get(&id)
    }

    /// Update a material's parameters and refresh its UBO
    pub fn update_material(
        &mut self,
        id: MaterialId,
        material: Material,
    ) -> VulkanResult<()> {
        // Check if material exists and create UBO outside of the mutable borrow
        let material_ubo = if self.materials.contains_key(&id) {
            self.create_material_ubo(&material)?
        } else {
            return Err(VulkanError::InitializationFailed(
                format!("Material {:?} not found", id),
            ));
        };

        // Now update the material
        if let Some(existing_material) = self.materials.get_mut(&id) {
            *existing_material = material;
            
            // Update the UBO buffer
            if let Some(buffer) = self.material_ubo_buffers.get(&id) {
                buffer.write_data(material_ubo.as_bytes())?;
                log::debug!("Updated material {:?} UBO", id);
            }

            Ok(())
        } else {
            Err(VulkanError::InitializationFailed(
                format!("Material {:?} not found", id),
            ))
        }
    }

    /// Get all registered material IDs
    pub fn get_all_material_ids(&self) -> Vec<MaterialId> {
        self.materials.keys().copied().collect()
    }

    /// Get number of registered materials
    pub fn material_count(&self) -> usize {
        self.materials.len()
    }

    /// Create a default PBR material (useful for testing)
    pub fn create_default_pbr_material(
        &mut self,
        context: &crate::render::backends::vulkan::VulkanContext,
    ) -> VulkanResult<MaterialId> {
        let material = Material::standard_pbr(Default::default())
            .with_name("Default PBR Material");
        self.register_material(material, context)
    }

    /// Create a default unlit material
    pub fn create_default_unlit_material(
        &mut self,
        context: &crate::render::backends::vulkan::VulkanContext,
    ) -> VulkanResult<MaterialId> {
        let material = Material::unlit(Default::default())
            .with_name("Default Unlit Material");
        self.register_material(material, context)
    }

    /// Create a MaterialUBO from a Material
    fn create_material_ubo(&self, material: &Material) -> VulkanResult<MaterialUBO> {
        let ubo = match &material.material_type {
            MaterialType::StandardPBR(params) => {
                MaterialUBO::StandardPBR(StandardMaterialUBO::from_params(params))
            }
            MaterialType::Unlit(params) => {
                MaterialUBO::Unlit(UnlitMaterialUBO::from_params(params))
            }
            MaterialType::Transparent { base_material, .. } => {
                // For transparent materials, use the base material's UBO
                // Alpha blending is handled by the pipeline, not the UBO
                match &**base_material {
                    MaterialType::StandardPBR(params) => {
                        MaterialUBO::StandardPBR(StandardMaterialUBO::from_params(params))
                    }
                    MaterialType::Unlit(params) => {
                        MaterialUBO::Unlit(UnlitMaterialUBO::from_params(params))
                    }
                    MaterialType::Transparent { .. } => {
                        return Err(VulkanError::InitializationFailed(
                            "Nested transparent materials not supported".to_string(),
                        ));
                    }
                }
            }
        };

        Ok(ubo)
    }

    /// Create a UBO buffer for a material
    fn create_ubo_buffer(
        &self,
        context: &crate::render::backends::vulkan::VulkanContext,
        material_ubo: &MaterialUBO,
    ) -> VulkanResult<Buffer> {
        let buffer = Buffer::new(
            context.raw_device().clone(),
            context.instance().clone(),
            context.physical_device().device,
            material_ubo.size() as u64,
            ash::vk::BufferUsageFlags::UNIFORM_BUFFER,
            ash::vk::MemoryPropertyFlags::HOST_VISIBLE | ash::vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        // Upload initial data
        buffer.write_data(material_ubo.as_bytes())?;

        Ok(buffer)
    }
}

impl Drop for MaterialManager {
    fn drop(&mut self) {
        log::debug!("MaterialManager dropping with {} materials", self.materials.len());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_material_id_generation() {
        let mut manager = MaterialManager::new(10);
        
        // IDs should start from 1 and increment
        assert_eq!(manager.next_id, 1);
        
        // Mock material creation (without actual Vulkan context)
        manager.next_id = 5;
        assert_eq!(manager.next_id, 5);
    }

    #[test]
    fn test_material_storage() {
        let manager = MaterialManager::new(10);
        
        assert_eq!(manager.material_count(), 0);
        assert!(manager.get_all_material_ids().is_empty());
    }
}
