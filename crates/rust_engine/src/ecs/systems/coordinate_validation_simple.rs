//! Simple coordinate system validation tests
//! 
//! Basic tests for coordinate transformations following Johannes Unterguggenberger's guide

use crate::foundation::math::{Mat4Ext, Vec3, Mat4};

/// Simple coordinate system validator
pub struct CoordinateSystemValidator;

impl CoordinateSystemValidator {
    /// Validate right-handed coordinate system conventions
    pub fn validate_right_handed_system() -> Result<(), String> {
        // Standard right-handed basis vectors
        let x_axis = Vec3::new(1.0, 0.0, 0.0); // Right
        let y_axis = Vec3::new(0.0, 1.0, 0.0); // Up
        let z_axis = Vec3::new(0.0, 0.0, 1.0); // Forward (toward viewer)
        
        // Verify cross products follow right-hand rule
        let cross_xy = x_axis.cross(&y_axis);
        if (cross_xy - z_axis).magnitude() > 1e-6 {
            return Err(format!("Right-hand rule violation: X × Y = {:?}, expected Z = {:?}", cross_xy, z_axis));
        }
        
        Ok(())
    }
    
    /// Validate specific coordinate parameters from our system
    pub fn validate_coordinate_parameters() -> Result<(), String> {
        // Test the exact parameters from our lighting system
        let test_direction = Vec3::new(-0.7, -1.0, 0.3);
        let normalized = test_direction.normalize();
        
        if (normalized.magnitude() - 1.0).abs() > 1e-6 {
            return Err(format!("Normalization failed: magnitude = {}", normalized.magnitude()));
        }
        
        Ok(())
    }
    
    /// Validate perspective projection matrix structure
    pub fn validate_perspective_projection() -> Result<(), String> {
        let fov_y = std::f32::consts::PI / 4.0; // 45 degrees
        let aspect = 16.0 / 9.0;                // Widescreen aspect
        let near = 0.1;
        let far = 100.0;
        
        let projection = Mat4::perspective(fov_y, aspect, near, far);
        
        // Basic validation - matrix should have non-zero diagonal elements
        if projection[(0, 0)] == 0.0 || projection[(1, 1)] == 0.0 || projection[(2, 2)] == 0.0 {
            return Err("Projection matrix has zero diagonal elements".to_string());
        }
        
        Ok(())
    }
    
    /// Validate Vulkan coordinate transformation matrix
    pub fn validate_vulkan_coordinate_transform() -> Result<(), String> {
        let transform = Mat4::vulkan_coordinate_transform();
        
        // Should be identity with Y and Z flipped
        if transform[(0, 0)] != 1.0 || transform[(1, 1)] != -1.0 || transform[(2, 2)] != -1.0 || transform[(3, 3)] != 1.0 {
            return Err("Vulkan coordinate transform matrix incorrect".to_string());
        }
        
        Ok(())
    }
    
    /// Run basic coordinate validation tests
    pub fn run_basic_validations() -> Vec<Result<(), String>> {
        vec![
            Self::validate_right_handed_system(),
            Self::validate_coordinate_parameters(),
            Self::validate_perspective_projection(),
            Self::validate_vulkan_coordinate_transform(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_coordinate_system_validation() {
        let results = CoordinateSystemValidator::run_basic_validations();
        
        for (i, result) in results.iter().enumerate() {
            match result {
                Ok(()) => println!("✓ Basic validation test {} passed", i + 1),
                Err(msg) => panic!("✗ Basic validation test {} failed: {}", i + 1, msg),
            }
        }
    }
    
    #[test]
    fn test_parameter_preservation() {
        // Test the exact (-0.7, -1.0, 0.3) parameters used in our system
        let direction = Vec3::new(-0.7, -1.0, 0.3);
        let normalized = direction.normalize();
        
        // Verify specific coordinate preservation
        assert!((normalized.x.abs() - (0.7_f32 / direction.magnitude())).abs() < 1e-5);
        assert!((normalized.y.abs() - (1.0_f32 / direction.magnitude())).abs() < 1e-5);
        assert!((normalized.z.abs() - (0.3_f32 / direction.magnitude())).abs() < 1e-5);
    }
}
