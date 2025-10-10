//! # Render Queue System
//!
//! This module provides a batch rendering system that collects and organizes
//! render commands for efficient GPU submission. It replaces the single-object
//! draw_mesh_3d() approach with a batched system that can handle multiple objects
//! per frame.
//!
//! ## Architecture
//!
//! - **RenderQueue**: Collects and organizes all render commands for a frame
//! - **RenderCommand**: Individual rendering instruction with mesh, material, and transform
//! - **CommandType**: Separates opaque and transparent objects for proper rendering order
//!
//! ## Performance Goals
//!
//! - Collect 100+ render commands in <1ms
//! - Submit commands to GPU in batches for maximum efficiency
//! - Memory-efficient storage with minimal allocations per frame

use crate::ecs::Entity;
use crate::render::material::MaterialId;
use nalgebra::Matrix4;

/// Handle to a render command for sorting and organization
pub type RenderCommandId = u32;

/// Type of render command for sorting and batching
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandType {
    /// Opaque objects rendered front-to-back for depth testing efficiency
    Opaque,
    /// Transparent objects rendered back-to-front for alpha blending
    Transparent,
}

/// Individual render command containing all data needed to render an object
#[repr(C)]
#[derive(Debug, Clone)]
pub struct RenderCommand {
    /// Entity that owns this render command
    pub entity_id: Entity,
    
    /// Material ID to apply
    pub material_id: MaterialId,
    
    /// World transform matrix
    pub transform: Matrix4<f32>,
    
    /// Type of command for sorting
    pub command_type: CommandType,
    
    /// Distance from camera for depth sorting
    pub depth_key: f32,
}

impl RenderCommand {
    /// Create a new opaque render command
    pub fn opaque(
        entity_id: Entity,
        material_id: MaterialId,
        transform: Matrix4<f32>,
        depth_key: f32,
    ) -> Self {
        Self {
            entity_id,
            material_id,
            transform,
            command_type: CommandType::Opaque,
            depth_key,
        }
    }
    
    /// Create a new transparent render command
    pub fn transparent(
        entity_id: Entity,
        material_id: MaterialId,
        transform: Matrix4<f32>,
        depth_key: f32,
    ) -> Self {
        Self {
            entity_id,
            material_id,
            transform,
            command_type: CommandType::Transparent,
            depth_key,
        }
    }
}

/// Collection of render commands organized for efficient GPU submission
pub struct RenderQueue {
    /// Opaque objects sorted front-to-back (near to far)
    opaque_commands: Vec<RenderCommand>,
    
    /// Transparent objects sorted back-to-front (far to near)
    transparent_commands: Vec<RenderCommand>,
    
    /// Command counter for unique IDs
    next_command_id: RenderCommandId,
}

impl RenderQueue {
    /// Create a new empty render queue
    pub fn new() -> Self {
        Self {
            opaque_commands: Vec::new(),
            transparent_commands: Vec::new(),
            next_command_id: 0,
        }
    }
    
    /// Create a render queue with pre-allocated capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            opaque_commands: Vec::with_capacity(capacity),
            transparent_commands: Vec::with_capacity(capacity / 4), // Fewer transparent objects typically
            next_command_id: 0,
        }
    }
    
    /// Add a render command to the queue
    pub fn add_command(&mut self, command: RenderCommand) -> RenderCommandId {
        let command_id = self.next_command_id;
        self.next_command_id += 1;
        
        match command.command_type {
            CommandType::Opaque => {
                self.opaque_commands.push(command);
            }
            CommandType::Transparent => {
                self.transparent_commands.push(command);
            }
        }
        
        command_id
    }
    
    /// Sort commands for optimal rendering order
    pub fn sort_commands(&mut self) {
        // Sort opaque objects front-to-back (near to far) for early depth rejection
        self.opaque_commands.sort_by(|a, b| {
            a.depth_key.partial_cmp(&b.depth_key).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        // Sort transparent objects back-to-front (far to near) for correct alpha blending
        self.transparent_commands.sort_by(|a, b| {
            b.depth_key.partial_cmp(&a.depth_key).unwrap_or(std::cmp::Ordering::Equal)
        });
    }
    
    /// Get all opaque commands in sorted order
    pub fn opaque_commands(&self) -> &[RenderCommand] {
        &self.opaque_commands
    }
    
    /// Get all transparent commands in sorted order
    pub fn transparent_commands(&self) -> &[RenderCommand] {
        &self.transparent_commands
    }
    
    /// Get total number of commands
    pub fn command_count(&self) -> usize {
        self.opaque_commands.len() + self.transparent_commands.len()
    }
    
    /// Clear all commands for next frame
    pub fn clear(&mut self) {
        self.opaque_commands.clear();
        self.transparent_commands.clear();
        self.next_command_id = 0;
    }
    
    /// Check if queue is empty
    pub fn is_empty(&self) -> bool {
        self.opaque_commands.is_empty() && self.transparent_commands.is_empty()
    }
}

impl Default for RenderQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_queue_creation() {
        let queue = RenderQueue::new();
        assert!(queue.is_empty());
        assert_eq!(queue.command_count(), 0);
    }

    #[test]
    fn test_add_opaque_command() {
        let mut world = crate::ecs::World::new();
        let entity = world.create_entity();
        
        let mut queue = RenderQueue::new();
        let command = RenderCommand::opaque(
            entity,
            MaterialId(0),
            Matrix4::identity(),
            5.0,
        );
        
        let command_id = queue.add_command(command);
        assert_eq!(command_id, 0);
        assert_eq!(queue.command_count(), 1);
        assert_eq!(queue.opaque_commands().len(), 1);
        assert_eq!(queue.transparent_commands().len(), 0);
    }

    #[test]
    fn test_add_transparent_command() {
        let mut world = crate::ecs::World::new();
        let entity = world.create_entity();
        
        let mut queue = RenderQueue::new();
        let command = RenderCommand::transparent(
            entity,
            MaterialId(0),
            Matrix4::identity(),
            5.0,
        );
        
        let command_id = queue.add_command(command);
        assert_eq!(command_id, 0);
        assert_eq!(queue.command_count(), 1);
        assert_eq!(queue.opaque_commands().len(), 0);
        assert_eq!(queue.transparent_commands().len(), 1);
    }

    #[test]
    fn test_command_sorting() {
        let mut world = crate::ecs::World::new();
        let entity1 = world.create_entity();
        let entity2 = world.create_entity();
        let entity3 = world.create_entity();
        let entity4 = world.create_entity();
        
        let mut queue = RenderQueue::new();
        
        // Add opaque objects with different depths
        queue.add_command(RenderCommand::opaque(
            entity1, MaterialId(0), Matrix4::identity(), 10.0 // Far
        ));
        queue.add_command(RenderCommand::opaque(
            entity2, MaterialId(0), Matrix4::identity(), 5.0 // Near
        ));
        
        // Add transparent objects with different depths
        queue.add_command(RenderCommand::transparent(
            entity3, MaterialId(0), Matrix4::identity(), 8.0 // Far
        ));
        queue.add_command(RenderCommand::transparent(
            entity4, MaterialId(0), Matrix4::identity(), 3.0 // Near
        ));
        
        queue.sort_commands();
        
        // Opaque should be sorted front-to-back (near to far)
        let opaque = queue.opaque_commands();
        assert_eq!(opaque[0].depth_key, 5.0);  // Nearest first
        assert_eq!(opaque[1].depth_key, 10.0); // Farthest last
        
        // Transparent should be sorted back-to-front (far to near)
        let transparent = queue.transparent_commands();
        assert_eq!(transparent[0].depth_key, 8.0); // Farthest first
        assert_eq!(transparent[1].depth_key, 3.0); // Nearest last
    }

    #[test]
    fn test_clear_queue() {
        let mut world = crate::ecs::World::new();
        let entity = world.create_entity();
        
        let mut queue = RenderQueue::new();
        queue.add_command(RenderCommand::opaque(
            entity, MaterialId(0), Matrix4::identity(), 5.0
        ));
        
        assert_eq!(queue.command_count(), 1);
        
        queue.clear();
        assert!(queue.is_empty());
        assert_eq!(queue.command_count(), 0);
    }
}
