//! UI Render Backend Trait
//! 
//! Defines the interface between the UI system and rendering backends.
//! Keeps the UI system independent of Vulkan/DirectX/OpenGL specifics.

use crate::render::systems::text::FontAtlas;
use crate::ui::rendering::{PanelVertex, UIVertex};
use crate::foundation::math::Vec4;

/// Handle to a font atlas uploaded to the GPU
#[derive(Debug, Clone, Copy)]
pub struct FontAtlasHandle(pub u64);

/// Backend-agnostic UI rendering interface
pub trait UIRenderBackend {
    /// Begin UI rendering pass
    fn begin_ui_pass(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    
    /// Render a batch of UI panels
    /// 
    /// # Arguments
    /// * `vertices` - All panel vertices in a single buffer
    /// * `draws` - Draw ranges: (start_index, vertex_count, color)
    fn render_panel_batch(
        &mut self,
        vertices: &[PanelVertex],
        draws: &[(usize, usize, Vec4)],
    ) -> Result<(), Box<dyn std::error::Error>>;
    
    /// Render a batch of text glyphs
    /// 
    /// # Arguments
    /// * `vertices` - All text vertices in a single buffer
    /// * `draws` - Draw ranges: (start_vertex, vertex_count)
    fn render_text_batch(
        &mut self,
        vertices: &[UIVertex],
        draws: &[(usize, usize)],
    ) -> Result<(), Box<dyn std::error::Error>>;
    
    /// End UI rendering pass
    fn end_ui_pass(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    
    /// Upload font atlas to GPU
    fn upload_font_atlas(
        &mut self,
        atlas: &FontAtlas,
    ) -> Result<FontAtlasHandle, Box<dyn std::error::Error>>;
    
    /// Update screen size for UI coordinate calculations
    fn set_screen_size(&mut self, width: u32, height: u32);
    
    /// Get current screen size
    fn get_screen_size(&self) -> (u32, u32);
}
