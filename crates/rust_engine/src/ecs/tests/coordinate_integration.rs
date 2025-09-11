//! Integration tests for coordinate system validation with ECS
//! 
//! Tests complete ECS workflows with Transform and Light components
//! following Johannes Unterguggenberger guide and Game Engine Architecture

use crate::ecs::{World, Entity};
use crate::ecs::components::{TransformComponent, LightComponent, LightType, TransformFactory, LightFactory};
use crate::ecs::systems::{LightingSystem, CoordinateSystemValidator};
use crate::foundation::math::{Vec3, Transform};
use approx::assert_relative_eq;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ecs_coordinate_integration() {
        let mut world = World::new();
        
        // Create entities with Transform and Light components
        let directional_entity = world.create_entity();
        let point_entity = world.create_entity();
        
        // Add components using our factories
        let directional_direction = Vec3::new(-0.7, -1.0, 0.3); // From our validation
        let directional_transform = TransformFactory::directional_light(directional_direction);
        let directional_light = LightFactory::directional_light(
            directional_direction,
            Vec3::new(1.0, 1.0, 1.0),
            1.0
        );
        
        let point_position = Vec3::new(2.0, 3.0, -1.0);
        let point_transform = TransformFactory::point_light(point_position);
        let point_light = LightFactory::point_light(
            point_position,
            Vec3::new(1.0, 0.8, 0.6),
            1.5,
            10.0
        );
        
        // Add components to entities
        world.add_component(directional_entity, directional_transform);
        world.add_component(directional_entity, directional_light);
        world.add_component(point_entity, point_transform);
        world.add_component(point_entity, point_light);
        
        // Test that we can query both component types
        let transforms: Vec<_> = world.query::<TransformComponent>();
        let lights: Vec<_> = world.query::<LightComponent>();
        
        assert_eq!(transforms.len(), 2);
        assert_eq!(lights.len(), 2);
        
        // Verify coordinate consistency between components
        let directional_transform_comp = world.get_component::<TransformComponent>(directional_entity).unwrap();
        let directional_light_comp = world.get_component::<LightComponent>(directional_entity).unwrap();
        
        // Transform rotation should align with light direction
        let math_transform = directional_transform_comp.to_math_transform();
        let forward_from_transform = math_transform.rotation * Vec3::new(0.0, 0.0, -1.0);
        
        if let LightType::Directional { direction, .. } = &directional_light_comp.light_type {
            assert_relative_eq!(forward_from_transform, *direction, epsilon = 1e-5);
        } else {
            panic!("Expected directional light");
        }
        
        // Point light position should match transform position
        let point_transform_comp = world.get_component::<TransformComponent>(point_entity).unwrap();
        let point_light_comp = world.get_component::<LightComponent>(point_entity).unwrap();
        
        if let LightType::Point { position, .. } = &point_light_comp.light_type {
            assert_relative_eq!(point_transform_comp.position, *position, epsilon = 1e-6);
        } else {
            panic!("Expected point light");
        }
    }
    
    #[test]
    fn test_lighting_system_coordinate_validation() {
        let mut world = World::new();
        let lighting_system = LightingSystem::new();
        
        // Create multiple lights with different configurations
        let lights_config = vec![
            (Vec3::new(-0.7, -1.0, 0.3), Vec3::new(1.0, 1.0, 1.0)),  // Our validation direction
            (Vec3::new(0.0, -1.0, 0.0), Vec3::new(1.0, 0.9, 0.8)),   // Straight down
            (Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.8, 0.8, 1.0)),    // From the right
        ];
        
        for (direction, color) in lights_config {
            let entity = world.create_entity();
            
            let transform = TransformFactory::directional_light(direction);
            let light = LightFactory::directional_light(direction, color, 1.0);
            
            world.add_component(entity, transform);
            world.add_component(entity, light);
        }
        
        // Build multi-light environment
        let multi_light_env = lighting_system.build_multi_light_environment(&world);
        
        // Validate that all lights are present and coordinates are preserved
        assert_eq!(multi_light_env.directional_lights.len(), 3);
        
        // Check that our specific validation direction is preserved
        let validation_direction = Vec3::new(-0.7, -1.0, 0.3).normalize();
        let found_validation_light = multi_light_env.directional_lights.iter()
            .any(|light| (light.direction - validation_direction).magnitude() < 1e-5);
        
        assert!(found_validation_light, "Validation direction not preserved in lighting system");
    }
    
    #[test]
    fn test_coordinate_validation_suite_integration() {
        // Run the comprehensive validation suite
        let results = CoordinateSystemValidator::run_all_validations();
        
        for (i, result) in results.iter().enumerate() {
            if let Err(msg) = result {
                panic!("Coordinate validation test {} failed: {}", i + 1, msg);
            }
        }
        
        println!("✓ All coordinate system validations passed");
    }
    
    #[test]
    fn test_transform_component_ecs_roundtrip() {
        let mut world = World::new();
        
        // Test various transform configurations
        let test_transforms = vec![
            TransformFactory::directional_light(Vec3::new(-0.7, -1.0, 0.3)),
            TransformFactory::point_light(Vec3::new(1.0, 2.0, -3.0)),
            TransformFactory::look_at(
                Vec3::new(0.0, 0.0, 5.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0)
            ),
        ];
        
        let mut entities = Vec::new();
        
        // Add transforms to ECS
        for transform in &test_transforms {
            let entity = world.create_entity();
            world.add_component(entity, transform.clone());
            entities.push(entity);
        }
        
        // Retrieve and verify
        for (entity, original_transform) in entities.iter().zip(test_transforms.iter()) {
            let retrieved = world.get_component::<TransformComponent>(*entity).unwrap();
            
            assert_relative_eq!(retrieved.position, original_transform.position, epsilon = 1e-6);
            assert_relative_eq!(retrieved.scale, original_transform.scale, epsilon = 1e-6);
            
            // Quaternions may be flipped but represent same rotation
            let dot = original_transform.rotation.coords.dot(&retrieved.rotation.coords);
            assert!(dot.abs() > 0.999, "Quaternion mismatch for entity {:?}", entity);
        }
    }
    
    #[test]
    fn test_coordinate_preservation_through_systems() {
        let mut world = World::new();
        let lighting_system = LightingSystem::new();
        
        // Create light with our exact validation parameters
        let entity = world.create_entity();
        let direction = Vec3::new(-0.7, -1.0, 0.3);
        let color = Vec3::new(1.0, 1.0, 1.0);
        
        let transform = TransformFactory::directional_light(direction);
        let light = LightFactory::directional_light(direction, color, 1.0);
        
        world.add_component(entity, transform);
        world.add_component(entity, light);
        
        // Process through lighting system
        let multi_light_env = lighting_system.build_multi_light_environment(&world);
        
        // Verify exact parameter preservation
        assert_eq!(multi_light_env.directional_lights.len(), 1);
        
        let processed_light = &multi_light_env.directional_lights[0];
        let expected_direction = direction.normalize();
        
        // Coordinates should be preserved exactly within numerical precision
        assert!((processed_light.direction.x - expected_direction.x).abs() < 1e-6);
        assert!((processed_light.direction.y - expected_direction.y).abs() < 1e-6);
        assert!((processed_light.direction.z - expected_direction.z).abs() < 1e-6);
        
        // Color should also be preserved
        assert_relative_eq!(processed_light.color, color, epsilon = 1e-6);
    }
    
    #[test]
    fn test_matrix_chain_validation_with_ecs() {
        let mut world = World::new();
        
        // Create a transform that will go through the full matrix chain
        let entity = world.create_entity();
        let transform = TransformComponent::from_transform(
            Vec3::new(1.0, 2.0, -3.0),                          // World position
            crate::foundation::math::Transform::from_position_rotation(
                Vec3::zeros(),
                nalgebra::Quat::from_axis_angle(&Vec3::y_axis(), std::f32::consts::PI / 4.0)
            ).rotation,
            Vec3::new(2.0, 1.0, 1.0)                           // Scale
        );
        
        world.add_component(entity, transform.clone());
        
        // Convert to matrix and validate chain
        let math_transform = transform.to_math_transform();
        let model_matrix = math_transform.to_matrix();
        
        // Test vertex transformation
        let test_vertex = nalgebra::Point3::new(1.0, 0.0, 0.0);
        let transformed = model_matrix.transform_point(&test_vertex);
        
        // Manual calculation: scale * rotate * translate
        let scaled = nalgebra::Point3::new(2.0, 0.0, 0.0); // Scale X by 2
        let rotated = math_transform.rotation * scaled.coords;
        let final_pos = rotated + transform.position;
        
        assert_relative_eq!(transformed.coords, final_pos, epsilon = 1e-5);
    }
}
