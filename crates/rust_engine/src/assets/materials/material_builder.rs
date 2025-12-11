//! Procedural material builder for runtime material creation
//!
//! Provides a builder pattern API for creating materials programmatically,
//! with presets for common game effects (shields, explosions, lasers, etc.).

use crate::foundation::math::Vec3;
use crate::render::resources::materials::{Material, StandardMaterialParams};

/// Builder for creating materials programmatically at runtime
///
/// # Examples
/// ```
/// use rust_engine::render::MaterialBuilder;
/// use rust_engine::foundation::math::Vec3;
///
/// // Create a glowing shield material
/// let shield = MaterialBuilder::new()
///     .base_color(Vec3::new(0.2, 0.6, 1.0))
///     .metallic(0.3)
///     .roughness(0.4)
///     .emission(Vec3::new(0.3, 0.7, 1.0))
///     .emission_strength(2.5)
///     .alpha(0.6)
///     .build();
/// ```
pub struct MaterialBuilder {
    params: StandardMaterialParams,
    name: Option<String>,
}

impl MaterialBuilder {
    /// Create a new material builder with default PBR parameters
    pub fn new() -> Self {
        Self {
            params: StandardMaterialParams::default(),
            name: None,
        }
    }

    /// Set the base color (albedo)
    pub fn base_color(mut self, color: Vec3) -> Self {
        self.params.base_color = color;
        self
    }

    /// Set the base color from RGB values (0-1 range)
    pub fn base_color_rgb(mut self, r: f32, g: f32, b: f32) -> Self {
        self.params.base_color = Vec3::new(r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0));
        self
    }

    /// Set the base color from hex color code
    pub fn base_color_hex(mut self, hex: u32) -> Self {
        let r = ((hex >> 16) & 0xFF) as f32 / 255.0;
        let g = ((hex >> 8) & 0xFF) as f32 / 255.0;
        let b = (hex & 0xFF) as f32 / 255.0;
        self.params.base_color = Vec3::new(r, g, b);
        self
    }

    /// Set metallic factor (0.0 = dielectric, 1.0 = metallic)
    pub fn metallic(mut self, metallic: f32) -> Self {
        self.params.metallic = metallic.clamp(0.0, 1.0);
        self
    }

    /// Set roughness factor (0.0 = mirror smooth, 1.0 = completely rough)
    pub fn roughness(mut self, roughness: f32) -> Self {
        self.params.roughness = roughness.clamp(0.0, 1.0);
        self
    }

    /// Set emission color for self-illuminated materials
    pub fn emission(mut self, color: Vec3) -> Self {
        self.params.emission = color;
        self
    }

    /// Set emission color from RGB values
    pub fn emission_rgb(mut self, r: f32, g: f32, b: f32) -> Self {
        self.params.emission = Vec3::new(r, g, b);
        self
    }

    /// Set emission strength multiplier
    pub fn emission_strength(mut self, strength: f32) -> Self {
        self.params.emission_strength = strength.max(0.0);
        self
    }

    /// Set alpha transparency (0.0 = fully transparent, 1.0 = opaque)
    pub fn alpha(mut self, alpha: f32) -> Self {
        self.params.alpha = alpha.clamp(0.0, 1.0);
        self
    }

    /// Set ambient occlusion factor
    pub fn ambient_occlusion(mut self, ao: f32) -> Self {
        self.params.ambient_occlusion = ao.clamp(0.0, 1.0);
        self
    }

    /// Set normal map scale
    pub fn normal_scale(mut self, scale: f32) -> Self {
        self.params.normal_scale = scale.max(0.0);
        self
    }

    /// Set material name for debugging
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Build the final material
    pub fn build(self) -> Material {
        let mut material = Material::standard_pbr(self.params);
        if let Some(name) = self.name {
            material = material.with_name(name);
        }
        material
    }

    /// Build as transparent material with alpha blending
    pub fn build_transparent(self) -> Material {
        let mut material = Material::transparent_pbr(self.params);
        if let Some(name) = self.name {
            material = material.with_name(name);
        }
        material
    }

    // ===== PROCEDURAL PRESETS =====

    /// Create a shield hit effect material (glowing transparent barrier)
    ///
    /// # Arguments
    /// * `hit_color` - Color of the shield (typically cyan/blue)
    /// * `intensity` - Hit intensity (0.0 = faint, 1.0 = strong)
    pub fn shield_hit(hit_color: Vec3, intensity: f32) -> Material {
        let intensity = intensity.clamp(0.0, 1.0);
        
        Self::new()
            .base_color(hit_color)
            .metallic(0.2)
            .roughness(0.3)
            .emission(hit_color * 1.5)
            .emission_strength(3.0 * intensity)
            .alpha(0.4 + 0.3 * intensity)
            .name("Shield Hit")
            .build_transparent()
    }

    /// Create an explosion flash material (bright glowing sphere)
    ///
    /// # Arguments
    /// * `temperature` - Color temperature (0.0 = red/orange, 1.0 = white/blue)
    /// * `intensity` - Explosion brightness
    pub fn explosion_flash(temperature: f32, intensity: f32) -> Material {
        let temperature = temperature.clamp(0.0, 1.0);
        let intensity = intensity.max(0.0);
        
        // Color shifts from red-orange to yellow-white to blue-white
        let base_color = if temperature < 0.5 {
            // Red to yellow
            let t = temperature * 2.0;
            Vec3::new(1.0, 0.3 + 0.7 * t, 0.1)
        } else {
            // Yellow to blue-white
            let t = (temperature - 0.5) * 2.0;
            Vec3::new(1.0, 1.0, 0.8 + 0.2 * t)
        };
        
        Self::new()
            .base_color(base_color)
            .metallic(0.0)
            .roughness(1.0)
            .emission(base_color)
            .emission_strength(5.0 * intensity)
            .alpha(0.8)
            .name("Explosion Flash")
            .build()
    }

    /// Create a laser beam material (glowing ray)
    ///
    /// # Arguments
    /// * `laser_color` - Color of the laser
    /// * `power` - Laser power (affects glow intensity)
    pub fn laser_beam(laser_color: Vec3, power: f32) -> Material {
        let power = power.max(0.0);
        
        Self::new()
            .base_color(laser_color)
            .metallic(0.0)
            .roughness(0.0)
            .emission(laser_color)
            .emission_strength(8.0 * power)
            .alpha(0.9)
            .name("Laser Beam")
            .build_transparent()
    }

    /// Create an engine glow material (pulsing thruster effect)
    ///
    /// # Arguments
    /// * `glow_color` - Color of the engine exhaust
    /// * `thrust` - Engine thrust level (0.0-1.0)
    pub fn engine_glow(glow_color: Vec3, thrust: f32) -> Material {
        let thrust = thrust.clamp(0.0, 1.0);
        
        Self::new()
            .base_color(glow_color)
            .metallic(0.0)
            .roughness(0.2)
            .emission(glow_color)
            .emission_strength(2.0 + 4.0 * thrust)
            .alpha(0.7 + 0.3 * thrust)
            .name("Engine Glow")
            .build_transparent()
    }

    /// Create a metallic surface material
    ///
    /// # Arguments
    /// * `base_color` - Color of the metal
    /// * `smoothness` - Surface smoothness (0.0 = rough, 1.0 = mirror)
    pub fn metal(base_color: Vec3, smoothness: f32) -> Material {
        let smoothness = smoothness.clamp(0.0, 1.0);
        
        Self::new()
            .base_color(base_color)
            .metallic(0.9)
            .roughness(1.0 - smoothness)
            .name("Metal")
            .build()
    }

    /// Create a glowing crystal material
    ///
    /// # Arguments
    /// * `crystal_color` - Color of the crystal
    /// * `glow_intensity` - Internal glow strength
    pub fn glowing_crystal(crystal_color: Vec3, glow_intensity: f32) -> Material {
        let glow_intensity = glow_intensity.max(0.0);
        
        Self::new()
            .base_color(crystal_color)
            .metallic(0.1)
            .roughness(0.1)
            .emission(crystal_color * 0.5)
            .emission_strength(glow_intensity)
            .alpha(0.6)
            .name("Glowing Crystal")
            .build_transparent()
    }

    /// Create a force field material (energy barrier effect)
    pub fn force_field(field_color: Vec3) -> Material {
        Self::new()
            .base_color(field_color)
            .metallic(0.0)
            .roughness(0.5)
            .emission(field_color)
            .emission_strength(1.5)
            .alpha(0.3)
            .name("Force Field")
            .build_transparent()
    }

    /// Create a hologram material (transparent glowing projection)
    pub fn hologram(holo_color: Vec3) -> Material {
        Self::new()
            .base_color(holo_color)
            .metallic(0.0)
            .roughness(0.8)
            .emission(holo_color)
            .emission_strength(2.0)
            .alpha(0.5)
            .name("Hologram")
            .build_transparent()
    }

    // ===== ANIMATION HELPERS =====

    /// Calculate pulsing emission strength
    ///
    /// # Arguments
    /// * `time` - Current time in seconds
    /// * `frequency` - Pulse frequency in Hz
    /// * `min_strength` - Minimum emission strength
    /// * `max_strength` - Maximum emission strength
    pub fn pulsing_emission(time: f32, frequency: f32, min_strength: f32, max_strength: f32) -> f32 {
        let pulse = (time * frequency * 2.0 * std::f32::consts::PI).sin();
        let normalized = (pulse + 1.0) * 0.5; // 0.0 to 1.0
        min_strength + normalized * (max_strength - min_strength)
    }

    /// Calculate flickering intensity (random-looking flicker)
    ///
    /// # Arguments
    /// * `time` - Current time in seconds
    /// * `base_intensity` - Base intensity value
    /// * `flicker_amount` - How much to flicker (0.0-1.0)
    pub fn flickering_intensity(time: f32, base_intensity: f32, flicker_amount: f32) -> f32 {
        // Use multiple sine waves at different frequencies for pseudo-random flicker
        let flicker1 = (time * 17.3).sin();
        let flicker2 = (time * 23.7).sin();
        let flicker3 = (time * 31.1).sin();
        let flicker = (flicker1 + flicker2 * 0.5 + flicker3 * 0.3) / 1.8;
        
        let variation = flicker * flicker_amount;
        (base_intensity + variation).clamp(0.0, 1.0)
    }

    /// Calculate color shift over time (for animated effects)
    ///
    /// # Arguments
    /// * `time` - Current time in seconds
    /// * `color_a` - First color
    /// * `color_b` - Second color
    /// * `cycle_duration` - Time for one complete color cycle in seconds
    pub fn color_shift(time: f32, color_a: Vec3, color_b: Vec3, cycle_duration: f32) -> Vec3 {
        let t = ((time / cycle_duration) % 1.0).abs();
        let smooth_t = t * t * (3.0 - 2.0 * t); // Smoothstep interpolation
        color_a * (1.0 - smooth_t) + color_b * smooth_t
    }
}

impl Default for MaterialBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_builder() {
        let material = MaterialBuilder::new()
            .base_color(Vec3::new(1.0, 0.0, 0.0))
            .metallic(0.8)
            .roughness(0.2)
            .build();

        if let crate::render::resources::materials::MaterialType::StandardPBR(params) = material.material_type {
            assert_eq!(params.base_color, Vec3::new(1.0, 0.0, 0.0));
            assert_eq!(params.metallic, 0.8);
            assert_eq!(params.roughness, 0.2);
        } else {
            panic!("Expected StandardPBR material");
        }
    }

    #[test]
    fn test_clamping() {
        let material = MaterialBuilder::new()
            .metallic(1.5) // Should clamp to 1.0
            .roughness(-0.5) // Should clamp to 0.0
            .alpha(2.0) // Should clamp to 1.0
            .build();

        if let crate::render::resources::materials::MaterialType::StandardPBR(params) = material.material_type {
            assert_eq!(params.metallic, 1.0);
            assert_eq!(params.roughness, 0.0);
            assert_eq!(params.alpha, 1.0);
        }
    }

    #[test]
    fn test_hex_color() {
        let material = MaterialBuilder::new()
            .base_color_hex(0xFF8000) // Orange
            .build();

        if let crate::render::resources::materials::MaterialType::StandardPBR(params) = material.material_type {
            assert!((params.base_color.x - 1.0).abs() < 0.01);
            assert!((params.base_color.y - 0.5).abs() < 0.01);
            assert!((params.base_color.z - 0.0).abs() < 0.01);
        }
    }

    #[test]
    fn test_shield_hit_preset() {
        let shield = MaterialBuilder::shield_hit(Vec3::new(0.2, 0.6, 1.0), 0.8);
        
        assert_eq!(shield.name, Some("Shield Hit".to_string()));
        
        if let crate::render::resources::materials::MaterialType::Transparent { base_material, .. } = shield.material_type {
            if let crate::render::resources::materials::MaterialType::StandardPBR(params) = *base_material {
                assert!(params.emission_strength > 0.0);
                assert!(params.alpha > 0.4);
                assert!(params.alpha < 1.0);
            }
        } else {
            panic!("Expected Transparent material");
        }
    }

    #[test]
    fn test_explosion_preset() {
        let explosion = MaterialBuilder::explosion_flash(0.5, 2.0);
        
        if let crate::render::resources::materials::MaterialType::StandardPBR(params) = explosion.material_type {
            assert!(params.emission_strength > 0.0);
            assert!(params.roughness == 1.0); // Explosions are rough
        }
    }

    #[test]
    fn test_metal_preset() {
        let metal = MaterialBuilder::metal(Vec3::new(0.8, 0.8, 0.9), 0.9);
        
        if let crate::render::resources::materials::MaterialType::StandardPBR(params) = metal.material_type {
            assert!(params.metallic > 0.8);
            assert!(params.roughness < 0.2); // High smoothness = low roughness
        }
    }

    #[test]
    fn test_pulsing_emission() {
        let strength1 = MaterialBuilder::pulsing_emission(0.0, 1.0, 1.0, 3.0);
        let strength2 = MaterialBuilder::pulsing_emission(0.25, 1.0, 1.0, 3.0);
        
        // Should pulse between min and max
        assert!(strength1 >= 1.0 && strength1 <= 3.0);
        assert!(strength2 >= 1.0 && strength2 <= 3.0);
    }

    #[test]
    fn test_color_shift() {
        let color_a = Vec3::new(1.0, 0.0, 0.0); // Red
        let color_b = Vec3::new(0.0, 0.0, 1.0); // Blue
        
        let color_start = MaterialBuilder::color_shift(0.0, color_a, color_b, 2.0);
        let color_mid = MaterialBuilder::color_shift(1.0, color_a, color_b, 2.0);
        let color_end = MaterialBuilder::color_shift(2.0, color_a, color_b, 2.0);
        
        // Should interpolate between colors
        assert!(color_start.x > color_mid.x); // More red at start
        assert!(color_mid.z > color_start.z); // More blue in middle
        assert!((color_end.x - color_start.x).abs() < 0.1); // Cycle back
    }

    #[test]
    fn test_transparent_build() {
        let material = MaterialBuilder::new()
            .alpha(0.5)
            .build_transparent();

        matches!(material.material_type, crate::render::resources::materials::MaterialType::Transparent { .. });
    }
}
