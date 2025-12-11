//! UI rendering system
//!
//! Handles batched rendering of UI elements (panels, text, buttons) following
//! Game Engine Architecture Chapter 11.4.4 principles:
//! - Renders AFTER 3D scene with depth testing disabled
//! - Orthographic projection for 2D screen-space elements
//! - Batched draw calls to minimize state changes
//! - Dirty tracking to avoid unnecessary buffer updates

use crate::foundation::math::{Vec2, Vec4};
use crate::render::resources::materials::TextureHandle;
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

// =============================================================================
// Frame Rendering Data (Proposal #6)
// =============================================================================

/// Backend-agnostic UI rendering data
/// 
/// UI Manager exports this structure containing all UI elements to render.
/// Renderer consumes this without needing to know UI node hierarchy.
#[derive(Debug, Clone, Default)]
pub struct UIRenderData {
    /// UI quads (panels, buttons, sprites)
    pub quads: Vec<RenderQuad>,
    
    /// UI text elements
    pub texts: Vec<RenderText>,
}

impl UIRenderData {
    /// Create empty UI render data with no elements
    pub fn empty() -> Self {
        Self::default()
    }
}

/// UI quad primitive (panel, button background, sprite) for rendering
#[derive(Debug, Clone)]
pub struct RenderQuad {
    /// Screen position (pixels from top-left)
    pub position: Vec2,
    
    /// Size (width, height in pixels)
    pub size: Vec2,
    
    /// Color (RGBA)
    pub color: Vec4,
    
    /// Optional texture
    pub texture: Option<TextureHandle>,
    
    /// Z-order for layering (higher = front)
    pub depth: f32,
}

/// UI text primitive for rendering
#[derive(Debug, Clone)]
pub struct RenderText {
    /// Screen position (pixels from top-left)
    pub position: Vec2,
    
    /// Text content
    pub text: String,
    
    /// Font ID (index into font atlas)
    pub font: usize,
    
    /// Text color (RGBA)
    pub color: Vec4,
    
    /// Font size (pixels)
    pub size: f32,
    
    /// Z-order for layering (higher = front)
    pub depth: f32,
}

// =============================================================================
// UIRenderer (Original System)
// =============================================================================

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

// =============================================================================
// Unified Frame Rendering Support (Proposal #6)
// =============================================================================

/// Render UI overlay from backend-agnostic data
///
/// This function converts UIRenderData (quads, text) into vertex buffers
/// and submits them to the Vulkan backend for rendering. Used by the
/// unified render_frame() API.
///
/// # Arguments
/// * `ui_data` - Backend-agnostic UI rendering primitives
/// * `vulkan_renderer` - Vulkan backend to submit draw calls to
pub fn render_ui_data(
    ui_data: &UIRenderData,
    vulkan_renderer: &mut crate::render::backends::vulkan::VulkanRenderer,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::ui::UIRenderBackend;
    
    // Early exit if no UI elements
    if ui_data.quads.is_empty() && ui_data.texts.is_empty() {
        return Ok(());
    }
    
    // Get current screen size for coordinate conversion
    let (screen_width, screen_height) = vulkan_renderer.get_swapchain_extent();
    let screen_width = screen_width as f32;
    let screen_height = screen_height as f32;
    
    // Begin UI pass
    vulkan_renderer.begin_ui_pass()?;
    
    // ---- Batch all quads ----
    let mut all_panel_vertices = Vec::new();
    let mut panel_draws = Vec::new();
    
    for quad in &ui_data.quads {
        // Convert pixel coordinates to NDC (-1 to +1)
        let x_ndc = (quad.position.x / screen_width) * 2.0 - 1.0;
        let y_ndc = (quad.position.y / screen_height) * 2.0 - 1.0;
        let width_ndc = (quad.size.x / screen_width) * 2.0;
        let height_ndc = (quad.size.y / screen_height) * 2.0;
        
        let x2_ndc = x_ndc + width_ndc;
        let y2_ndc = y_ndc + height_ndc;
        
        // Generate quad vertices (2 triangles, 6 vertices)
        let start = all_panel_vertices.len();
        all_panel_vertices.extend_from_slice(&[
            PanelVertex { position: [x_ndc, y_ndc] },      // Top-left
            PanelVertex { position: [x2_ndc, y_ndc] },     // Top-right
            PanelVertex { position: [x_ndc, y2_ndc] },     // Bottom-left
            PanelVertex { position: [x_ndc, y2_ndc] },     // Bottom-left
            PanelVertex { position: [x2_ndc, y_ndc] },     // Top-right
            PanelVertex { position: [x2_ndc, y2_ndc] },    // Bottom-right
        ]);
        
        panel_draws.push((start, 6, quad.color));
    }
    
    // Render panel batch
    if !all_panel_vertices.is_empty() {
        vulkan_renderer.render_panel_batch(&all_panel_vertices, &panel_draws)?;
    }
    
    // ---- Batch all text ----
    if !ui_data.texts.is_empty() {
        // Get font atlas from Vulkan renderer
        let font_atlas = match &vulkan_renderer.font_atlas {
            Some(atlas) => atlas,
            None => {
                log::warn!("render_ui_data: No font atlas available for text rendering");
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
        
        // Render text batch
        if !all_text_vertices.is_empty() {
            vulkan_renderer.render_text_batch(&all_text_vertices, &text_draws)?;
        }
    }
    
    // End UI pass
    vulkan_renderer.end_ui_pass()?;
    
    Ok(())
}

/// Render a single text element, appending vertices to the batch
fn render_text_element(
    text_elem: &RenderText,
    font_atlas: &crate::render::systems::text::FontAtlas,
    screen_width: f32,
    screen_height: f32,
    all_vertices: &mut Vec<UIVertex>,
    draws: &mut Vec<(usize, usize)>,
) {
    use crate::render::systems::text::TextLayout;
    
    // Generate glyph vertices using TextLayout
    let text_layout = TextLayout::new(font_atlas);
    let (text_vertices, text_indices) = text_layout.layout_text(&text_elem.text);
    
    if text_vertices.is_empty() || text_indices.is_empty() {
        return; // Skip empty text
    }
    
    let text_scale = 1.0f32; // Match UIRenderer default
    let start = all_vertices.len();
    
    for &index in &text_indices {
        let vertex = &text_vertices[index as usize];
        
        // Scale glyph vertex and position relative to text origin
        let local_x = vertex.position.x * text_scale;
        let local_y = vertex.position.y * text_scale;
        
        // Convert to screen coordinates (Y-flip for fonts)
        let screen_x = text_elem.position.x + local_x;
        let screen_y = text_elem.position.y - local_y;
        
        // Convert to NDC
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
