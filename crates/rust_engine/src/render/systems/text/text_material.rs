//! Text material utilities
//!
//! Helper functions for creating materials suitable for text rendering.
//! Text can be rendered in two modes:
//! - **Lit text**: Participates in scene lighting (world-space UI, 3D signs, etc.)
//! - **Unlit text**: Flat shaded (screen-space UI, HUD elements)

use crate::render::resources::materials::{Material, UnlitMaterialParams, StandardMaterialParams, TextureHandle};
use crate::foundation::math::Vec3;

/// Create a lit text material that participates in scene lighting
///
/// Uses the TransparentPBR pipeline with alpha blending. The text will receive
/// lighting from scene lights and cast shadows. Ideal for world-space text like
/// 3D signs, labels, floating damage numbers, etc.
///
/// # Arguments
///
/// * `font_atlas_texture` - Handle to the font atlas texture (alpha channel is glyph coverage)
/// * `color` - RGB color to tint the text (0.0-1.0 range)
///
/// # Example
///
/// ```no_run
/// let white = Vec3::new(1.0, 1.0, 1.0);
/// let material = create_lit_text_material(atlas_texture, white);
/// ```
pub fn create_lit_text_material(
    font_atlas_texture: TextureHandle,
    color: Vec3,
) -> Material {
    Material::transparent_pbr(StandardMaterialParams {
        base_color: color,
        metallic: 0.0,     // Text is non-metallic
        roughness: 1.0,    // Text is fully rough (diffuse)
        base_color_texture_enabled: true,  // Enable texture sampling!
        ..Default::default()
    })
    .with_base_color_texture(font_atlas_texture)
}

/// Create an unlit text material for flat rendering
///
/// Uses the TransparentUnlit pipeline with alpha blending. The text will NOT receive
/// lighting and will always appear at full brightness. Ideal for UI elements like
/// HUD text, menus, tooltips, etc.
///
/// # Arguments
///
/// * `font_atlas_texture` - Handle to the font atlas texture (alpha channel is glyph coverage)
/// * `color` - RGB color to tint the text (0.0-1.0 range)
///
/// # Example
///
/// ```no_run
/// let white = Vec3::new(1.0, 1.0, 1.0);
/// let material = create_unlit_text_material(atlas_texture, white);
/// ```
pub fn create_unlit_text_material(
    font_atlas_texture: TextureHandle,
    color: Vec3,
) -> Material {
    Material::transparent_unlit(UnlitMaterialParams {
        color,
        alpha: 1.0,
    })
    .with_base_color_texture(font_atlas_texture)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_lit_text_material() {
        let texture = TextureHandle(42);
        let white = Vec3::new(1.0, 1.0, 1.0);
        
        let material = create_lit_text_material(texture, white);
        
        // Verify it's a transparent PBR material
        assert_eq!(material.required_pipeline(), crate::render::resources::materials::PipelineType::TransparentPBR);
        
        // Verify texture is attached
        assert_eq!(material.textures.base_color, Some(texture));
    }
    
    #[test]
    fn test_unlit_text_material() {
        let texture = TextureHandle(42);
        let white = Vec3::new(1.0, 1.0, 1.0);
        
        let material = create_unlit_text_material(texture, white);
        
        // Verify it's a transparent unlit material
        assert_eq!(material.required_pipeline(), crate::render::resources::materials::PipelineType::TransparentUnlit);
        
        // Verify texture is attached
        assert_eq!(material.textures.base_color, Some(texture));
    }
}
