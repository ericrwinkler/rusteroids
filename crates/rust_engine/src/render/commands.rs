//! Rendering command pattern for clean render API
//!
//! This module provides simplified rendering commands for the new clean API.
//! Note: There's also render::resources::shared::render_queue::RenderCommand
//! which is used for the batch rendering system with ECS integration.
//!
//! # Design Philosophy
//!
//! The renderer should be a passive library that:
//! - Accepts pre-computed matrices (no transform logic)
//! - Accepts resource handles (no entity management)
//! - Only handles drawing (no lifecycle, input, or timing)

use crate::foundation::math::Mat4;
use crate::render::resources::materials::Material;
use crate::render::api::ObjectResourceHandle;

/// A simplified command to render a single object
///
/// This is a lighter-weight interface than the full RenderQueue system.
/// All transform calculations should be done by the ECS/scene graph before creating commands.
///
/// # Example
///
/// ```rust,ignore
/// let cmd = SimpleRenderCommand {
///     transform: entity_transform.to_matrix(),
///     material: entity.material.clone(),
///     resource_handle: entity.render_resources,
/// };
/// renderer.submit_simple_command(&cmd)?;
/// ```
#[derive(Clone)]
pub struct SimpleRenderCommand {
    /// Pre-computed model-to-world transformation matrix
    /// This should be calculated by the ECS Transform system
    pub transform: Mat4,
    
    /// Material properties and textures for rendering
    pub material: Material,
    
    /// Opaque handle to GPU resources (descriptor sets, uniform buffers, etc.)
    /// Managed by the backend, created via `create_render_resources()`
    pub resource_handle: ObjectResourceHandle,
}

impl SimpleRenderCommand {
    /// Create a new render command
    pub fn new(transform: Mat4, material: Material, resource_handle: ObjectResourceHandle) -> Self {
        Self {
            transform,
            material,
            resource_handle,
        }
    }
}

/// Batch of render commands for efficient submission
///
/// When rendering many objects, submitting them as a batch allows the
/// renderer to optimize state changes and reduce API overhead.
pub struct SimpleRenderBatch {
    /// List of commands to execute
    pub commands: Vec<SimpleRenderCommand>,
}

impl SimpleRenderBatch {
    /// Create a new empty batch
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }
    
    /// Create a batch with pre-allocated capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            commands: Vec::with_capacity(capacity),
        }
    }
    
    /// Add a command to this batch
    pub fn add(&mut self, command: SimpleRenderCommand) {
        self.commands.push(command);
    }
    
    /// Get the number of commands in this batch
    pub fn len(&self) -> usize {
        self.commands.len()
    }
    
    /// Check if the batch is empty
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
    
    /// Clear all commands from this batch
    pub fn clear(&mut self) {
        self.commands.clear();
    }
}

impl Default for SimpleRenderBatch {
    fn default() -> Self {
        Self::new()
    }
}
