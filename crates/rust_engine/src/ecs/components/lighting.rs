//! Lighting component for ECS
//! 
//! Pure data component following Game Engine Architecture principles:
//! - Components contain only data, no logic
//! - All logic resides in systems
//! - Cache-friendly storage via ECS World

use crate::foundation::math::Vec3;
use crate::ecs::Component;

/// Pure data component for lights - no logic, only data
/// Following Game Engine Architecture principle: "Components should be pure data containers"
#[derive(Debug, Clone)]
pub struct LightComponent {
    /// The type of light (directional, point, or spot)
    pub light_type: LightType,
    /// RGB color values for the light (0.0 to 1.0 range)
    pub color: Vec3,
    /// Light intensity multiplier (0.0 = no light, 1.0 = full intensity)
    pub intensity: f32,
    /// Direction vector for directional/spot lights in world space
    pub direction: Vec3,      // For directional/spot lights (world space)
    /// Position for point/spot lights in world space
    pub position: Vec3,       // For point/spot lights (world space)
    /// Maximum range/distance for point/spot lights
    pub range: f32,           // For point/spot lights
    /// Inner cone angle for spot lights in radians
    pub inner_cone: f32,      // For spot lights (radians)
    /// Outer cone angle for spot lights in radians
    pub outer_cone: f32,      // For spot lights (radians)
    /// Whether the light is currently enabled/active
    pub enabled: bool,
    /// Whether this light should cast shadows
    pub cast_shadows: bool,
}

/// Types of lights supported by the lighting system
#[derive(Debug, Clone, PartialEq)]
pub enum LightType {
    /// Directional light (like sunlight) with parallel rays
    Directional,
    /// Point light that radiates in all directions from a position
    Point,
    /// Spot light that creates a cone of light from a position
    Spot,
}

impl Component for LightComponent {}

/// Factory functions for creating light components (logic in system, not component)
/// Following Game Engine Architecture principle: "All logic should reside in systems"
pub struct LightFactory;

impl LightFactory {
    /// Create directional light component with world-space direction
    /// CRITICAL: Maintains coordinate system consistency per Johannes Unterguggenberger guide
    pub fn directional(direction: Vec3, color: Vec3, intensity: f32) -> LightComponent {
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
    pub fn point(position: Vec3, color: Vec3, intensity: f32, range: f32) -> LightComponent {
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
    pub fn spot(
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
        // Test: Engine ECS produces EXACT same lighting as hardcoded system
        let baseline_direction = Vec3::new(-0.7, -1.0, 0.3);
        let baseline_color = Vec3::new(1.0, 0.95, 0.9);
        let baseline_intensity = 1.5;
        
        let light_component = LightFactory::directional(
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
        let forward = Vec3::new(0.0, 0.0, 1.0);  // +Z points toward viewer (right-handed)
        
        // Right-handed check: right × up = forward
        let cross_product = right.cross(&up);
        assert_vec3_approx_eq(cross_product, forward);
    }
}
