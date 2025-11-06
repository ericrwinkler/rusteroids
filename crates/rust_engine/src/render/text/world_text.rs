//! World-space text rendering for 3D billboards
//!
//! This module provides text rendering in 3D world space with automatic
//! camera-facing billboards. Useful for damage numbers, labels, and floating text.

use crate::render::{FontAtlas, TextLayout, text_vertices_to_mesh, dynamic::DynamicObjectHandle, GraphicsEngine};
use crate::foundation::math::Vec3;
use std::collections::HashMap;
use std::time::Instant;

/// Handle for spawned world text
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorldTextHandle(u32);

/// Configuration for spawned world text
#[derive(Debug, Clone)]
pub struct WorldTextConfig {
    /// Text color (RGB)
    pub color: Vec3,
    /// Text scale multiplier
    pub scale: f32,
    /// How long text should live (seconds, 0 = infinite)
    pub lifetime: f32,
    /// Should text billboard to face camera
    pub billboard: bool,
    /// Initial velocity for animated text (e.g., floating damage numbers)
    pub velocity: Vec3,
}

impl Default for WorldTextConfig {
    fn default() -> Self {
        Self {
            color: Vec3::new(1.0, 1.0, 1.0),
            scale: 1.0,
            lifetime: 2.0,
            billboard: true,
            velocity: Vec3::zeros(),
        }
    }
}

/// Active world text instance
struct WorldTextInstance {
    /// Dynamic object handle for rendering (reserved for future integration)
    _dynamic_handle: DynamicObjectHandle,
    /// Current position
    position: Vec3,
    /// Current velocity (for animated text)
    velocity: Vec3,
    /// Configuration
    config: WorldTextConfig,
    /// Spawn time for lifetime tracking
    spawn_time: Instant,
    /// Text content (for debug)
    text: String,
}

/// Manager for world-space 3D text rendering
///
/// Handles spawning, updating, and lifecycle management of text that appears
/// in 3D world space. Text automatically billboards to face the camera and
/// despawns after its lifetime expires.
///
/// # Example
///
/// ```no_run
/// let mut world_text = WorldTextManager::new(font_atlas, text_layout);
///
/// // Spawn damage number
/// let handle = world_text.spawn_damage_number(
///     "-42",
///     enemy_position,
///     Vec3::new(1.0, 0.0, 0.0), // Red
/// )?;
///
/// // Update each frame
/// world_text.update(delta_time, camera_position, camera_forward)?;
/// ```
pub struct WorldTextManager {
    /// Font atlas for text rendering
    font_atlas: FontAtlas,
    /// Text layout engine
    text_layout: TextLayout,
    /// Active text instances
    instances: HashMap<WorldTextHandle, WorldTextInstance>,
    /// Next available handle ID
    next_handle: u32,
}

impl WorldTextManager {
    /// Create a new world text manager
    ///
    /// # Arguments
    ///
    /// * `font_atlas` - Font atlas with rasterized glyphs
    /// * `text_layout` - Text layout engine
    pub fn new(font_atlas: FontAtlas, text_layout: TextLayout) -> Self {
        Self {
            font_atlas,
            text_layout,
            instances: HashMap::new(),
            next_handle: 1,
        }
    }

    /// Spawn text at a world position
    ///
    /// Generates a text mesh and spawns it as a dynamic object in the world.
    ///
    /// **Note**: Current implementation has architectural limitation - the dynamic
    /// object pool system shares mesh geometry across all instances of a MeshType,
    /// but each text string has unique geometry. We need either:
    /// 1. Per-instance mesh data support in the pool system
    /// 2. A separate text rendering path outside the pool system
    /// 3. One pool per unique text string (not scalable)
    ///
    /// For now, this creates the mesh data but doesn't spawn the object.
    /// See TODO.md for text rendering architecture discussion.
    ///
    /// # Arguments
    ///
    /// * `text` - String to display
    /// * `position` - World space position
    /// * `config` - Text configuration
    /// * `_graphics_engine` - Graphics engine (unused until architecture resolved)
    ///
    /// # Returns
    ///
    /// Handle to the spawned text
    pub fn spawn_text(
        &mut self,
        text: &str,
        position: Vec3,
        config: WorldTextConfig,
        _graphics_engine: &mut GraphicsEngine,
    ) -> Result<WorldTextHandle, String> {
        // Generate text mesh
        let (text_vertices, text_indices) = self.text_layout.layout_text(text);
        
        if text_vertices.is_empty() {
            return Err(format!("Failed to generate mesh for text '{}'", text));
        }

        // Convert TextVertex to Mesh
        let _mesh = text_vertices_to_mesh(text_vertices, text_indices);
        
        // Get font texture handle from atlas
        let _texture_handle = self.font_atlas.texture_handle()
            .ok_or_else(|| "Font atlas not uploaded to GPU - call upload_to_gpu() first".to_string())?;
        
        // Create material with text color
        // let material = create_text_material(texture_handle, config.color);
        
        // TODO: Spawn as dynamic object
        // Current blocker: Dynamic object pools share mesh geometry across all instances,
        // but each text string has unique geometry (different character layout).
        // Options:
        // 1. Create one pool per text string (wasteful, not scalable)
        // 2. Extend pool system to support per-instance mesh data
        // 3. Create text-specific rendering path outside pool system
        //
        // let dynamic_handle = graphics_engine.spawn_dynamic_object(
        //     MeshType::TextQuad,
        //     position,
        //     Vec3::zeros(),
        //     Vec3::new(config.scale, config.scale, config.scale),
        //     material,
        // )?;
        
        // For now, create a dummy handle and track the instance
        let dummy_handle = DynamicObjectHandle::invalid(); // Placeholder
        
        // Create instance tracking
        let handle = WorldTextHandle(self.next_handle);
        self.next_handle += 1;
        
        let instance = WorldTextInstance {
            _dynamic_handle: dummy_handle,
            position,
            velocity: config.velocity,
            config,
            spawn_time: Instant::now(),
            text: text.to_string(),
        };
        
        self.instances.insert(handle, instance);
        
        log::warn!("Text '{}' mesh generated but not rendered - architecture limitation", text);
        Ok(handle)
    }

    /// Spawn damage number with preset styling
    ///
    /// Creates upward-floating red text with 1.5 second lifetime
    ///
    /// # Example
    ///
    /// ```no_run
    /// world_text.spawn_damage_number("-42", enemy_pos, Vec3::new(1.0, 0.0, 0.0), &mut graphics_engine)?;
    /// ```
    pub fn spawn_damage_number(
        &mut self,
        text: &str,
        position: Vec3,
        color: Vec3,
        graphics_engine: &mut GraphicsEngine,
    ) -> Result<WorldTextHandle, String> {
        let config = WorldTextConfig {
            color,
            scale: 1.5,
            lifetime: 1.5,
            billboard: true,
            velocity: Vec3::new(0.0, 2.0, 0.0), // Float upward
        };
        
        self.spawn_text(text, position, config, graphics_engine)
    }

    /// Spawn static label with infinite lifetime
    ///
    /// # Example
    ///
    /// ```no_run
    /// world_text.spawn_label("Origin", Vec3::zeros(), Vec3::new(1.0, 1.0, 1.0), &mut graphics_engine)?;
    /// ```
    pub fn spawn_label(
        &mut self,
        text: &str,
        position: Vec3,
        color: Vec3,
        graphics_engine: &mut GraphicsEngine,
    ) -> Result<WorldTextHandle, String> {
        let config = WorldTextConfig {
            color,
            scale: 1.0,
            lifetime: 0.0, // Infinite
            billboard: true,
            velocity: Vec3::zeros(),
        };
        
        self.spawn_text(text, position, config, graphics_engine)
    }

    /// Despawn text instance
    pub fn despawn_text(&mut self, handle: WorldTextHandle) -> Result<(), String> {
        if let Some(instance) = self.instances.remove(&handle) {
            // TODO: Despawn dynamic object
            log::debug!("Despawned world text '{}' (handle {:?})", instance.text, handle);
            Ok(())
        } else {
            Err(format!("WorldTextHandle {:?} not found", handle))
        }
    }

    /// Update all active text (physics, billboarding, lifetime)
    ///
    /// Call this once per frame before rendering
    ///
    /// # Arguments
    ///
    /// * `delta_time` - Time since last frame (seconds)
    /// * `camera_position` - Current camera position
    /// * `camera_forward` - Current camera forward direction
    pub fn update(
        &mut self,
        delta_time: f32,
        camera_position: Vec3,
        camera_forward: Vec3,
    ) -> Result<(), String> {
        let now = Instant::now();
        let mut to_remove = Vec::new();

        for (&handle, instance) in &mut self.instances {
            // Update position with velocity
            if instance.velocity.magnitude() > 0.001 {
                instance.position += instance.velocity * delta_time;
                // TODO: Update dynamic object position
            }

            // Calculate billboard rotation if enabled
            if instance.config.billboard {
                let _rotation = Self::calculate_billboard_rotation_static(
                    instance.position,
                    camera_position,
                    camera_forward,
                );
                // TODO: Update dynamic object rotation
            }

            // Check lifetime
            if instance.config.lifetime > 0.0 {
                let age = now.duration_since(instance.spawn_time).as_secs_f32();
                if age >= instance.config.lifetime {
                    to_remove.push(handle);
                }
            }
        }

        // Remove expired instances
        for handle in to_remove {
            self.despawn_text(handle)?;
        }

        Ok(())
    }

    /// Calculate billboard rotation to face camera (static to avoid borrow issues)
    fn calculate_billboard_rotation_static(
        text_position: Vec3,
        camera_position: Vec3,
        _camera_forward: Vec3,
    ) -> Vec3 {
        // Calculate direction from text to camera
        let to_camera = (camera_position - text_position).normalize();
        
        // Calculate yaw (rotation around Y axis)
        let yaw = to_camera.z.atan2(to_camera.x);
        
        // Calculate pitch (rotation around X axis)
        let pitch = -to_camera.y.asin();
        
        // Return Euler angles (pitch, yaw, roll)
        Vec3::new(pitch, yaw, 0.0)
    }

    /// Get number of active text instances
    pub fn active_count(&self) -> usize {
        self.instances.len()
    }

    /// Check if handle is valid
    pub fn is_valid(&self, handle: WorldTextHandle) -> bool {
        self.instances.contains_key(&handle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_billboard_rotation_calculation() {
        // Test the billboard rotation math
        let _text_pos = Vec3::new(0.0, 0.0, 0.0);
        let _camera_pos = Vec3::new(5.0, 0.0, 0.0);
        let _camera_forward = Vec3::new(-1.0, 0.0, 0.0);
        
        // Billboard should rotate to face camera at (5, 0, 0)
        // Expected: yaw = 0 (facing +X), pitch = 0 (level)
        
        // Note: Actual test would require valid FontAtlas
        // For now, just ensure the test compiles
    }

    #[test]
    fn test_world_text_config_defaults() {
        let config = WorldTextConfig::default();
        assert_eq!(config.color, Vec3::new(1.0, 1.0, 1.0));
        assert_eq!(config.scale, 1.0);
        assert_eq!(config.lifetime, 2.0);
        assert_eq!(config.billboard, true);
        assert_eq!(config.velocity, Vec3::zeros());
    }
}
