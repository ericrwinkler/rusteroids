// src/render/material.rs
use crate::render::texture::Texture;
use crate::util::math::Vec3;
use ash::vk;

#[derive(Debug, Clone)]
pub struct Material {
    pub albedo: Vec3,           // Base color (RGB)
    pub metallic: f32,          // Metallic factor (0.0 = dielectric, 1.0 = metallic)
    pub roughness: f32,         // Surface roughness (0.0 = mirror, 1.0 = rough)
    pub alpha: f32,             // Transparency (1.0 = opaque, 0.0 = transparent)
    pub albedo_texture: Option<usize>,    // Texture ID for albedo map
    pub normal_texture: Option<usize>,    // Texture ID for normal map
    pub metallic_roughness_texture: Option<usize>, // Texture ID for metallic/roughness map
}

impl Material {
    pub fn new() -> Self {
        Self {
            albedo: Vec3::new(1.0, 1.0, 1.0), // White
            metallic: 0.0,
            roughness: 0.5,
            alpha: 1.0,
            albedo_texture: None,
            normal_texture: None,
            metallic_roughness_texture: None,
        }
    }

    pub fn with_albedo(mut self, albedo: Vec3) -> Self {
        self.albedo = albedo;
        self
    }

    pub fn with_color(mut self, r: f32, g: f32, b: f32) -> Self {
        self.albedo = Vec3::new(r, g, b);
        self
    }

    pub fn with_metallic(mut self, metallic: f32) -> Self {
        self.metallic = metallic.clamp(0.0, 1.0);
        self
    }

    pub fn with_roughness(mut self, roughness: f32) -> Self {
        self.roughness = roughness.clamp(0.0, 1.0);
        self
    }

    pub fn with_alpha(mut self, alpha: f32) -> Self {
        self.alpha = alpha.clamp(0.0, 1.0);
        self
    }

    pub fn with_albedo_texture(mut self, texture_id: usize) -> Self {
        self.albedo_texture = Some(texture_id);
        self
    }

    pub fn with_normal_texture(mut self, texture_id: usize) -> Self {
        self.normal_texture = Some(texture_id);
        self
    }

    pub fn with_metallic_roughness_texture(mut self, texture_id: usize) -> Self {
        self.metallic_roughness_texture = Some(texture_id);
        self
    }

    pub fn is_transparent(&self) -> bool {
        self.alpha < 1.0
    }

    // Predefined materials
    pub fn plastic_red() -> Self {
        Self::new()
            .with_color(0.8, 0.2, 0.2)
            .with_metallic(0.0)
            .with_roughness(0.3)
    }

    pub fn metal_gold() -> Self {
        Self::new()
            .with_color(1.0, 0.8, 0.2)
            .with_metallic(1.0)
            .with_roughness(0.1)
    }

    pub fn metal_steel() -> Self {
        Self::new()
            .with_color(0.7, 0.7, 0.7)
            .with_metallic(1.0)
            .with_roughness(0.2)
    }

    pub fn glass() -> Self {
        Self::new()
            .with_color(1.0, 1.0, 1.0)
            .with_metallic(0.0)
            .with_roughness(0.0)
            .with_alpha(0.1)
    }
}

impl Default for Material {
    fn default() -> Self {
        Self::new()
    }
}

// Material uniform buffer structure for shaders
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MaterialUniforms {
    pub albedo: [f32; 3],
    pub metallic: f32,
    pub roughness: f32,
    pub alpha: f32,
    pub has_albedo_texture: u32,
    pub has_normal_texture: u32,
}

impl From<&Material> for MaterialUniforms {
    fn from(material: &Material) -> Self {
        Self {
            albedo: [material.albedo.x, material.albedo.y, material.albedo.z],
            metallic: material.metallic,
            roughness: material.roughness,
            alpha: material.alpha,
            has_albedo_texture: if material.albedo_texture.is_some() { 1 } else { 0 },
            has_normal_texture: if material.normal_texture.is_some() { 1 } else { 0 },
        }
    }
}
