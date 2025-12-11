//! MTL (Material Template Library) file parser
//!
//! Parses Wavefront .mtl files into structured data for material creation.
//! Supports standard Phong lighting properties and common texture maps.

use std::collections::HashMap;

use crate::foundation::math::Vec3;

/// Parsed MTL material data (Wavefront Phong model)
#[derive(Debug, Clone)]
pub struct MtlData {
    /// Material name
    pub name: String,
    /// Ambient color (Ka)
    pub ambient: Vec3,
    /// Diffuse color (Kd) - becomes base_color in PBR
    pub diffuse: Vec3,
    /// Specular color (Ks) - used to derive metallic in PBR
    pub specular: Vec3,
    /// Emission color (Ke)
    pub emission: Vec3,
    /// Specular exponent (Ns) - 0 to 1000, used to derive roughness in PBR
    pub specular_exponent: f32,
    /// Dissolve/opacity (d) - 0.0 = transparent, 1.0 = opaque
    pub dissolve: f32,
    /// Illumination model (illum) - 0-10
    pub illumination_model: u32,
    /// Diffuse texture map (map_Kd)
    pub diffuse_map: Option<String>,
    /// Specular texture map (map_Ks)
    pub specular_map: Option<String>,
    /// Bump/normal map (map_Bump or bump)
    pub normal_map: Option<String>,
    /// Emission texture map (map_Ke) - Blender extension
    pub emission_map: Option<String>,
    /// Metallic-roughness texture map (map_Pr/map_Pm) - PBR extension
    pub metallic_roughness_map: Option<String>,
    /// Ambient occlusion map (map_Ka)
    pub ambient_occlusion_map: Option<String>,
}

impl Default for MtlData {
    fn default() -> Self {
        Self {
            name: String::new(),
            ambient: Vec3::new(1.0, 1.0, 1.0),
            diffuse: Vec3::new(0.8, 0.8, 0.8),
            specular: Vec3::new(0.5, 0.5, 0.5),
            emission: Vec3::new(0.0, 0.0, 0.0),
            specular_exponent: 250.0,
            dissolve: 1.0,
            illumination_model: 2,
            diffuse_map: None,
            specular_map: None,
            normal_map: None,
            emission_map: None,
            metallic_roughness_map: None,
            ambient_occlusion_map: None,
        }
    }
}

/// MTL file parser
pub struct MtlParser;

impl MtlParser {
    /// Parse MTL file contents into a map of material name -> MtlData
    ///
    /// # Arguments
    /// * `contents` - The text contents of the MTL file
    ///
    /// # Returns
    /// A HashMap mapping material names to their parsed data
    pub fn parse(contents: &str) -> Result<HashMap<String, MtlData>, String> {
        let mut materials = HashMap::new();
        let mut current_material: Option<MtlData> = None;
        
        for (line_num, line) in contents.lines().enumerate() {
            let line = line.trim();
            
            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            
            let mut tokens = line.split_whitespace();
            let command = match tokens.next() {
                Some(cmd) => cmd,
                None => continue,
            };
            
            match command {
                "newmtl" => {
                    // Save previous material if exists
                    if let Some(mat) = current_material.take() {
                        materials.insert(mat.name.clone(), mat);
                    }
                    
                    // Start new material
                    let name = tokens.next()
                        .ok_or_else(|| format!("Line {}: newmtl missing material name", line_num + 1))?
                        .to_string();
                    current_material = Some(MtlData {
                        name,
                        ..Default::default()
                    });
                },
                
                "Ka" => {
                    if let Some(ref mut mat) = current_material {
                        mat.ambient = Self::parse_vec3(&mut tokens, line_num, "Ka")?;
                    }
                },
                
                "Kd" => {
                    if let Some(ref mut mat) = current_material {
                        mat.diffuse = Self::parse_vec3(&mut tokens, line_num, "Kd")?;
                    }
                },
                
                "Ks" => {
                    if let Some(ref mut mat) = current_material {
                        mat.specular = Self::parse_vec3(&mut tokens, line_num, "Ks")?;
                    }
                },
                
                "Ke" => {
                    if let Some(ref mut mat) = current_material {
                        mat.emission = Self::parse_vec3(&mut tokens, line_num, "Ke")?;
                    }
                },
                
                "Ns" => {
                    if let Some(ref mut mat) = current_material {
                        mat.specular_exponent = Self::parse_f32(&mut tokens, line_num, "Ns")?;
                    }
                },
                
                "d" => {
                    if let Some(ref mut mat) = current_material {
                        mat.dissolve = Self::parse_f32(&mut tokens, line_num, "d")?;
                    }
                },
                
                "Tr" => {
                    // Transparency (inverted dissolve): Tr = 1.0 - d
                    if let Some(ref mut mat) = current_material {
                        let transparency = Self::parse_f32(&mut tokens, line_num, "Tr")?;
                        mat.dissolve = 1.0 - transparency;
                    }
                },
                
                "illum" => {
                    if let Some(ref mut mat) = current_material {
                        mat.illumination_model = Self::parse_u32(&mut tokens, line_num, "illum")?;
                    }
                },
                
                "map_Kd" => {
                    if let Some(ref mut mat) = current_material {
                        mat.diffuse_map = Some(Self::parse_texture_path(&mut tokens, line_num, "map_Kd")?);
                    }
                },
                
                "map_Ks" => {
                    if let Some(ref mut mat) = current_material {
                        mat.specular_map = Some(Self::parse_texture_path(&mut tokens, line_num, "map_Ks")?);
                    }
                },
                
                "map_Bump" | "bump" => {
                    if let Some(ref mut mat) = current_material {
                        mat.normal_map = Some(Self::parse_texture_path(&mut tokens, line_num, command)?);
                    }
                },
                
                "map_Ke" => {
                    if let Some(ref mut mat) = current_material {
                        mat.emission_map = Some(Self::parse_texture_path(&mut tokens, line_num, "map_Ke")?);
                    }
                },
                
                "map_Ka" => {
                    if let Some(ref mut mat) = current_material {
                        mat.ambient_occlusion_map = Some(Self::parse_texture_path(&mut tokens, line_num, "map_Ka")?);
                    }
                },
                
                // Ignore unknown commands silently
                _ => {}
            }
        }
        
        // Save final material
        if let Some(mat) = current_material {
            materials.insert(mat.name.clone(), mat);
        }
        
        Ok(materials)
    }
    
    /// Parse a Vec3 color from RGB tokens
    fn parse_vec3<'a, I>(tokens: &mut I, line_num: usize, command: &str) -> Result<Vec3, String>
    where
        I: Iterator<Item = &'a str>
    {
        let r = Self::parse_f32(tokens, line_num, command)?;
        let g = Self::parse_f32(tokens, line_num, command)?;
        let b = Self::parse_f32(tokens, line_num, command)?;
        Ok(Vec3::new(r, g, b))
    }
    
    /// Parse a single f32 value
    fn parse_f32<'a, I>(tokens: &mut I, line_num: usize, command: &str) -> Result<f32, String>
    where
        I: Iterator<Item = &'a str>
    {
        let token = tokens.next()
            .ok_or_else(|| format!("Line {}: {} missing value", line_num + 1, command))?;
        token.parse::<f32>()
            .map_err(|_| format!("Line {}: {} invalid float value '{}'", line_num + 1, command, token))
    }
    
    /// Parse a single u32 value
    fn parse_u32<'a, I>(tokens: &mut I, line_num: usize, command: &str) -> Result<u32, String>
    where
        I: Iterator<Item = &'a str>
    {
        let token = tokens.next()
            .ok_or_else(|| format!("Line {}: {} missing value", line_num + 1, command))?;
        token.parse::<u32>()
            .map_err(|_| format!("Line {}: {} invalid integer value '{}'", line_num + 1, command, token))
    }
    
    /// Parse texture file path (may contain spaces, take rest of line)
    fn parse_texture_path<'a, I>(tokens: &mut I, line_num: usize, command: &str) -> Result<String, String>
    where
        I: Iterator<Item = &'a str>
    {
        // Collect remaining tokens (texture paths can have spaces)
        let path: Vec<&str> = tokens.collect();
        if path.is_empty() {
            return Err(format!("Line {}: {} missing texture path", line_num + 1, command));
        }
        Ok(path.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_material() {
        let mtl_content = r#"
# Simple material
newmtl TestMaterial
Ka 1.0 1.0 1.0
Kd 0.8 0.2 0.2
Ks 0.5 0.5 0.5
Ns 250.0
d 1.0
illum 2
"#;
        
        let materials = MtlParser::parse(mtl_content).unwrap();
        assert_eq!(materials.len(), 1);
        
        let mat = materials.get("TestMaterial").unwrap();
        assert_eq!(mat.name, "TestMaterial");
        assert_eq!(mat.diffuse, Vec3::new(0.8, 0.2, 0.2));
        assert_eq!(mat.specular_exponent, 250.0);
        assert_eq!(mat.dissolve, 1.0);
    }

    #[test]
    fn test_parse_material_with_textures() {
        let mtl_content = r#"
newmtl TexturedMaterial
Kd 1.0 1.0 1.0
map_Kd textures/diffuse.png
map_Bump textures/normal.png
map_Ke textures/emission.png
"#;
        
        let materials = MtlParser::parse(mtl_content).unwrap();
        let mat = materials.get("TexturedMaterial").unwrap();
        
        assert_eq!(mat.diffuse_map, Some("textures/diffuse.png".to_string()));
        assert_eq!(mat.normal_map, Some("textures/normal.png".to_string()));
        assert_eq!(mat.emission_map, Some("textures/emission.png".to_string()));
    }

    #[test]
    fn test_parse_multiple_materials() {
        let mtl_content = r#"
newmtl Material1
Kd 1.0 0.0 0.0

newmtl Material2
Kd 0.0 1.0 0.0
"#;
        
        let materials = MtlParser::parse(mtl_content).unwrap();
        assert_eq!(materials.len(), 2);
        
        assert_eq!(materials.get("Material1").unwrap().diffuse, Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(materials.get("Material2").unwrap().diffuse, Vec3::new(0.0, 1.0, 0.0));
    }

    #[test]
    fn test_parse_transparency() {
        let mtl_content = r#"
newmtl TransparentMat
Tr 0.3
"#;
        
        let materials = MtlParser::parse(mtl_content).unwrap();
        let mat = materials.get("TransparentMat").unwrap();
        
        // Tr = 1.0 - d, so Tr 0.3 means d = 0.7
        assert!((mat.dissolve - 0.7).abs() < 0.001);
    }

    #[test]
    fn test_parse_emission() {
        let mtl_content = r#"
newmtl EmissiveMat
Ke 0.2 0.6 1.0
"#;
        
        let materials = MtlParser::parse(mtl_content).unwrap();
        let mat = materials.get("EmissiveMat").unwrap();
        
        assert_eq!(mat.emission, Vec3::new(0.2, 0.6, 1.0));
    }
}
