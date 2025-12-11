//! Material loader with MTL file support
//!
//! Loads materials from MTL files and converts Phong lighting model to PBR parameters.

use std::path::{Path, PathBuf};
use std::fs;

use super::mtl_parser::{MtlParser, MtlData};
use crate::render::resources::materials::{Material, StandardMaterialParams};

/// Texture paths for a material
#[derive(Debug, Clone, Default)]
pub struct MaterialTexturePaths {
    /// Path to base color (albedo) texture
    pub base_color: Option<PathBuf>,
    /// Path to normal map texture
    pub normal: Option<PathBuf>,
    /// Path to emission texture
    pub emission: Option<PathBuf>,
    /// Path to metallic-roughness texture
    pub metallic_roughness: Option<PathBuf>,
    /// Path to ambient occlusion texture
    pub ambient_occlusion: Option<PathBuf>,
}

/// Loaded material with texture path metadata for optional texture loading
#[derive(Debug, Clone)]
pub struct LoadedMaterial {
    /// The material with PBR parameters
    pub material: Material,
    /// Resolved texture paths from the MTL file
    pub texture_paths: MaterialTexturePaths,
}

/// Material loader for creating Material instances from MTL files
pub struct MaterialLoader;

impl MaterialLoader {
    /// Load material from MTL file
    ///
    /// # Arguments
    /// * `mtl_path` - Path to the .mtl file
    /// * `material_name` - Name of the specific material to load (matches newmtl name)
    ///
    /// # Returns
    /// A LoadedMaterial with the material and resolved texture paths
    pub fn load_mtl(mtl_path: impl AsRef<Path>, material_name: &str) -> Result<LoadedMaterial, String> {
        let mtl_path = mtl_path.as_ref();
        
        // Read MTL file
        let contents = fs::read_to_string(mtl_path)
            .map_err(|e| format!("Failed to read MTL file {:?}: {}", mtl_path, e))?;
        
        // Parse MTL
        let materials = MtlParser::parse(&contents)?;
        
        // Find requested material
        let mtl_data = materials.get(material_name)
            .ok_or_else(|| format!("Material '{}' not found in {:?}", material_name, mtl_path))?;
        
        // Convert to PBR material with texture paths
        let loaded_material = Self::convert_to_pbr(mtl_data, mtl_path);
        
        Ok(loaded_material)
    }
    
    /// Load all materials from an MTL file
    ///
    /// # Arguments
    /// * `mtl_path` - Path to the .mtl file
    ///
    /// # Returns
    /// A vector of (material_name, LoadedMaterial) tuples
    pub fn load_all_mtl(mtl_path: impl AsRef<Path>) -> Result<Vec<(String, LoadedMaterial)>, String> {
        let mtl_path = mtl_path.as_ref();
        
        // Read MTL file
        let contents = fs::read_to_string(mtl_path)
            .map_err(|e| format!("Failed to read MTL file {:?}: {}", mtl_path, e))?;
        
        // Parse MTL
        let materials = MtlParser::parse(&contents)?;
        
        // Convert all materials
        let mut result = Vec::new();
        for (name, mtl_data) in materials {
            let loaded_material = Self::convert_to_pbr(&mtl_data, mtl_path);
            result.push((name, loaded_material));
        }
        
        Ok(result)
    }
    
    /// Load textures from paths and attach them to a material
    ///
    /// # Arguments
    /// * `loaded` - LoadedMaterial containing the material and texture paths
    /// * `graphics_engine` - Graphics engine to upload textures to
    ///
    /// # Returns
    /// Material with textures attached
    pub fn attach_textures(
        mut loaded: LoadedMaterial,
        graphics_engine: &mut crate::render::GraphicsEngine
    ) -> Result<crate::render::resources::materials::Material, String> {
        use crate::assets::ImageData;
        use crate::render::resources::materials::TextureType;
        
        // Load and attach base color texture
        if let Some(ref path) = loaded.texture_paths.base_color {
            if path.exists() {
                let image = ImageData::from_file(path)
                    .map_err(|e| format!("Failed to load base color texture {:?}: {}", path, e))?;
                let handle = graphics_engine.upload_texture_from_image_data(image, TextureType::BaseColor)
                    .map_err(|e| format!("Failed to upload base color texture: {}", e))?;
                loaded.material = loaded.material.with_base_color_texture(handle);
            }
        }
        
        // Load and attach normal map
        if let Some(ref path) = loaded.texture_paths.normal {
            if path.exists() {
                let image = ImageData::from_file(path)
                    .map_err(|e| format!("Failed to load normal map {:?}: {}", path, e))?;
                let handle = graphics_engine.upload_texture_from_image_data(image, TextureType::Normal)
                    .map_err(|e| format!("Failed to upload normal map: {}", e))?;
                loaded.material = loaded.material.with_normal_texture(handle);
            }
        }
        
        // Load and attach emission texture
        if let Some(ref path) = loaded.texture_paths.emission {
            if path.exists() {
                let image = ImageData::from_file(path)
                    .map_err(|e| format!("Failed to load emission texture {:?}: {}", path, e))?;
                let handle = graphics_engine.upload_texture_from_image_data(image, TextureType::Emission)
                    .map_err(|e| format!("Failed to upload emission texture: {}", e))?;
                loaded.material = loaded.material.with_emission_texture(handle);
            }
        }
        
        // Load and attach metallic-roughness texture
        if let Some(ref path) = loaded.texture_paths.metallic_roughness {
            if path.exists() {
                let image = ImageData::from_file(path)
                    .map_err(|e| format!("Failed to load metallic-roughness texture {:?}: {}", path, e))?;
                let handle = graphics_engine.upload_texture_from_image_data(image, TextureType::MetallicRoughness)
                    .map_err(|e| format!("Failed to upload metallic-roughness texture: {}", e))?;
                loaded.material = loaded.material.with_metallic_roughness_texture(handle);
            }
        }
        
        // Load and attach ambient occlusion texture
        if let Some(ref path) = loaded.texture_paths.ambient_occlusion {
            if path.exists() {
                let image = ImageData::from_file(path)
                    .map_err(|e| format!("Failed to load AO texture {:?}: {}", path, e))?;
                let handle = graphics_engine.upload_texture_from_image_data(image, TextureType::AmbientOcclusion)
                    .map_err(|e| format!("Failed to upload AO texture: {}", e))?;
                loaded.material = loaded.material.with_ao_texture(handle);
            }
        }
        
        Ok(loaded.material)
    }
    
    /// Load material from MTL file with automatic texture loading
    ///
    /// Convenience method that loads the MTL material and automatically
    /// loads and attaches all textures found in the MTL file.
    ///
    /// # Arguments
    /// * `mtl_path` - Path to the .mtl file
    /// * `material_name` - Name of the specific material to load
    /// * `graphics_engine` - Graphics engine to upload textures to
    ///
    /// # Returns
    /// Material with textures already attached
    pub fn load_mtl_with_textures(
        mtl_path: impl AsRef<Path>,
        material_name: &str,
        graphics_engine: &mut crate::render::GraphicsEngine
    ) -> Result<crate::render::resources::materials::Material, String> {
        let loaded = Self::load_mtl(mtl_path, material_name)?;
        Self::attach_textures(loaded, graphics_engine)
    }
    
    /// Convert Phong MTL data to PBR Material with texture paths
    ///
    /// Conversion formulas:
    /// - base_color = Kd (diffuse color)
    /// - metallic = average(Ks) (high specular = metallic)
    /// - roughness = 1.0 - (Ns / 1000.0) (high specular exponent = low roughness = shiny)
    /// - emission = Ke
    /// - alpha = d (dissolve)
    ///
    /// Automatically creates transparent material if dissolve < 1.0
    fn convert_to_pbr(mtl_data: &MtlData, mtl_path: &Path) -> LoadedMaterial {
        // Base color from diffuse
        let base_color = mtl_data.diffuse;
        
        // Metallic from specular color (high specular = metallic)
        // Average the RGB components of Ks
        let metallic = (mtl_data.specular.x + mtl_data.specular.y + mtl_data.specular.z) / 3.0;
        
        // Roughness from specular exponent (inverse relationship)
        // Ns range: 0-1000 (0 = rough, 1000 = mirror)
        // Roughness range: 0-1 (0 = mirror, 1 = rough)
        let roughness = 1.0 - (mtl_data.specular_exponent.clamp(0.0, 1000.0) / 1000.0);
        
        // Emission color
        let emission = mtl_data.emission;
        
        // Determine emission strength (use magnitude of emission color)
        let emission_strength = if emission.x > 0.0 || emission.y > 0.0 || emission.z > 0.0 {
            // If emission is present, use a default strength
            1.0
        } else {
            0.0
        };
        
        // Alpha from dissolve
        let alpha = mtl_data.dissolve;
        
        // Ambient occlusion from ambient color (average Ka components)
        // This is a rough approximation - true AO should come from a texture
        let ambient_occlusion = (mtl_data.ambient.x + mtl_data.ambient.y + mtl_data.ambient.z) / 3.0;
        
        // Create PBR material parameters
        let params = StandardMaterialParams {
            base_color,
            alpha,
            metallic,
            roughness,
            ambient_occlusion,
            emission,
            emission_strength,
            normal_scale: 1.0,
            base_color_texture_enabled: mtl_data.diffuse_map.is_some(),
            normal_texture_enabled: mtl_data.normal_map.is_some(),
        };
        
        // Create material - use transparent variant if dissolve < 1.0
        let mut material = if alpha < 1.0 {
            Material::transparent_pbr(params)
        } else {
            Material::standard_pbr(params)
        };
        
        // Set material name for debugging
        material = material.with_name(&mtl_data.name);
        
        // Resolve texture paths relative to MTL file
        let mtl_dir = Self::get_mtl_directory(mtl_path);
        let texture_paths = MaterialTexturePaths {
            base_color: mtl_data.diffuse_map.as_ref()
                .map(|path| Self::resolve_texture_path(&mtl_dir, path)),
            normal: mtl_data.normal_map.as_ref()
                .map(|path| Self::resolve_texture_path(&mtl_dir, path)),
            emission: mtl_data.emission_map.as_ref()
                .map(|path| Self::resolve_texture_path(&mtl_dir, path)),
            metallic_roughness: mtl_data.metallic_roughness_map.as_ref()
                .map(|path| Self::resolve_texture_path(&mtl_dir, path)),
            ambient_occlusion: mtl_data.ambient_occlusion_map.as_ref()
                .map(|path| Self::resolve_texture_path(&mtl_dir, path)),
        };
        
        LoadedMaterial {
            material,
            texture_paths,
        }
    }
    
    /// Get the directory containing the MTL file (for resolving relative texture paths)
    pub fn get_mtl_directory(mtl_path: &Path) -> PathBuf {
        mtl_path.parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."))
    }
    
    /// Resolve a texture path relative to a base directory
    ///
    /// Texture paths in MTL files can be:
    /// - Relative to the MTL file directory
    /// - Absolute paths
    pub fn resolve_texture_path(base_dir: &Path, texture_path: &str) -> PathBuf {
        let texture_path = Path::new(texture_path);
        
        // If absolute, return as-is
        if texture_path.is_absolute() {
            return texture_path.to_path_buf();
        }
        
        // Otherwise, resolve relative to base directory
        base_dir.join(texture_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
    use crate::foundation::math::Vec3;

    #[test]
    fn test_phong_to_pbr_conversion() {
        // Create test MTL data
        let mtl_data = MtlData {
            name: "TestMat".to_string(),
            diffuse: Vec3::new(0.8, 0.2, 0.2),
            specular: Vec3::new(0.9, 0.9, 0.9),
            specular_exponent: 800.0, // High = shiny = low roughness
            emission: Vec3::new(0.1, 0.5, 1.0),
            dissolve: 0.9,
            ..Default::default()
        };
        
        let loaded = MaterialLoader::convert_to_pbr(&mtl_data, Path::new("test.mtl"));
        
        // Check conversion
        if let crate::render::resources::materials::MaterialType::StandardPBR(params) = loaded.material.material_type {
            assert_eq!(params.base_color, Vec3::new(0.8, 0.2, 0.2));
            assert_eq!(params.alpha, 0.9);
            
            // Metallic = avg(Ks) = (0.9+0.9+0.9)/3 = 0.9
            assert!((params.metallic - 0.9).abs() < 0.001);
            
            // Roughness = 1.0 - (800/1000) = 0.2 (smooth)
            assert!((params.roughness - 0.2).abs() < 0.001);
            
            assert_eq!(params.emission, Vec3::new(0.1, 0.5, 1.0));
            assert_eq!(params.emission_strength, 1.0);
        } else {
            panic!("Expected StandardPBR material");
        }
    }

    #[test]
    fn test_roughness_conversion() {
        // Test edge cases for roughness conversion
        
        // Ns = 0 (matte) -> roughness = 1.0
        let mtl_matte = MtlData {
            specular_exponent: 0.0,
            ..Default::default()
        };
        let loaded_matte = MaterialLoader::convert_to_pbr(&mtl_matte, Path::new("test.mtl"));
        if let crate::render::resources::materials::MaterialType::StandardPBR(params) = loaded_matte.material.material_type {
            assert_eq!(params.roughness, 1.0);
        }
        
        // Ns = 1000 (mirror) -> roughness = 0.0
        let mtl_mirror = MtlData {
            specular_exponent: 1000.0,
            ..Default::default()
        };
        let loaded_mirror = MaterialLoader::convert_to_pbr(&mtl_mirror, Path::new("test.mtl"));
        if let crate::render::resources::materials::MaterialType::StandardPBR(params) = loaded_mirror.material.material_type {
            assert_eq!(params.roughness, 0.0);
        }
        
        // Ns = 500 (semi-gloss) -> roughness = 0.5
        let mtl_semi = MtlData {
            specular_exponent: 500.0,
            ..Default::default()
        };
        let loaded_semi = MaterialLoader::convert_to_pbr(&mtl_semi, Path::new("test.mtl"));
        if let crate::render::resources::materials::MaterialType::StandardPBR(params) = loaded_semi.material.material_type {
            assert_eq!(params.roughness, 0.5);
        }
    }

    #[test]
    fn test_metallic_conversion() {
        // High Ks (bright specular) = metallic
        let mtl_metal = MtlData {
            specular: Vec3::new(0.9, 0.95, 1.0),
            ..Default::default()
        };
        let loaded_metal = MaterialLoader::convert_to_pbr(&mtl_metal, Path::new("test.mtl"));
        if let crate::render::resources::materials::MaterialType::StandardPBR(params) = loaded_metal.material.material_type {
            let expected_metallic = (0.9 + 0.95 + 1.0) / 3.0;
            assert!((params.metallic - expected_metallic).abs() < 0.001);
        }
        
        // Low Ks (dim specular) = dielectric
        let mtl_dielectric = MtlData {
            specular: Vec3::new(0.1, 0.1, 0.1),
            ..Default::default()
        };
        let loaded_dielectric = MaterialLoader::convert_to_pbr(&mtl_dielectric, Path::new("test.mtl"));
        if let crate::render::resources::materials::MaterialType::StandardPBR(params) = loaded_dielectric.material.material_type {
            let expected_metallic = (0.1 + 0.1 + 0.1) / 3.0;
            assert!((params.metallic - expected_metallic).abs() < 0.001);
        }
    }

    #[test]
    fn test_load_mtl_file() {
        // Create a temporary MTL file
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, r#"
# Test MTL file
newmtl SpaceshipMetal
Ka 1.0 1.0 1.0
Kd 0.8 0.8 0.9
Ks 0.9 0.95 1.0
Ke 0.2 0.6 1.0
Ns 800.0
d 1.0
illum 2
"#).unwrap();
        
        // Load the material
        let loaded = MaterialLoader::load_mtl(temp_file.path(), "SpaceshipMetal").unwrap();
        
        // Verify it loaded correctly
        assert_eq!(loaded.material.name, Some("SpaceshipMetal".to_string()));
        
        if let crate::render::resources::materials::MaterialType::StandardPBR(params) = loaded.material.material_type {
            assert_eq!(params.base_color, Vec3::new(0.8, 0.8, 0.9));
            assert_eq!(params.emission, Vec3::new(0.2, 0.6, 1.0));
            assert!((params.roughness - 0.2).abs() < 0.001);
        } else {
            panic!("Expected StandardPBR material");
        }
    }

    #[test]
    fn test_load_nonexistent_material() {
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "newmtl ExistingMat\nKd 1.0 0.0 0.0\n").unwrap();
        
        let result = MaterialLoader::load_mtl(temp_file.path(), "NonExistentMat");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_resolve_texture_path() {
        let mtl_dir = Path::new("resources/models");
        
        // Relative path
        let resolved = MaterialLoader::resolve_texture_path(mtl_dir, "textures/diffuse.png");
        assert_eq!(resolved, PathBuf::from("resources/models/textures/diffuse.png"));
        
        // Absolute path (on Windows)
        #[cfg(windows)]
        {
            let absolute = "C:\\textures\\diffuse.png";
            let resolved = MaterialLoader::resolve_texture_path(mtl_dir, absolute);
            assert_eq!(resolved, PathBuf::from(absolute));
        }
    }
}
