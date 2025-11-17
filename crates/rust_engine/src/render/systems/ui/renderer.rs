//! UI rendering system
//!
//! Handles batched rendering of UI elements (panels, text, buttons) following
//! Game Engine Architecture Chapter 11.4.4 principles:
//! - Renders AFTER 3D scene with depth testing disabled
//! - Orthographic projection for 2D screen-space elements
//! - Batched draw calls to minimize state changes
//! - Dirty tracking to avoid unnecessary buffer updates

use crate::foundation::math::Vec4;
use super::{UIPanel, UIText, UIButton, UILayout};

/// Vertex data for UI rendering
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct UIVertex {
    /// Position in NDC coordinates
    pub position: [f32; 2],
    /// Texture coordinates (for text/textured elements)
    pub uv: [f32; 2],
}

/// Simple position-only vertex for solid color panels (no UVs)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PanelVertex {
    /// Position in NDC coordinates
    pub position: [f32; 2],
}

/// UI render command for a single element
#[derive(Debug, Clone)]
pub enum UIRenderCommand {
    /// Render a solid color panel (position-only vertices, 8 bytes per vertex)
    Panel {
        /// Panel vertex data
        vertices: Vec<PanelVertex>,
        /// Panel color
        color: Vec4,
    },
    /// Render text using glyph atlas (position + UV vertices, 16 bytes per vertex)
    Text {
        /// Text vertex data
        vertices: Vec<UIVertex>,
        /// Text color
        color: Vec4,
    },
}

/// UI renderer that generates render commands from UI elements
/// 
/// This is the bridge between high-level UI components and low-level Vulkan rendering.
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
    /// 
    /// Generates a quad with 6 vertices (2 triangles) in pixel-space,
    /// converted to NDC using current screen dimensions.
    /// 
    /// Note: Caller should check dirty state before calling this to avoid
    /// unnecessary work. This method always adds to render_commands.
    pub fn update_panel(
        &mut self,
        panel: &UIPanel,
        screen_width: f32,
        screen_height: f32,
    ) {
        // Calculate screen-space position
        let pos = UILayout::calculate_position(&panel.element, screen_width, screen_height);
        let (width, height) = panel.element.size;
        
        // Convert to NDC coordinates (SOURCE OF TRUTH formula)
        let (x_ndc, y_ndc) = UILayout::screen_to_ndc(pos.x, pos.y, screen_width, screen_height);
        let (width_ndc, height_ndc) = UILayout::size_to_ndc(width, height, screen_width, screen_height);
        
        let x2_ndc = x_ndc + width_ndc;
        let y2_ndc = y_ndc + height_ndc;
        
        // Generate quad vertices (2 triangles, 6 vertices)
        let vertices = vec![
            // Triangle 1
            PanelVertex { position: [x_ndc, y_ndc] },      // Top-left
            PanelVertex { position: [x2_ndc, y_ndc] },     // Top-right
            PanelVertex { position: [x_ndc, y2_ndc] },     // Bottom-left
            // Triangle 2
            PanelVertex { position: [x_ndc, y2_ndc] },     // Bottom-left
            PanelVertex { position: [x2_ndc, y_ndc] },     // Top-right
            PanelVertex { position: [x2_ndc, y2_ndc] },    // Bottom-right
        ];
        
        self.render_commands.push(UIRenderCommand::Panel {
            vertices,
            color: panel.color,
        });
    }
    
    /// Update render commands for text
    /// 
    /// Uses TextLayout system to generate quads for each glyph, converting
    /// from local glyph space to screen-space NDC coordinates.
    /// 
    /// Note: Caller should check dirty state before calling this to avoid
    /// unnecessary work. This method always adds to render_commands.
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
        
        log::debug!("update_text: '{}' produced {} vertices, {} indices", 
            text.text, text_vertices.len(), text_indices.len());
        
        if text_vertices.is_empty() || text_indices.is_empty() {
            log::warn!("update_text: No vertices generated for text '{}'", text.text);
            return; // Nothing to render
        }
        
        // Calculate screen-space position
        let pos = UILayout::calculate_position(&text.element, screen_width, screen_height);
        let text_scale = 1.0f32; // TODO: Make configurable via UIText
        
        // Convert glyph vertices to screen-space NDC
        let mut ndc_vertices = Vec::with_capacity(text_indices.len());
        
        for &index in &text_indices {
            let vertex = &text_vertices[index as usize];
            
            // Scale glyph vertex and position relative to text origin
            let local_x = vertex.position.x * text_scale;
            let local_y = vertex.position.y * text_scale;
            
            // Convert to screen coordinates (Y-flip for fonts)
            let screen_x = pos.x + local_x;
            let screen_y = pos.y - local_y;
            
            // Convert to NDC
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
    /// 
    /// Renders button as:
    /// 1. Background panel (colored based on button state)
    /// 2. Text label (centered on button)
    /// 
    /// Note: Caller should check dirty state before calling this to avoid
    /// unnecessary work. This method always adds to render_commands.
    pub fn update_button(
        &mut self,
        button: &UIButton,
        font_atlas: &crate::render::systems::text::FontAtlas,
        screen_width: f32,
        screen_height: f32,
    ) {
        // Get color based on button state
        let background_color = match button.state {
            super::components::ButtonState::Normal => button.normal_color,
            super::components::ButtonState::Hovered => button.hover_color,
            super::components::ButtonState::Pressed => button.pressed_color,
            super::components::ButtonState::Disabled => button.disabled_color,
        };
        
        // Render background panel
        let panel = UIPanel {
            element: button.element.clone(),
            color: background_color,
            border_width: button.border_width,
            border_color: button.border_color,
            corner_radius: 0.0,
        };
        self.update_panel(&panel, screen_width, screen_height);
        
        // Render text label centered on button
        if !button.text.is_empty() {
            // Offset text position to center it within the button
            let text_x = button.element.position.0 + 4.0; // Small padding from left (FIX ME, center text by length, add text positioning modes)
            let text_y = button.element.position.1 + (button.element.size.1 / 2.0) + 8.0; // plus half font height
            
            let mut text_element = button.element.clone();
            text_element.position = (text_x, text_y);
            
            let text_ui = UIText {
                element: text_element,
                text: button.text.clone(),
                font_size: button.font_size,
                color: button.text_color,
                h_align: super::components::HorizontalAlign::Center,
                v_align: super::components::VerticalAlign::Middle,
            };
            self.update_text(&text_ui, font_atlas, screen_width, screen_height);
        }
    }
    
    /// Get all render commands for current frame
    pub fn get_render_commands(&self) -> &[UIRenderCommand] {
        &self.render_commands
    }
    
    /// Clear render commands (call when dirty to regenerate)
    /// This clears the command list but does NOT reset the dirty flag yet
    pub fn clear(&mut self) {
        self.render_commands.clear();
        // Note: dirty flag stays true until rendering complete
    }
    
    /// Mark rendering complete for this frame (resets dirty flag)
    pub fn end_frame(&mut self) {
        self.dirty = false;
    }
}

impl Default for UIRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::systems::ui::{UIElement, Anchor};
    
    #[test]
    fn test_panel_vertex_generation() {
        let mut renderer = UIRenderer::new();
        
        let panel = UIPanel {
            element: UIElement {
                position: (10.0, 10.0),
                size: (100.0, 50.0),
                anchor: Anchor::TopLeft,
                visible: true,
                z_order: 0,
            },
            color: Vec4::new(0.0, 0.0, 1.0, 0.7),
            ..Default::default()
        };
        
        renderer.update_panel(&panel, 800.0, 600.0);
        
        let commands = renderer.get_render_commands();
        assert_eq!(commands.len(), 1);
        
        match &commands[0] {
            UIRenderCommand::Panel { vertices, color } => {
                assert_eq!(vertices.len(), 6); // 2 triangles
                assert_eq!(*color, Vec4::new(0.0, 0.0, 1.0, 0.7));
            }
            _ => panic!("Expected Panel command"),
        }
    }
    
    #[test]
    fn test_dirty_tracking_on_resize() {
        let mut renderer = UIRenderer::new();
        assert!(renderer.is_dirty()); // Initially dirty
        
        let panel = UIPanel::default();
        renderer.update_panel(&panel, 800.0, 600.0);
        renderer.clear();
        
        assert!(!renderer.is_dirty()); // Clean after update
        
        // Resize should mark dirty
        renderer.update_panel(&panel, 1024.0, 768.0);
        assert!(renderer.is_dirty());
    }
}
