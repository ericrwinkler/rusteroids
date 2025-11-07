//! Material Uniform Buffer Objects for GPU data transfer
//!
//! This module defines the GPU-side material data structures that are uploaded
//! to uniform buffers for shader access.

/// Standard PBR material uniform data for GPU
/// 
/// This structure is uploaded to Set 1, Binding 0 for PBR materials.
/// Layout must match the corresponding GLSL uniform block.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy)]
pub struct StandardMaterialUBO {
    /// Base color (albedo) - RGB + alpha
    pub base_color: [f32; 4],
    /// Metallic, roughness, ambient occlusion, normal_scale
    pub metallic_roughness_ao_normal: [f32; 4],
    /// Emission color - RGB + emission strength
    pub emission: [f32; 4],
    /// Texture usage flags - base_color, normal, metallic_roughness, ao
    pub texture_flags: [u32; 4],
    /// Additional parameters (reserved for future use)
    pub additional_params: [f32; 4],
    /// Padding to ensure 16-byte alignment
    pub _padding: [f32; 4],
}

impl StandardMaterialUBO {
    /// Create from standard material parameters
    pub fn from_params(params: &super::StandardMaterialParams) -> Self {
        Self {
            base_color: [params.base_color.x, params.base_color.y, params.base_color.z, params.alpha],
            metallic_roughness_ao_normal: [
                params.metallic,
                params.roughness,
                params.ambient_occlusion,
                params.normal_scale,
            ],
            emission: [
                params.emission.x,
                params.emission.y,
                params.emission.z,
                params.emission_strength,
            ],
            texture_flags: [0, 0, 0, 0], // Will be set based on bound textures
            additional_params: [0.0; 4],
            _padding: [0.0; 4],
        }
    }

    /// Set texture usage flags
    pub fn with_texture_flags(mut self, base_color: bool, normal: bool, metallic_roughness: bool, ao: bool) -> Self {
        self.texture_flags[0] = if base_color { 1 } else { 0 };
        self.texture_flags[1] = if normal { 1 } else { 0 };
        self.texture_flags[2] = if metallic_roughness { 1 } else { 0 };
        self.texture_flags[3] = if ao { 1 } else { 0 };
        self
    }
    
    /// Set emission texture flag (additional_params.x)
    pub fn with_emission_texture(mut self, enabled: bool) -> Self {
        self.additional_params[0] = if enabled { 1.0 } else { 0.0 };
        self
    }
    
    /// Set opacity texture flag (additional_params.y)
    pub fn with_opacity_texture(mut self, enabled: bool) -> Self {
        self.additional_params[1] = if enabled { 1.0 } else { 0.0 };
        self
    }
}

/// Unlit material uniform data for GPU
/// 
/// This structure is uploaded to Set 1, Binding 0 for unlit materials.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy)]
pub struct UnlitMaterialUBO {
    /// Material color - RGB + alpha
    pub color: [f32; 4],
    /// Texture usage flags - base_color, unused, unused, unused
    pub texture_flags: [u32; 4],
    /// Additional parameters (reserved for future use)
    pub additional_params: [f32; 4],
    /// Padding to ensure proper size
    pub _padding: [f32; 4],
}

impl UnlitMaterialUBO {
    /// Create from unlit material parameters
    pub fn from_params(params: &super::UnlitMaterialParams) -> Self {
        Self {
            color: [params.color.x, params.color.y, params.color.z, params.alpha],
            texture_flags: [0, 0, 0, 0],
            additional_params: [0.0; 4],
            _padding: [0.0; 4],
        }
    }

    /// Set texture usage flags
    pub fn with_texture_flags(mut self, base_color: bool) -> Self {
        self.texture_flags[0] = if base_color { 1 } else { 0 };
        self
    }
    
    /// Set opacity texture flag (additional_params.y) - for consistency with StandardMaterialUBO
    pub fn with_opacity_texture(mut self, enabled: bool) -> Self {
        self.additional_params[1] = if enabled { 1.0 } else { 0.0 };
        self
    }
}

/// Material UBO that can hold any material type
/// 
/// This enum allows for type-safe material UBO handling while maintaining
/// a unified interface for GPU uploads.
#[derive(Debug, Clone, Copy)]
pub enum MaterialUBO {
    /// Standard PBR material uniform data
    StandardPBR(StandardMaterialUBO),
    /// Unlit material uniform data
    Unlit(UnlitMaterialUBO),
}

impl MaterialUBO {
    /// Get the size of this UBO in bytes
    pub fn size(&self) -> usize {
        match self {
            MaterialUBO::StandardPBR(_) => std::mem::size_of::<StandardMaterialUBO>(),
            MaterialUBO::Unlit(_) => std::mem::size_of::<UnlitMaterialUBO>(),
        }
    }

    /// Get a byte slice of this UBO for GPU upload
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            MaterialUBO::StandardPBR(ubo) => bytemuck::bytes_of(ubo),
            MaterialUBO::Unlit(ubo) => bytemuck::bytes_of(ubo),
        }
    }
}

// Safety: These UBOs only contain f32 and u32 values with proper alignment
unsafe impl bytemuck::Pod for StandardMaterialUBO {}
unsafe impl bytemuck::Zeroable for StandardMaterialUBO {}

unsafe impl bytemuck::Pod for UnlitMaterialUBO {}
unsafe impl bytemuck::Zeroable for UnlitMaterialUBO {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_material_ubo_sizes() {
        // Ensure UBOs are properly sized and aligned
        assert_eq!(std::mem::size_of::<StandardMaterialUBO>(), 96); // 6 * 16 bytes
        assert_eq!(std::mem::size_of::<UnlitMaterialUBO>(), 64);    // 4 * 16 bytes
        
        // Ensure proper alignment
        assert_eq!(std::mem::align_of::<StandardMaterialUBO>(), 16);
        assert_eq!(std::mem::align_of::<UnlitMaterialUBO>(), 16);
    }

    #[test]
    fn test_standard_material_conversion() {
        let params = super::super::StandardMaterialParams {
            base_color: crate::foundation::math::Vec3::new(0.8, 0.6, 0.4),
            alpha: 0.9,
            metallic: 0.1,
            roughness: 0.7,
            ..Default::default()
        };

        let ubo = StandardMaterialUBO::from_params(&params);
        
        assert_eq!(ubo.base_color, [0.8, 0.6, 0.4, 0.9]);
        assert_eq!(ubo.metallic_roughness_ao_normal[0], 0.1); // metallic
        assert_eq!(ubo.metallic_roughness_ao_normal[1], 0.7); // roughness
    }
}
