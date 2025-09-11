//! Simple integration tests for coordinate system validation with ECS
//! 
//! Basic tests of ECS workflow with Transform and Light components

use crate::ecs::{World};
use crate::ecs::components::{TransformComponent, LightComponent, TransformFactory, LightFactory};
use crate::ecs::systems::{LightingSystem, CoordinateSystemValidator};
use crate::foundation::math::Vec3;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_basic_ecs_coordinate_integration() {
        let mut world = World::new();
        
        // Create entities with Transform and Light components
        let directional_entity = world.create_entity();
        
        // Add components using our factories
        let directional_direction = Vec3::new(-0.7, -1.0, 0.3); // From our validation
        let directional_transform = TransformFactory::directional_light(directional_direction);
        let directional_light = LightFactory::directional(
            directional_direction,
            Vec3::new(1.0, 1.0, 1.0),
            1.0
        );
        
        // Add components to entities
        world.add_component(directional_entity, directional_transform);
        world.add_component(directional_entity, directional_light);
        
        // Test that we can query both component types
        let transforms: Vec<_> = world.query::<TransformComponent>();
        let lights: Vec<_> = world.query::<LightComponent>();
        
        assert_eq!(transforms.len(), 1);
        assert_eq!(lights.len(), 1);
    }
    
    #[test]
    fn test_lighting_system_basic_validation() {
        let mut world = World::new();
        let mut lighting_system = LightingSystem::new();
        
        // Create a directional light with known direction
        let entity = world.create_entity();
        let direction = Vec3::new(-0.7, -1.0, 0.3);
        
        let transform = TransformFactory::directional_light(direction);
        let light = LightFactory::directional(direction, Vec3::new(1.0, 1.0, 1.0), 1.0);
        
        world.add_component(entity, transform);
        world.add_component(entity, light);
        
        // Build multi-light environment
        let multi_light_env = lighting_system.build_multi_light_environment(
            &world,
            Vec3::new(0.1, 0.1, 0.1), // ambient color
            0.3                        // ambient intensity
        );
        
        // Validate that exactly one directional light is found
        assert_eq!(multi_light_env.header.directional_light_count, 1);
    }
    
    #[test]
    fn test_coordinate_validation_suite() {
        // Run the basic validation suite
        let results = CoordinateSystemValidator::run_basic_validations();
        
        for (i, result) in results.iter().enumerate() {
            if let Err(msg) = result {
                panic!("Coordinate validation test {} failed: {}", i + 1, msg);
            }
        }
        
        println!("✓ All basic coordinate system validations passed");
    }
    
    #[test]
    fn test_transform_component_basic_roundtrip() {
        let mut world = World::new();
        
        // Test basic transform configuration
        let original_transform = TransformFactory::directional_light(Vec3::new(-0.7, -1.0, 0.3));
        
        let entity = world.create_entity();
        world.add_component(entity, original_transform.clone());
        
        // Retrieve and verify
        let retrieved = world.get_component::<TransformComponent>(entity).unwrap();
        
        // Position should match
        assert_eq!(retrieved.position, original_transform.position);
        assert_eq!(retrieved.scale, original_transform.scale);
    }
}
