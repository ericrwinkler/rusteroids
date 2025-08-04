// src/render/draw_queue.rs
use crate::render::mesh::Mesh;
use crate::util::math::{Mat4, Vec3};
use ash::vk;

#[derive(Debug, Clone)]
pub struct DrawCommand {
    pub mesh_id: usize,
    pub transform_matrix: Mat4,
    pub material_id: Option<usize>,
    pub depth: f32, // For sorting
}

impl DrawCommand {
    pub fn new(mesh_id: usize, transform_matrix: Mat4) -> Self {
        Self {
            mesh_id,
            transform_matrix,
            material_id: None,
            depth: 0.0,
        }
    }

    pub fn with_material(mut self, material_id: usize) -> Self {
        self.material_id = Some(material_id);
        self
    }

    pub fn with_depth(mut self, depth: f32) -> Self {
        self.depth = depth;
        self
    }
}

pub struct DrawQueue {
    pub opaque_commands: Vec<DrawCommand>,
    pub transparent_commands: Vec<DrawCommand>,
    pub ui_commands: Vec<DrawCommand>,
}

impl DrawQueue {
    pub fn new() -> Self {
        Self {
            opaque_commands: Vec::new(),
            transparent_commands: Vec::new(),
            ui_commands: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.opaque_commands.clear();
        self.transparent_commands.clear();
        self.ui_commands.clear();
    }

    pub fn submit_opaque(&mut self, command: DrawCommand) {
        self.opaque_commands.push(command);
    }

    pub fn submit_transparent(&mut self, command: DrawCommand) {
        self.transparent_commands.push(command);
    }

    pub fn submit_ui(&mut self, command: DrawCommand) {
        self.ui_commands.push(command);
    }

    pub fn sort(&mut self) {
        // Sort opaque objects front-to-back for early Z rejection
        self.opaque_commands.sort_by(|a, b| a.depth.partial_cmp(&b.depth).unwrap());
        
        // Sort transparent objects back-to-front for proper blending
        self.transparent_commands.sort_by(|a, b| b.depth.partial_cmp(&a.depth).unwrap());
        
        // UI commands usually don't need depth sorting, but we can sort by draw order
        // (they're already in submission order)
    }

    pub fn execute<F>(&self, mut draw_func: F) 
    where 
        F: FnMut(&DrawCommand)
    {
        // Draw opaque objects first
        for command in &self.opaque_commands {
            draw_func(command);
        }

        // Then transparent objects
        for command in &self.transparent_commands {
            draw_func(command);
        }

        // Finally UI elements
        for command in &self.ui_commands {
            draw_func(command);
        }
    }

    pub fn get_total_commands(&self) -> usize {
        self.opaque_commands.len() + self.transparent_commands.len() + self.ui_commands.len()
    }

    pub fn get_first_opaque_command(&self) -> Option<&DrawCommand> {
        self.opaque_commands.first()
    }
}

impl Default for DrawQueue {
    fn default() -> Self {
        Self::new()
    }
}
