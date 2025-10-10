//! # Batch Renderer
//!
//! This module provides a batch rendering system that efficiently submits
//! multiple render commands to the GPU in a single frame. It replaces the
//! inefficient single-object rendering approach with optimized batching.
//!
//! ## Architecture
//!
//! - **BatchRenderer**: Coordinates batch submission to GPU
//! - **RenderBatch**: Group of commands sharing resources for optimal GPU utilization
//! - **BatchExecutor**: Handles actual GPU command submission
//!
//! ## Performance Goals
//!
//! - Submit 100+ objects in <2ms per frame
//! - Minimize GPU state changes through intelligent batching
//! - Efficient memory usage with pre-allocated command buffers

use crate::render::render_queue::{RenderQueue, RenderCommand};
use crate::render::material::MaterialId;
use crate::render::{Renderer, Mesh, Material};
use nalgebra::Matrix4;

/// Result type for batch rendering operations
pub type BatchResult<T> = Result<T, BatchError>;

/// Errors that can occur during batch rendering
#[derive(Debug, thiserror::Error)]
pub enum BatchError {
    /// General rendering errors from the GPU or graphics API
    #[error("Rendering error: {0}")]
    RenderingError(String),
    
    /// Invalid or unknown material ID requested
    #[error("Invalid material ID: {0:?}")]
    InvalidMaterial(MaterialId),
    
    /// Too many commands for a single batch
    #[error("Batch size limit exceeded: {current} > {max}")]
    BatchSizeExceeded { 
        /// Current number of commands
        current: usize, 
        /// Maximum allowed commands
        max: usize 
    },
}

/// A batch of render commands for efficient rendering
#[derive(Debug)]
pub struct RenderBatch {
    /// All commands in this batch
    commands: Vec<RenderCommand>,
    
    /// Material used by all commands in this batch (for material-based batching)
    material_id: Option<MaterialId>,
}

impl RenderBatch {
    /// Create a new empty render batch
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            material_id: None,
        }
    }
    
    /// Create a batch with a specific material
    pub fn with_material(material_id: MaterialId) -> Self {
        Self {
            commands: Vec::new(),
            material_id: Some(material_id),
        }
    }
    
    /// Add a command to this batch
    pub fn add_command(&mut self, command: RenderCommand) -> BatchResult<()> {
        // Validate material compatibility
        if let Some(batch_material) = self.material_id {
            if command.material_id != batch_material {
                return Err(BatchError::InvalidMaterial(command.material_id));
            }
        }
        
        // Set batch material from first command if not set
        if self.commands.is_empty() && self.material_id.is_none() {
            self.material_id = Some(command.material_id);
        }
        
        self.commands.push(command);
        Ok(())
    }
    
    /// Get all commands in this batch
    pub fn commands(&self) -> &[RenderCommand] {
        &self.commands
    }
    
    /// Get the material ID for this batch
    pub fn material_id(&self) -> Option<MaterialId> {
        self.material_id
    }
    
    /// Check if this batch is empty
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
    
    /// Get the number of commands in this batch
    pub fn command_count(&self) -> usize {
        self.commands.len()
    }
}

/// Statistics for batch rendering performance monitoring
#[derive(Debug, Clone, Default)]
pub struct BatchStats {
    /// Total number of render commands processed
    pub total_commands: usize,
    
    /// Number of batches created
    pub batch_count: usize,
    
    /// Time spent collecting commands (microseconds)
    pub collection_time_us: u64,
    
    /// Time spent submitting to GPU (microseconds)
    pub submission_time_us: u64,
    
    /// Number of GPU state changes
    pub state_changes: usize,
}

impl BatchStats {
    /// Calculate average commands per batch
    pub fn avg_commands_per_batch(&self) -> f32 {
        if self.batch_count == 0 {
            0.0
        } else {
            self.total_commands as f32 / self.batch_count as f32
        }
    }
    
    /// Get total frame time in microseconds
    pub fn total_time_us(&self) -> u64 {
        self.collection_time_us + self.submission_time_us
    }
}

/// Batch rendering system that efficiently submits multiple objects to GPU
pub struct BatchRenderer {
    /// Current batches being built
    current_batches: Vec<RenderBatch>,
    
    /// Maximum number of commands per batch
    max_batch_size: usize,
    
    /// Performance statistics
    stats: BatchStats,
    
    /// Whether to batch by material
    batch_by_material: bool,
}

impl BatchRenderer {
    /// Create a new batch renderer with default settings
    pub fn new() -> Self {
        Self {
            current_batches: Vec::new(),
            max_batch_size: 100,
            stats: BatchStats::default(),
            batch_by_material: true,
        }
    }
    
    /// Create a batch renderer with custom configuration
    pub fn with_config(max_batch_size: usize, batch_by_material: bool) -> Self {
        Self {
            current_batches: Vec::new(),
            max_batch_size,
            stats: BatchStats::default(),
            batch_by_material,
        }
    }
    
    /// Process a render queue by batching commands
    pub fn process_queue(&mut self, queue: &RenderQueue) -> BatchResult<()> {
        let start_time = std::time::Instant::now();
        
        // Reset stats and batches
        self.stats = BatchStats::default();
        self.current_batches.clear();
        
        // Process opaque commands first
        self.process_commands(queue.opaque_commands())?;
        
        // Process transparent commands second
        self.process_commands(queue.transparent_commands())?;
        
        let collection_time = start_time.elapsed();
        self.stats.collection_time_us = collection_time.as_micros() as u64;
        self.stats.total_commands = queue.command_count();
        
        Ok(())
    }
    
    /// Process a set of commands into batches
    fn process_commands(&mut self, commands: &[RenderCommand]) -> BatchResult<()> {
        for command in commands {
            self.add_command_to_batch(command.clone())?;
        }
        Ok(())
    }
    
    /// Add a command to an appropriate batch
    fn add_command_to_batch(&mut self, command: RenderCommand) -> BatchResult<()> {
        // Find an existing compatible batch
        let mut batch_index = None;
        
        for (i, batch) in self.current_batches.iter().enumerate() {
            if self.is_command_compatible_with_batch(&command, batch) {
                if batch.command_count() < self.max_batch_size {
                    batch_index = Some(i);
                    break;
                }
            }
        }
        
        // Add to existing batch or create new one
        match batch_index {
            Some(index) => {
                self.current_batches[index].add_command(command)?;
            }
            None => {
                let mut new_batch = if self.batch_by_material {
                    RenderBatch::with_material(command.material_id)
                } else {
                    RenderBatch::new()
                };
                
                new_batch.add_command(command)?;
                self.current_batches.push(new_batch);
                self.stats.batch_count += 1;
            }
        }
        
        Ok(())
    }
    
    /// Check if a command is compatible with a batch
    fn is_command_compatible_with_batch(&self, command: &RenderCommand, batch: &RenderBatch) -> bool {
        if self.batch_by_material {
            if let Some(batch_material) = batch.material_id() {
                return command.material_id == batch_material;
            }
        }
        
        // If no specific batching strategy, any command is compatible
        true
    }
    
    /// Execute batches using the current renderer
    /// This method submits all render commands through individual draw_mesh_3d calls
    pub fn execute_with_renderer(&mut self, queue: &RenderQueue, renderer: &mut crate::render::Renderer) -> BatchResult<()> {
        let start_time = std::time::Instant::now();
        
        // Process the queue into batches
        self.process_queue(queue)?;
        
        // Submit batches to renderer
        let submission_start = std::time::Instant::now();
        
        // For now, we'll submit each command individually through draw_mesh_3d
        // TODO: Implement true batch submission when renderer supports it
        
        // First render opaque objects (front to back)
        for command in queue.opaque_commands() {
            self.submit_single_command(command, renderer)?;
        }
        
        // Then render transparent objects (back to front)
        for command in queue.transparent_commands() {
            self.submit_single_command(command, renderer)?;
        }
        
        let submission_time = submission_start.elapsed();
        let _total_time = start_time.elapsed();
        
        self.stats.submission_time_us = submission_time.as_micros() as u64;
        self.stats.total_commands = queue.command_count();
        self.stats.batch_count = self.current_batches.len();
        
        Ok(())
    }
    
    /// Submit a single render command to the renderer
    /// This is a temporary bridge until we have true batch rendering
    fn submit_single_command(&self, _command: &RenderCommand, _renderer: &mut crate::render::Renderer) -> BatchResult<()> {
        // TODO: Implement proper command submission once we have mesh/material access
        // For now, this is handled in the main render loop directly
        
        // This method will be fully implemented when we add mesh/material storage to RenderCommand
        // or implement a proper asset management system that can resolve IDs to objects
        // The actual rendering will happen in the fallback in main_dynamic.rs
        
        Ok(())
    }
    
    /// Get render commands that need to be submitted
    /// This allows external code to iterate through batches and submit them
    pub fn get_render_commands(&self) -> Vec<&RenderCommand> {
        let mut commands = Vec::new();
        
        for batch in &self.current_batches {
            commands.extend(batch.commands());
        }
        
        commands
    }
    
    /// Get current batch rendering statistics
    pub fn stats(&self) -> &BatchStats {
        &self.stats
    }
    
    /// Reset statistics
    pub fn reset_stats(&mut self) {
        self.stats = BatchStats::default();
    }
}

impl Default for BatchRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_batch_creation() {
        let batch = RenderBatch::new();
        assert!(batch.is_empty());
        assert_eq!(batch.command_count(), 0);
        assert_eq!(batch.material_id(), None);
    }

    #[test]
    fn test_batch_with_material() {
        let material = MaterialId(1);
        let batch = RenderBatch::with_material(material);
        assert_eq!(batch.material_id(), Some(material));
    }

    #[test]
    fn test_batch_renderer_creation() {
        let renderer = BatchRenderer::new();
        assert_eq!(renderer.max_batch_size, 100);
        assert!(renderer.batch_by_material);
    }

    #[test]
    fn test_batch_stats() {
        let mut stats = BatchStats::default();
        stats.total_commands = 100;
        stats.batch_count = 10;
        
        assert_eq!(stats.avg_commands_per_batch(), 10.0);
        assert_eq!(stats.total_time_us(), 0);
    }
}
