//! UI Manager
//! 
//! Central UI system that manages the scene graph, input processing, and rendering.

use super::{UIPanel, UIText, UIButton, UINodeId};
use super::backend::UIRenderBackend;
use crate::render::systems::text::FontAtlas;
use crate::render::systems::ui::{UIRenderer, UIInputProcessor, UIRenderCommand};
use crate::events::EventSystem;
use std::collections::HashMap;

/// UI element storage
enum UINode {
    Panel(UIPanel),
    Text(UIText),
    Button(UIButton),
}

/// Central UI management system
pub struct UIManager {
    /// UI elements by ID
    nodes: HashMap<UINodeId, UINode>,
    
    /// Next node ID
    next_id: u64,
    
    /// UI renderer for generating render commands
    ui_renderer: UIRenderer,
    
    /// Input processor for mouse/keyboard
    input_processor: UIInputProcessor,
    
    /// Event system for UI interactions
    event_system: EventSystem,
    
    /// Font atlas for text rendering (optional - managed by backend)
    font_atlas: Option<FontAtlas>,
    
    /// Current screen size
    screen_size: (f32, f32),
    
    /// Frame counter for timestamps
    frame_counter: u64,
}

impl UIManager {
    /// Create a new UI manager
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            next_id: 0,
            ui_renderer: UIRenderer::new(),
            input_processor: UIInputProcessor::new(800.0, 600.0), // Default screen size
            event_system: EventSystem::new(),
            font_atlas: None,
            screen_size: (800.0, 600.0),
            frame_counter: 0,
        }
    }
    
    /// Initialize with font atlas (optional - backend manages its own)
    pub fn initialize_font_atlas(&mut self, font_atlas: FontAtlas) {
        self.font_atlas = Some(font_atlas);
    }
    
    /// Add a panel to the UI
    pub fn add_panel(&mut self, panel: UIPanel) -> UINodeId {
        let id = UINodeId(self.next_id);
        self.next_id += 1;
        self.nodes.insert(id, UINode::Panel(panel));
        id
    }
    
    /// Add text to the UI
    pub fn add_text(&mut self, text: UIText) -> UINodeId {
        let id = UINodeId(self.next_id);
        self.next_id += 1;
        self.nodes.insert(id, UINode::Text(text));
        id
    }
    
    /// Add a button to the UI
    pub fn add_button(&mut self, button: UIButton) -> UINodeId {
        let id = UINodeId(self.next_id);
        self.next_id += 1;
        self.nodes.insert(id, UINode::Button(button));
        id
    }
    
    /// Remove a UI element
    pub fn remove_element(&mut self, id: UINodeId) {
        self.nodes.remove(&id);
    }
    
    /// Update text content
    pub fn update_text(&mut self, id: UINodeId, new_text: String) {
        if let Some(UINode::Text(text)) = self.nodes.get_mut(&id) {
            text.text = new_text;
            self.ui_renderer.mark_dirty(); // Mark dirty so render commands are regenerated
        }
    }
    
    /// Get button reference
    pub fn get_button(&self, id: UINodeId) -> Option<&UIButton> {
        if let Some(UINode::Button(button)) = self.nodes.get(&id) {
            Some(button)
        } else {
            None
        }
    }
    
    /// Get button mutable reference
    pub fn get_button_mut(&mut self, id: UINodeId) -> Option<&mut UIButton> {
        if let Some(UINode::Button(button)) = self.nodes.get_mut(&id) {
            Some(button)
        } else {
            None
        }
    }
    
    /// Update mouse position
    pub fn update_mouse_position(&mut self, x: f32, y: f32) {
        self.input_processor.update_mouse_position(x, y);
    }
    
    /// Update mouse button state
    pub fn update_mouse_button(&mut self, button: crate::render::systems::ui::MouseButton, pressed: bool) {
        self.input_processor.update_mouse_button(button, pressed);
    }
    
    /// Set screen size
    pub fn set_screen_size(&mut self, width: f32, height: f32) {
        self.screen_size = (width, height);
        self.input_processor.set_screen_size(width, height);
    }
    
    /// Get screen size
    pub fn get_screen_size(&self) -> (f32, f32) {
        self.screen_size
    }
    
    /// Get event system reference
    pub fn event_system(&self) -> &EventSystem {
        &self.event_system
    }
    
    /// Get event system mutable reference
    pub fn event_system_mut(&mut self) -> &mut EventSystem {
        &mut self.event_system
    }
    
    /// Update UI state (call once per frame before rendering)
    pub fn update(&mut self, _delta_time: f32) {
        self.frame_counter += 1;
        self.input_processor.begin_frame();
        
        // Process button input
        let timestamp = self.frame_counter as f64;
        let button_ids: Vec<UINodeId> = self.nodes.iter()
            .filter_map(|(id, node)| {
                if matches!(node, UINode::Button(_)) {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect();
        
        for id in button_ids {
            if let Some(UINode::Button(button)) = self.nodes.get_mut(&id) {
                self.input_processor.process_button(button, &mut self.event_system, timestamp);
            }
        }
        
        // Reset per-frame input flags AFTER processing all buttons
        self.input_processor.reset_frame_flags();
    }
    
    /// Render UI to the backend
    pub fn render(&mut self, backend: &mut dyn UIRenderBackend) -> Result<(), Box<dyn std::error::Error>> {
        // Font atlas is managed by the backend (e.g., VulkanRenderer)
        // UIManager just needs a reference for text layout
        let font_atlas = match &self.font_atlas {
            Some(atlas) => atlas,
            None => {
                // If no font atlas in UIManager, backend should have its own
                // Just skip text rendering for now
                log::warn!("No font atlas in UIManager - text will not render");
                return Ok(());
            }
        };
        
        log::debug!("UIManager rendering {} nodes", self.nodes.len());
        let (screen_width, screen_height) = self.screen_size;
        
        // Check if screen changed
        let screen_changed = self.ui_renderer.check_screen_changed(screen_width as u32, screen_height as u32);
        
        // Always rebuild UI for now (TODO: dirty tracking)
        if screen_changed || self.ui_renderer.is_dirty() {
            self.ui_renderer.clear();
            
            // Collect and sort nodes by z-order
            let mut sorted_nodes: Vec<(&UINodeId, &UINode)> = self.nodes.iter().collect();
            sorted_nodes.sort_by_key(|(_, node)| {
                match node {
                    UINode::Panel(panel) => panel.element.z_order,
                    UINode::Text(text) => text.element.z_order,
                    UINode::Button(button) => button.element.z_order,
                }
            });
            
            // Generate render commands
            for (_, node) in sorted_nodes {
                match node {
                    UINode::Panel(panel) => {
                        log::debug!("Rendering panel: {:?}", panel.element.position);
                        self.ui_renderer.update_panel(panel, screen_width, screen_height);
                    }
                    UINode::Text(text) => {
                        log::debug!("Rendering text: '{}' at {:?}", text.text, text.element.position);
                        self.ui_renderer.update_text(text, font_atlas, screen_width, screen_height);
                    }
                    UINode::Button(button) => {
                        log::debug!("Rendering button: '{}' at {:?}", button.text, button.element.position);
                        self.ui_renderer.update_button(button, font_atlas, screen_width, screen_height);
                    }
                }
            }
        }
        
        // Get render commands
        let commands: Vec<_> = self.ui_renderer.get_render_commands().to_vec();
        log::debug!("UIManager has {} render commands", commands.len());
        
        // Log command types
        for (i, command) in commands.iter().enumerate() {
            match command {
                UIRenderCommand::Panel { vertices, color } => {
                    log::debug!("  Command {}: Panel with {} vertices, color: {:?}", i, vertices.len(), color);
                }
                UIRenderCommand::Text { vertices, color } => {
                    log::debug!("  Command {}: Text with {} vertices, color: {:?}", i, vertices.len(), color);
                }
            }
        }
        
        // Batch panels
        let mut all_panel_vertices = Vec::new();
        let mut panel_draws = Vec::new();
        
        for command in commands.iter() {
            if let UIRenderCommand::Panel { vertices, color } = command {
                let start = all_panel_vertices.len();
                all_panel_vertices.extend_from_slice(vertices);
                panel_draws.push((start, vertices.len(), *color));
            }
        }
        
        // Batch text
        let mut all_text_vertices = Vec::new();
        let mut text_draws = Vec::new();
        
        for command in commands.iter() {
            if let UIRenderCommand::Text { vertices, color: _ } = command {
                all_text_vertices.extend_from_slice(vertices);
                text_draws.push((all_text_vertices.len() - vertices.len(), vertices.len()));
            }
        }
        
        // Render batches
        backend.begin_ui_pass()?;
        
        if !all_panel_vertices.is_empty() {
            backend.render_panel_batch(&all_panel_vertices, &panel_draws)?;
        }
        
        if !all_text_vertices.is_empty() {
            backend.render_text_batch(&all_text_vertices, &text_draws)?;
        }
        
        backend.end_ui_pass()?;
        
        // Mark frame complete
        self.ui_renderer.end_frame();
        
        Ok(())
    }
    
    /// Dispatch all pending events
    pub fn dispatch_events(&mut self) {
        self.event_system.dispatch();
    }
}

impl Default for UIManager {
    fn default() -> Self {
        Self::new()
    }
}
