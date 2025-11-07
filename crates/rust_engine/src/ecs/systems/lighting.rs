//! Lighting system for converting ECS entities to render data
//!
//! Following Game Engine Architecture principles: systems contain logic, components contain data

use crate::ecs::{World, LightComponent, LightType};
use crate::ecs::components::TransformComponent;
use crate::render::systems::lighting::{
    MultiLightEnvironment, DirectionalLightData, PointLightData, SpotLightData,
    MAX_DIRECTIONAL_LIGHTS, MAX_POINT_LIGHTS, MAX_SPOT_LIGHTS
};
use crate::foundation::math::Vec3;

/// Lighting system that processes light entities and produces rendering data
pub struct LightingSystem {
    /// Cached multi-light environment to avoid allocations
    cached_environment: MultiLightEnvironment,
}

impl LightingSystem {
    /// Creates a new lighting system with empty cached environment
    pub fn new() -> Self {
        Self {
            cached_environment: MultiLightEnvironment::new(),
        }
    }
    
    /// Extract lighting data from ECS world and build MultiLightEnvironment
    /// CRITICAL: Must produce identical lighting to hardcoded system for validation
    pub fn build_multi_light_environment(
        &mut self, 
        world: &World,
        ambient_color: Vec3,
        ambient_intensity: f32
    ) -> &MultiLightEnvironment {

        // Reset environment
        self.cached_environment = MultiLightEnvironment::new();
        
        // Set ambient lighting
        self.cached_environment.header.ambient_color = [
            ambient_color.x, 
            ambient_color.y, 
            ambient_color.z, 
            ambient_intensity
        ];
        
        // Query all light entities
        let light_entities = world.query::<LightComponent>();
        let transform_entities = world.query::<TransformComponent>();
        
        // Build entity -> transform lookup for efficiency
        let mut entity_transforms = std::collections::HashMap::new();
        for (entity, transform) in transform_entities {
            entity_transforms.insert(entity.id(), transform);
        }
        
        // Process all light entities
        let mut dir_count = 0;
        let mut point_count = 0;
        let mut spot_count = 0;
        
        for (entity, light_comp) in light_entities {
            if !light_comp.enabled {
                continue; // Skip disabled lights
            }
            
            let transform = entity_transforms.get(&entity.id());
            
            match light_comp.light_type {
                LightType::Directional => {
                    if dir_count < MAX_DIRECTIONAL_LIGHTS {
                        // CRITICAL: Must preserve exact coordinate values for validation
                        log::trace!("LightingSystem: Directional light direction: {:?}, intensity: {}", 
                            light_comp.direction, light_comp.intensity);
                        
                        self.cached_environment.directional_lights[dir_count] = DirectionalLightData {
                            direction: [
                                light_comp.direction.x,
                                light_comp.direction.y,
                                light_comp.direction.z,
                                light_comp.intensity, // Intensity in W component
                            ],
                            color: [
                                light_comp.color.x,
                                light_comp.color.y,
                                light_comp.color.z,
                                0.0, // Padding
                            ],
                        };
                        dir_count += 1;
                    }
                }
                LightType::Point => {
                    log::trace!("LightingSystem: Processing point light #{}", point_count);
                    if point_count < MAX_POINT_LIGHTS {
                        // Use position from light component or transform
                        let position = if light_comp.position != Vec3::new(0.0, 0.0, 0.0) {
                            light_comp.position
                        } else if let Some(transform) = transform {
                            transform.position
                        } else {
                            Vec3::new(0.0, 0.0, 0.0)
                        };
                        
                        log::trace!("LightingSystem: Point light position: {:?}, color: {:?}, intensity: {}", 
                            position, light_comp.color, light_comp.intensity);
                        
                        self.cached_environment.point_lights[point_count] = PointLightData {
                            position: [
                                position.x,
                                position.y,
                                position.z,
                                light_comp.range,
                            ],
                            color: [
                                light_comp.color.x,
                                light_comp.color.y,
                                light_comp.color.z,
                                light_comp.intensity,
                            ],
                            attenuation: [1.0, 0.22, 0.20, 0.0], // Balanced attenuation for longer-range vibrant lights
                        };
                        point_count += 1;
                    }
                }
                LightType::Spot => {
                    if spot_count < MAX_SPOT_LIGHTS {
                        // Use position from light component or transform
                        let position = if light_comp.position != Vec3::new(0.0, 0.0, 0.0) {
                            light_comp.position
                        } else if let Some(transform) = transform {
                            transform.position
                        } else {
                            Vec3::new(0.0, 0.0, 0.0)
                        };
                        
                        self.cached_environment.spot_lights[spot_count] = SpotLightData {
                            position: [
                                position.x,
                                position.y,
                                position.z,
                                light_comp.range,
                            ],
                            direction: [
                                light_comp.direction.x,
                                light_comp.direction.y,
                                light_comp.direction.z,
                                light_comp.intensity,
                            ],
                            color: [
                                light_comp.color.x,
                                light_comp.color.y,
                                light_comp.color.z,
                                0.0,
                            ],
                            cone_angles: [
                                light_comp.inner_cone,
                                light_comp.outer_cone,
                                0.0,
                                0.0,
                            ],
                        };
                        spot_count += 1;
                    }
                }
            }
        }
        
        // Set light counts
        self.cached_environment.header.directional_light_count = dir_count as u32;
        self.cached_environment.header.point_light_count = point_count as u32;
        self.cached_environment.header.spot_light_count = spot_count as u32;
        
        log::trace!("LightingSystem: Final counts - Dir: {}, Point: {}, Spot: {}", 
            dir_count, point_count, spot_count);
        
        &self.cached_environment
    }
}

impl Default for LightingSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::LightFactory;
    use crate::foundation::math::Transform;
    
    const EPSILON: f32 = 0.001;
    
    fn assert_vec3_approx_eq(a: [f32; 3], b: Vec3) {
        assert!((a[0] - b.x).abs() < EPSILON, "X mismatch: {} != {}", a[0], b.x);
        assert!((a[1] - b.y).abs() < EPSILON, "Y mismatch: {} != {}", a[1], b.y);  
        assert!((a[2] - b.z).abs() < EPSILON, "Z mismatch: {} != {}", a[2], b.z);
    }
    
    #[test]
    fn test_ecs_lighting_system_accuracy() {
        // Test: ECS lighting system produces EXACT same output as hardcoded system
        let mut world = World::new();
        let mut lighting_system = LightingSystem::new();
        
        // Create light entity with EXACT baseline parameters
        let entity = world.create_entity();
        let baseline_direction = Vec3::new(-0.7, -1.0, 0.3);
        let baseline_color = Vec3::new(1.0, 0.95, 0.9);
        let baseline_intensity = 1.5;
        
        world.add_component(entity, Transform::identity());
        world.add_component(entity, LightFactory::directional(
            baseline_direction,
            baseline_color,
            baseline_intensity
        ));
        
        // Build multi-light environment
        let multi_light_env = lighting_system.build_multi_light_environment(
            &world,
            Vec3::new(0.15, 0.12, 0.18), // Ambient color
            0.1                           // Ambient intensity
        );
        
        // Validate exact parameter preservation
        assert_eq!(multi_light_env.header.directional_light_count, 1);
        assert_eq!(multi_light_env.header.point_light_count, 0);
        assert_eq!(multi_light_env.header.spot_light_count, 0);
        
        let dir_light = &multi_light_env.directional_lights[0];
        assert_vec3_approx_eq(
            [dir_light.direction[0], dir_light.direction[1], dir_light.direction[2]], 
            baseline_direction.normalize()
        );
        assert_vec3_approx_eq(
            [dir_light.color[0], dir_light.color[1], dir_light.color[2]], 
            baseline_color
        );
        assert!((dir_light.direction[3] - baseline_intensity).abs() < EPSILON);
        
        // Validate ambient
        assert!((multi_light_env.header.ambient_color[0] - 0.15).abs() < EPSILON);
        assert!((multi_light_env.header.ambient_color[1] - 0.12).abs() < EPSILON);
        assert!((multi_light_env.header.ambient_color[2] - 0.18).abs() < EPSILON);
        assert!((multi_light_env.header.ambient_color[3] - 0.1).abs() < EPSILON);
    }
}
