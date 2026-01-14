//! UI renderer that converts widgets to render commands

use super::vertex::{UIVertex, PanelVertex};
use super::commands::UIRenderCommand;
use super::data::UIRenderData;
use crate::ui::widgets::{UIPanel, UIText, UIButton, UILayout};

/// UI renderer that generates render commands from UI elements
/// 
/// This is the bridge between high-level UI components and low-level rendering.
/// Responsibilities:
/// - Convert UI elements to vertex data
/// - Track dirty state (only update when needed)
/// - Batch similar elements for efficient rendering
pub struct UIRenderer {
    /// Render commands generated for current frame
    render_commands: Vec<UIRenderCommand>,
    
    /// Whether UI needs to be regenerated (window resize, element changes)
    dirty: bool,
    
    /// Last known screen dimensions (for detecting resize)
    last_screen_width: u32,
    last_screen_height: u32,
}

impl UIRenderer {
    /// Create a new UI renderer
    pub fn new() -> Self {
        Self {
            render_commands: Vec::new(),
            dirty: true,
            last_screen_width: 0,
            last_screen_height: 0,
        }
    }
    
    /// Mark UI as dirty (needs regeneration)
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }
    
    /// Check if UI needs regeneration
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }
    
    /// Check if screen dimensions have changed and update internal state
    pub fn check_screen_changed(&mut self, screen_width: u32, screen_height: u32) -> bool {
        let changed = self.last_screen_width != screen_width 
            || self.last_screen_height != screen_height;
        
        if changed {
            self.last_screen_width = screen_width;
            self.last_screen_height = screen_height;
            self.dirty = true;
        }
        
        changed
    }
    
    /// Update render commands for a panel
    pub fn update_panel(
        &mut self,
        panel: &UIPanel,
        screen_width: f32,
        screen_height: f32,
    ) {
        // Calculate screen-space position
        let pos = UILayout::calculate_position(&panel.element, screen_width, screen_height);
        let (width, height) = panel.element.size;
        
        // Convert to NDC coordinates
        let (x_ndc, y_ndc) = UILayout::screen_to_ndc(pos.x, pos.y, screen_width, screen_height);
        let (width_ndc, height_ndc) = UILayout::size_to_ndc(width, height, screen_width, screen_height);
        
        let x2_ndc = x_ndc + width_ndc;
        let y2_ndc = y_ndc + height_ndc;
        
        // Generate quad vertices (2 triangles, 6 vertices)
        let vertices = vec![
            PanelVertex { position: [x_ndc, y_ndc] },
            PanelVertex { position: [x2_ndc, y_ndc] },
            PanelVertex { position: [x_ndc, y2_ndc] },
            PanelVertex { position: [x_ndc, y2_ndc] },
            PanelVertex { position: [x2_ndc, y_ndc] },
            PanelVertex { position: [x2_ndc, y2_ndc] },
        ];
        
        self.render_commands.push(UIRenderCommand::Panel {
            vertices,
            color: panel.color,
        });
    }
    
    /// Update render commands for text
    pub fn update_text(
        &mut self,
        text: &UIText,
        font_atlas: &crate::render::systems::text::FontAtlas,
        screen_width: f32,
        screen_height: f32,
    ) {
        use crate::render::systems::text::TextLayout;
        
        let text_layout = TextLayout::new(font_atlas);
        let (text_vertices, text_indices) = text_layout.layout_text(&text.text);
        
        if text_vertices.is_empty() || text_indices.is_empty() {
            return;
        }
        
        let pos = UILayout::calculate_position(&text.element, screen_width, screen_height);
        let text_scale = 1.0f32;
        
        let mut ndc_vertices = Vec::with_capacity(text_indices.len());
        
        for &index in &text_indices {
            let vertex = &text_vertices[index as usize];
            
            let local_x = vertex.position.x * text_scale;
            let local_y = vertex.position.y * text_scale;
            
            let screen_x = pos.x + local_x;
            let screen_y = pos.y - local_y;
            
            let (ndc_x, ndc_y) = UILayout::screen_to_ndc(screen_x, screen_y, screen_width, screen_height);
            
            ndc_vertices.push(UIVertex {
                position: [ndc_x, ndc_y],
                uv: [vertex.uv.x, vertex.uv.y],
            });
        }
        
        self.render_commands.push(UIRenderCommand::Text {
            vertices: ndc_vertices,
            color: text.color,
        });
    }
    
    /// Update render commands for a button
    pub fn update_button(
        &mut self,
        button: &UIButton,
        font_atlas: &crate::render::systems::text::FontAtlas,
        screen_width: f32,
        screen_height: f32,
    ) {
        let background_color = button.get_current_color();
        
        // Render background panel
        let panel = UIPanel {
            element: button.element.clone(),
            color: background_color,
            border_width: button.border_width,
            border_color: button.border_color,
            corner_radius: 0.0,
        };
        self.update_panel(&panel, screen_width, screen_height);
        
        // Render text label
        if !button.text.is_empty() {
            let text_x = button.element.position.0 + 4.0;
            let text_y = button.element.position.1 + (button.element.size.1 / 2.0) + 8.0;
            
            let mut text_element = button.element.clone();
            text_element.position = (text_x, text_y);
            
            use crate::ui::widgets::{HorizontalAlign, VerticalAlign};
            
            let text_ui = UIText {
                element: text_element,
                text: button.text.clone(),
                font_size: button.font_size,
                color: button.text_color,
                h_align: HorizontalAlign::Center,
                v_align: VerticalAlign::Middle,
            };
            self.update_text(&text_ui, font_atlas, screen_width, screen_height);
        }
    }
    
    /// Get all render commands for current frame
    pub fn get_render_commands(&self) -> &[UIRenderCommand] {
        &self.render_commands
    }
    
    /// Clear render commands
    pub fn clear(&mut self) {
        self.render_commands.clear();
    }
    
    /// Mark rendering complete for this frame
    pub fn end_frame(&mut self) {
        self.dirty = false;
    }
}

impl Default for UIRenderer {
    fn default() -> Self {
        Self::new()
    }
}

/// Render UI overlay from backend-agnostic data
pub fn render_ui_data(
    ui_data: &UIRenderData,
    vulkan_renderer: &mut crate::render::backends::vulkan::VulkanRenderer,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::ui::backend::UIRenderBackend;
    
    if ui_data.quads.is_empty() && ui_data.texts.is_empty() {
        return Ok(());
    }
    
    let (screen_width, screen_height) = vulkan_renderer.get_swapchain_extent();
    let screen_width = screen_width as f32;
    let screen_height = screen_height as f32;
    
    vulkan_renderer.begin_ui_pass()?;
    
    // Batch all quads
    let mut all_panel_vertices = Vec::new();
    let mut panel_draws = Vec::new();
    
    for quad in &ui_data.quads {
        let x_ndc = (quad.position.x / screen_width) * 2.0 - 1.0;
        let y_ndc = (quad.position.y / screen_height) * 2.0 - 1.0;
        let width_ndc = (quad.size.x / screen_width) * 2.0;
        let height_ndc = (quad.size.y / screen_height) * 2.0;
        
        let x2_ndc = x_ndc + width_ndc;
        let y2_ndc = y_ndc + height_ndc;
        
        let start = all_panel_vertices.len();
        all_panel_vertices.extend_from_slice(&[
            PanelVertex { position: [x_ndc, y_ndc] },
            PanelVertex { position: [x2_ndc, y_ndc] },
            PanelVertex { position: [x_ndc, y2_ndc] },
            PanelVertex { position: [x_ndc, y2_ndc] },
            PanelVertex { position: [x2_ndc, y_ndc] },
            PanelVertex { position: [x2_ndc, y2_ndc] },
        ]);
        
        panel_draws.push((start, 6, quad.color));
    }
    
    if !all_panel_vertices.is_empty() {
        vulkan_renderer.render_panel_batch(&all_panel_vertices, &panel_draws)?;
    }
    
    // Batch all text
    if !ui_data.texts.is_empty() {
        let font_atlas = match &vulkan_renderer.font_atlas {
            Some(atlas) => atlas,
            None => {
                vulkan_renderer.end_ui_pass()?;
                return Ok(());
            }
        };
        
        let mut all_text_vertices = Vec::new();
        let mut text_draws = Vec::new();
        
        for text_elem in &ui_data.texts {
            render_text_element(
                text_elem,
                font_atlas,
                screen_width,
                screen_height,
                &mut all_text_vertices,
                &mut text_draws,
            );
        }
        
        if !all_text_vertices.is_empty() {
            vulkan_renderer.render_text_batch(&all_text_vertices, &text_draws)?;
        }
    }
    
    vulkan_renderer.end_ui_pass()?;
    Ok(())
}

fn render_text_element(
    text_elem: &super::data::RenderText,
    font_atlas: &crate::render::systems::text::FontAtlas,
    screen_width: f32,
    screen_height: f32,
    all_vertices: &mut Vec<UIVertex>,
    draws: &mut Vec<(usize, usize)>,
) {
    use crate::render::systems::text::TextLayout;
    
    let text_layout = TextLayout::new(font_atlas);
    let (text_vertices, text_indices) = text_layout.layout_text(&text_elem.text);
    
    if text_vertices.is_empty() || text_indices.is_empty() {
        return;
    }
    
    let text_scale = 1.0f32;
    let start = all_vertices.len();
    
    for &index in &text_indices {
        let vertex = &text_vertices[index as usize];
        
        let local_x = vertex.position.x * text_scale;
        let local_y = vertex.position.y * text_scale;
        
        let screen_x = text_elem.position.x + local_x;
        let screen_y = text_elem.position.y - local_y;
        
        let ndc_x = (screen_x / screen_width) * 2.0 - 1.0;
        let ndc_y = (screen_y / screen_height) * 2.0 - 1.0;
        
        all_vertices.push(UIVertex {
            position: [ndc_x, ndc_y],
            uv: [vertex.uv.x, vertex.uv.y],
        });
    }
    
    let count = all_vertices.len() - start;
    draws.push((start, count));
}
