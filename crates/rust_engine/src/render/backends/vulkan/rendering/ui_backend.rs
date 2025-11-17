//! Vulkan implementation of UIRenderBackend
//! 
//! Provides low-level Vulkan rendering for UI primitives.

use crate::ui::backend::{UIRenderBackend, FontAtlasHandle};
use crate::render::systems::text::FontAtlas;
use crate::render::systems::ui::{PanelVertex, UIVertex};
use crate::render::backends::vulkan::{VulkanRenderer, renderer::CommandRecordingState};
use crate::foundation::math::Vec4;

impl UIRenderBackend for VulkanRenderer {
    fn begin_ui_pass(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // UI rendering happens within the active render pass
        // No explicit begin needed - just ensure pipelines are initialized
        self.ensure_ui_pipeline_initialized()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        self.ensure_ui_text_pipeline_initialized()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        Ok(())
    }
    
    fn render_panel_batch(
        &mut self,
        vertices: &[PanelVertex],
        draws: &[(usize, usize, Vec4)],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let recording_state = self.get_command_recording_state()
            .ok_or("No active command recording")?;
        
        let command_buffer = recording_state.command_buffer;
        
        // Upload all vertices
        self.upload_ui_panel_vertices(command_buffer, vertices)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        
        // Issue draw calls for each range with different colors
        for (start, count, color) in draws {
            self.draw_ui_panel_range(command_buffer, *start, *count, *color)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        }
        
        Ok(())
    }
    
    fn render_text_batch(
        &mut self,
        vertices: &[UIVertex],
        draws: &[(usize, usize)],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let recording_state = self.get_command_recording_state()
            .ok_or("No active command recording")?;
        
        let command_buffer = recording_state.command_buffer;
        
        // Convert UIVertex to flat f32 array
        let mut flat_vertices = Vec::with_capacity(vertices.len() * 4);
        for vertex in vertices {
            flat_vertices.push(vertex.position[0]);
            flat_vertices.push(vertex.position[1]);
            flat_vertices.push(vertex.uv[0]);
            flat_vertices.push(vertex.uv[1]);
        }
        
        // Upload and draw
        self.upload_text_vertices_batch(command_buffer, &flat_vertices, draws)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        
        Ok(())
    }
    
    fn end_ui_pass(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // UI rendering completes within the render pass
        // No explicit end needed
        Ok(())
    }
    
    fn upload_font_atlas(
        &mut self,
        atlas: &FontAtlas,
    ) -> Result<FontAtlasHandle, Box<dyn std::error::Error>> {
        self.ensure_font_atlas_initialized_internal()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        Ok(FontAtlasHandle(0)) // Single font atlas for now
    }
    
    fn set_screen_size(&mut self, width: u32, height: u32) {
        // Screen size is tracked by swapchain extent
        // This is a no-op for Vulkan - size comes from swapchain
        let _ = (width, height);
    }
    
    fn get_screen_size(&self) -> (u32, u32) {
        self.get_swapchain_extent()
    }
}

impl VulkanRenderer {
    /// Get command recording state (needed for backend impl)
    pub(crate) fn get_command_recording_state(&self) -> Option<&crate::render::backends::vulkan::renderer::CommandRecordingState> {
        self.command_recording_state.as_ref()
    }
}
