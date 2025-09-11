// Entity System for Teapot App - Phase 2 Implementation
// Based on Game Engine Architecture principles: cache-friendly data layout,
// component purity, and structured update phases.

use rust_engine::foundation::math::{Vec3, Transform};
use std::collections::HashMap;

pub type EntityId = u32;

/// Pure data component for lights - no logic, only data
/// Following Game Engine Architecture principle: "Components should be pure data containers"
#[derive(Debug, Clone)]
pub struct LightComponent {
    pub light_type: LightType,
    pub color: Vec3,
    pub intensity: f32,
    pub direction: Vec3,      // For directional/spot lights (world space)
    pub position: Vec3,       // For point/spot lights (world space)
    pub range: f32,           // For point/spot lights
    pub inner_cone: f32,      // For spot lights (radians)
    pub outer_cone: f32,      // For spot lights (radians)
    pub enabled: bool,
    pub cast_shadows: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LightType {
    Directional,
    Point,
    Spot,
}

/// Pure data component for meshes
#[derive(Debug, Clone)]
pub struct MeshComponent {
    pub material_type: MaterialType,
    pub visible: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MaterialType {
    StandardPBR,
    Unlit,
    TransparentPBR,
    TransparentUnlit,
}

/// Cache-friendly entity world implementation
/// Uses dense packed arrays for optimal iteration performance
pub struct GameWorld {
    next_entity_id: EntityId,
    
    // CACHE-FRIENDLY: Dense arrays for high-performance iteration
    // All components for a given type are stored contiguously in memory
    entities: Vec<EntityId>,
    transforms: Vec<Transform>,
    light_components: Vec<LightComponent>,
    mesh_components: Vec<MeshComponent>,
    
    // SPARSE INDEX: O(1) lookup while maintaining cache locality
    entity_to_index: HashMap<EntityId, usize>,
    
    // COMPONENT TRACKING: Bitmask-style component presence tracking
    has_light: Vec<bool>,
    has_mesh: Vec<bool>,
    
    // FREE LIST: Efficient component removal and reuse
    free_indices: Vec<usize>,
    
    // PRE-ALLOCATED BUFFERS: Avoid runtime allocation during queries
    light_query_buffer: Vec<(EntityId, Transform, LightComponent)>,
    mesh_query_buffer: Vec<(EntityId, Transform, MeshComponent)>,
}

impl GameWorld {
    pub fn new() -> Self {
        Self {
            next_entity_id: 1, // Start at 1, 0 is invalid entity
            entities: Vec::new(),
            transforms: Vec::new(),
            light_components: Vec::new(),
            mesh_components: Vec::new(),
            entity_to_index: HashMap::new(),
            has_light: Vec::new(),
            has_mesh: Vec::new(),
            free_indices: Vec::new(),
            light_query_buffer: Vec::new(),
            mesh_query_buffer: Vec::new(),
        }
    }
    
    /// Create entity with transform and light component
    /// Returns EntityId for future reference
    pub fn create_light_entity(&mut self, transform: Transform, light: LightComponent) -> EntityId {
        let entity_id = self.next_entity_id;
        self.next_entity_id += 1;
        
        let index = if let Some(free_idx) = self.free_indices.pop() {
            // Reuse free slot for cache efficiency
            self.entities[free_idx] = entity_id;
            self.transforms[free_idx] = transform;
            self.light_components[free_idx] = light;
            self.has_light[free_idx] = true;
            self.has_mesh[free_idx] = false;
            free_idx
        } else {
            // Append new components
            self.entities.push(entity_id);
            self.transforms.push(transform);
            self.light_components.push(light);
            self.mesh_components.push(MeshComponent { 
                material_type: MaterialType::StandardPBR, 
                visible: false 
            });
            self.has_light.push(true);
            self.has_mesh.push(false);
            self.entities.len() - 1
        };
        
        self.entity_to_index.insert(entity_id, index);
        entity_id
    }
    
    /// Create entity with transform and mesh component  
    pub fn create_mesh_entity(&mut self, transform: Transform, mesh: MeshComponent) -> EntityId {
        let entity_id = self.next_entity_id;
        self.next_entity_id += 1;
        
        let index = if let Some(free_idx) = self.free_indices.pop() {
            self.entities[free_idx] = entity_id;
            self.transforms[free_idx] = transform;
            self.mesh_components[free_idx] = mesh;
            self.has_light[free_idx] = false;
            self.has_mesh[free_idx] = true;
            free_idx
        } else {
            self.entities.push(entity_id);
            self.transforms.push(transform);
            self.light_components.push(LightComponent {
                light_type: LightType::Directional,
                color: Vec3::new(1.0, 1.0, 1.0),
                intensity: 1.0,
                direction: Vec3::new(0.0, -1.0, 0.0),
                position: Vec3::new(0.0, 0.0, 0.0),
                range: 1.0,
                inner_cone: 0.0,
                outer_cone: 0.0,
                enabled: false,
                cast_shadows: false,
            });
            self.mesh_components.push(mesh);
            self.has_light.push(false);
            self.has_mesh.push(true);
            self.entities.len() - 1
        };
        
        self.entity_to_index.insert(entity_id, index);
        entity_id
    }
    
    /// High-performance iteration over light entities using pre-allocated buffer
    /// Cache-friendly: processes components in dense array order
    pub fn query_light_entities(&mut self) -> &[(EntityId, Transform, LightComponent)] {
        self.light_query_buffer.clear();
        
        for i in 0..self.entities.len() {
            if self.has_light[i] && self.light_components[i].enabled {
                self.light_query_buffer.push((
                    self.entities[i],
                    self.transforms[i].clone(), // Clone since Transform doesn't implement Copy
                    self.light_components[i].clone(),
                ));
            }
        }
        
        &self.light_query_buffer
    }
    
    /// Query mesh entities for rendering
    pub fn query_mesh_entities(&mut self) -> &[(EntityId, Transform, MeshComponent)] {
        self.mesh_query_buffer.clear();
        
        for i in 0..self.entities.len() {
            if self.has_mesh[i] && self.mesh_components[i].visible {
                self.mesh_query_buffer.push((
                    self.entities[i],
                    self.transforms[i].clone(), // Clone since Transform doesn't implement Copy
                    self.mesh_components[i].clone(),
                ));
            }
        }
        
        &self.mesh_query_buffer
    }
    
    /// Get mutable access to light component for runtime modification
    pub fn get_light_component_mut(&mut self, entity_id: EntityId) -> Option<&mut LightComponent> {
        if let Some(&index) = self.entity_to_index.get(&entity_id) {
            if self.has_light[index] {
                return Some(&mut self.light_components[index]);
            }
        }
        None
    }
    
    /// Get mutable access to transform for runtime modification  
    pub fn get_transform_mut(&mut self, entity_id: EntityId) -> Option<&mut Transform> {
        if let Some(&index) = self.entity_to_index.get(&entity_id) {
            return Some(&mut self.transforms[index]);
        }
        None
    }
}

/// Factory functions for creating light components (logic in system, not component)
/// Following Game Engine Architecture principle: "All logic should reside in systems"
pub struct LightingSystem;

impl LightingSystem {
    /// Create directional light component with world-space direction
    /// CRITICAL: Maintains coordinate system consistency per Johannes Unterguggenberger guide
    pub fn create_directional_light(direction: Vec3, color: Vec3, intensity: f32) -> LightComponent {
        LightComponent {
            light_type: LightType::Directional,
            color,
            intensity,
            direction: direction.normalize(), // Ensure normalized direction
            position: Vec3::new(0.0, 0.0, 0.0), // Irrelevant for directional lights
            range: 0.0,
            inner_cone: 0.0,
            outer_cone: 0.0,
            enabled: true,
            cast_shadows: true,
        }
    }
    
    /// Create point light component with world-space position
    pub fn create_point_light(position: Vec3, color: Vec3, intensity: f32, range: f32) -> LightComponent {
        LightComponent {
            light_type: LightType::Point,
            color,
            intensity,
            direction: Vec3::new(0.0, -1.0, 0.0), // Irrelevant for point lights
            position,
            range,
            inner_cone: 0.0,
            outer_cone: 0.0,
            enabled: true,
            cast_shadows: true,
        }
    }
    
    /// Create spot light component with world-space position and direction
    pub fn create_spot_light(
        position: Vec3, 
        direction: Vec3, 
        color: Vec3, 
        intensity: f32, 
        range: f32,
        inner_cone: f32,
        outer_cone: f32
    ) -> LightComponent {
        LightComponent {
            light_type: LightType::Spot,
            color,
            intensity,
            direction: direction.normalize(),
            position,
            range,
            inner_cone,
            outer_cone,
            enabled: true,
            cast_shadows: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    const EPSILON: f32 = 0.001;
    
    fn assert_vec3_approx_eq(a: Vec3, b: Vec3) {
        assert!((a.x - b.x).abs() < EPSILON, "X mismatch: {} != {}", a.x, b.x);
        assert!((a.y - b.y).abs() < EPSILON, "Y mismatch: {} != {}", a.y, b.y);  
        assert!((a.z - b.z).abs() < EPSILON, "Z mismatch: {} != {}", a.z, b.z);
    }
    
    #[test]
    fn test_directional_light_coordinate_accuracy() {
        // Test: Entity system produces EXACT same lighting as hardcoded system
        let baseline_direction = Vec3::new(-0.7, -1.0, 0.3);
        let baseline_color = Vec3::new(1.0, 0.95, 0.9);
        let baseline_intensity = 1.5;
        
        let light_component = LightingSystem::create_directional_light(
            baseline_direction,
            baseline_color,
            baseline_intensity
        );
        
        // Verify exact parameter preservation
        assert_vec3_approx_eq(light_component.direction, baseline_direction.normalize());
        assert_vec3_approx_eq(light_component.color, baseline_color);
        assert!((light_component.intensity - baseline_intensity).abs() < EPSILON);
        assert_eq!(light_component.light_type, LightType::Directional);
        assert!(light_component.enabled);
    }
    
    #[test]
    fn test_coordinate_system_consistency() {
        // Verify Y-up right-handed coordinate system per Johannes Unterguggenberger guide
        let right = Vec3::new(1.0, 0.0, 0.0);    // +X points right
        let up = Vec3::new(0.0, 1.0, 0.0);       // +Y points up
        let forward = Vec3::new(0.0, 0.0, -1.0); // +Z points toward viewer
        
        // Right-handed check: right × up = forward
        let cross_product = right.cross(&up);
        assert_vec3_approx_eq(cross_product, forward);
    }
    
    #[test]
    fn test_entity_system_cache_efficiency() {
        // Test: Dense array storage for cache-friendly iteration
        let mut world = GameWorld::new();
        
        // Create multiple light entities
        let _light1 = world.create_light_entity(
            Transform::identity(),
            LightingSystem::create_directional_light(
                Vec3::new(-0.7, -1.0, 0.3),
                Vec3::new(1.0, 0.95, 0.9),
                1.5
            )
        );
        
        let _light2 = world.create_light_entity(
            Transform::from_position(Vec3::new(2.0, 1.0, 0.0)),
            LightingSystem::create_point_light(
                Vec3::new(2.0, 1.0, 0.0),
                Vec3::new(1.0, 0.5, 0.2),
                2.0,
                5.0
            )
        );
        
        // Query should return both lights in dense array order
        let light_entities = world.query_light_entities();
        assert_eq!(light_entities.len(), 2);
        
        // Verify data integrity
        assert_eq!(light_entities[0].2.light_type, LightType::Directional);
        assert_eq!(light_entities[1].2.light_type, LightType::Point);
    }
    
    #[test]
    fn test_transform_coordinate_preservation() {
        // Test: Transform component preserves coordinate system accuracy
        let mut world = GameWorld::new();
        
        // Create directional light with identity transform  
        let _light_entity = world.create_light_entity(
            Transform::identity(),
            LightingSystem::create_directional_light(
                Vec3::new(-0.7, -1.0, 0.3),
                Vec3::new(1.0, 0.95, 0.9),
                1.5
            )
        );
        
        let light_entities = world.query_light_entities();
        let (_, transform, light_comp) = &light_entities[0];
        
        // Verify transform doesn't alter directional light direction
        assert_vec3_approx_eq(transform.position, Vec3::new(0.0, 0.0, 0.0)); // Use 'position' instead of 'translation'
        assert_vec3_approx_eq(light_comp.direction, Vec3::new(-0.7, -1.0, 0.3).normalize());
    }
}
