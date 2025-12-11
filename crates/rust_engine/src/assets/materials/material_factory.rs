//! Unified material creation API
//!
//! MaterialFactory provides a single interface for creating materials from any source:
//! - File-based loading (MTL files)
//! - Procedural building (MaterialBuilder)
//! - Preset materials for common use cases

use std::path::Path;
use crate::render::resources::materials::Material;
use super::{MaterialLoader, MaterialBuilder, LoadedMaterial};

/// Unified factory for material creation
pub struct MaterialFactory;

impl MaterialFactory {
    /// Load material from MTL file
    ///
    /// # Arguments
    /// * `mtl_path` - Path to the .mtl file
    /// * `material_name` - Name of the material to load
    ///
    /// # Returns
    /// LoadedMaterial with the material and texture paths
    pub fn from_mtl(
        mtl_path: impl AsRef<Path>,
        material_name: &str,
    ) -> Result<LoadedMaterial, String> {
        MaterialLoader::load_mtl(mtl_path, material_name)
    }

    /// Load material from MTL file with automatic texture loading
    ///
    /// # Arguments
    /// * `mtl_path` - Path to the .mtl file
    /// * `material_name` - Name of the material to load
    /// * `graphics_engine` - Graphics engine to upload textures to
    ///
    /// # Returns
    /// Material with all textures loaded and attached
    pub fn from_mtl_with_textures(
        mtl_path: impl AsRef<Path>,
        material_name: &str,
        graphics_engine: &mut crate::render::GraphicsEngine,
    ) -> Result<Material, String> {
        MaterialLoader::load_mtl_with_textures(mtl_path, material_name, graphics_engine)
    }

    /// Create material using procedural builder
    ///
    /// # Returns
    /// MaterialBuilder for fluent API construction
    pub fn builder() -> MaterialBuilder {
        MaterialBuilder::new()
    }

    /// Create material from preset
    ///
    /// Available presets:
    /// - "shield_hit" - Energy shield impact effect
    /// - "explosion_flash" - Bright explosion flash
    /// - "laser_beam" - Laser weapon beam
    /// - "engine_glow" - Engine exhaust glow
    /// - "metal" - Metallic surface
    /// - "glowing_crystal" - Luminous crystal
    /// - "force_field" - Energy force field
    /// - "hologram" - Holographic display
    ///
    /// # Arguments
    /// * `preset_name` - Name of the preset
    ///
    /// # Returns
    /// Material with preset configuration, or None if preset doesn't exist
    pub fn from_preset(preset_name: &str) -> Option<Material> {
        use crate::foundation::math::Vec3;

        match preset_name {
            "shield_hit" => Some(MaterialBuilder::shield_hit(
                Vec3::new(0.3, 0.7, 1.0),
                2.0,
            )),
            "explosion_flash" => Some(MaterialBuilder::explosion_flash(
                6000.0, // temperature
                10.0,   // intensity
            )),
            "laser_beam" => Some(MaterialBuilder::laser_beam(
                Vec3::new(1.0, 0.2, 0.2),
                5.0,
            )),
            "engine_glow" => Some(MaterialBuilder::engine_glow(
                Vec3::new(0.2, 0.6, 1.0),
                3.0,
            )),
            "metal" => Some(MaterialBuilder::metal(
                Vec3::new(0.8, 0.8, 0.8),
                0.9, // smoothness (high = shiny)
            )),
            "glowing_crystal" => Some(MaterialBuilder::glowing_crystal(
                Vec3::new(0.3, 1.0, 0.7),
                2.0,
            )),
            "force_field" => Some(MaterialBuilder::force_field(
                Vec3::new(0.0, 1.0, 0.5),
            )),
            "hologram" => Some(MaterialBuilder::hologram(
                Vec3::new(0.0, 1.0, 1.0),
            )),
            _ => None,
        }
    }

    /// List all available preset names
    pub fn list_presets() -> Vec<&'static str> {
        vec![
            "shield_hit",
            "explosion_flash",
            "laser_beam",
            "engine_glow",
            "metal",
            "glowing_crystal",
            "force_field",
            "hologram",
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preset_exists() {
        let material = MaterialFactory::from_preset("metal");
        assert!(material.is_some());
    }

    #[test]
    fn test_preset_nonexistent() {
        let material = MaterialFactory::from_preset("nonexistent");
        assert!(material.is_none());
    }

    #[test]
    fn test_list_presets() {
        let presets = MaterialFactory::list_presets();
        assert_eq!(presets.len(), 8);
        assert!(presets.contains(&"metal"));
        assert!(presets.contains(&"laser_beam"));
    }

    #[test]
    fn test_builder() {
        let material = MaterialFactory::builder()
            .base_color_rgb(1.0, 0.0, 0.0)
            .metallic(0.5)
            .build();
        
        assert!(material.name.is_none());
    }
}
