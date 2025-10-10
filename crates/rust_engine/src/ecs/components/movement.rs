//! Movement component for entities that can move in 3D space
//!
//! This component provides velocity, acceleration, and rotational movement
//! for dynamic objects in the scene.

use crate::ecs::Component;
use crate::foundation::math::Vec3;

/// Component for entities that can move
#[derive(Debug, Clone)]
pub struct MovementComponent {
    /// Linear velocity in units per second
    pub velocity: Vec3,
    
    /// Linear acceleration in units per second squared
    pub acceleration: Vec3,
    
    /// Angular velocity in radians per second
    pub angular_velocity: Vec3,
    
    /// Angular acceleration in radians per second squared
    pub angular_acceleration: Vec3,
    
    /// Maximum speed limit (0 = no limit)
    pub max_speed: f32,
    
    /// Damping factor for velocity (0 = no damping, 1 = instant stop)
    pub linear_damping: f32,
    
    /// Damping factor for angular velocity
    pub angular_damping: f32,
    
    /// Whether movement is enabled
    pub enabled: bool,
}

impl MovementComponent {
    /// Create a new movement component
    pub fn new() -> Self {
        Self {
            velocity: Vec3::zeros(),
            acceleration: Vec3::zeros(),
            angular_velocity: Vec3::zeros(),
            angular_acceleration: Vec3::zeros(),
            max_speed: 0.0, // No limit by default
            linear_damping: 0.0,
            angular_damping: 0.0,
            enabled: true,
        }
    }
    
    /// Create a movement component with initial velocity
    pub fn with_velocity(velocity: Vec3) -> Self {
        Self {
            velocity,
            acceleration: Vec3::zeros(),
            angular_velocity: Vec3::zeros(),
            angular_acceleration: Vec3::zeros(),
            max_speed: 0.0,
            linear_damping: 0.0,
            angular_damping: 0.0,
            enabled: true,
        }
    }
    
    /// Create a movement component with rotation
    pub fn with_rotation(angular_velocity: Vec3) -> Self {
        Self {
            velocity: Vec3::zeros(),
            acceleration: Vec3::zeros(),
            angular_velocity,
            angular_acceleration: Vec3::zeros(),
            max_speed: 0.0,
            linear_damping: 0.0,
            angular_damping: 0.0,
            enabled: true,
        }
    }
    
    /// Set velocity
    pub fn set_velocity(&mut self, velocity: Vec3) {
        self.velocity = velocity;
    }
    
    /// Add to velocity
    pub fn add_velocity(&mut self, delta_velocity: Vec3) {
        self.velocity += delta_velocity;
    }
    
    /// Set acceleration
    pub fn set_acceleration(&mut self, acceleration: Vec3) {
        self.acceleration = acceleration;
    }
    
    /// Add to acceleration
    pub fn add_acceleration(&mut self, delta_acceleration: Vec3) {
        self.acceleration += delta_acceleration;
    }
    
    /// Set angular velocity
    pub fn set_angular_velocity(&mut self, angular_velocity: Vec3) {
        self.angular_velocity = angular_velocity;
    }
    
    /// Set maximum speed
    pub fn set_max_speed(&mut self, max_speed: f32) {
        self.max_speed = max_speed.max(0.0);
    }
    
    /// Set linear damping
    pub fn set_linear_damping(&mut self, damping: f32) {
        self.linear_damping = damping.clamp(0.0, 1.0);
    }
    
    /// Set angular damping
    pub fn set_angular_damping(&mut self, damping: f32) {
        self.angular_damping = damping.clamp(0.0, 1.0);
    }
    
    /// Enable or disable movement
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
    
    /// Apply physics integration step
    pub fn integrate(&mut self, delta_time: f32) {
        if !self.enabled {
            return;
        }
        
        // Integrate linear motion
        self.velocity += self.acceleration * delta_time;
        
        // Apply speed limit
        if self.max_speed > 0.0 {
            let speed = self.velocity.magnitude();
            if speed > self.max_speed {
                self.velocity = self.velocity.normalize() * self.max_speed;
            }
        }
        
        // Apply linear damping
        if self.linear_damping > 0.0 {
            self.velocity *= (1.0 - self.linear_damping * delta_time).max(0.0);
        }
        
        // Integrate angular motion
        self.angular_velocity += self.angular_acceleration * delta_time;
        
        // Apply angular damping
        if self.angular_damping > 0.0 {
            self.angular_velocity *= (1.0 - self.angular_damping * delta_time).max(0.0);
        }
    }
    
    /// Get position delta for this frame
    pub fn get_position_delta(&self, delta_time: f32) -> Vec3 {
        if !self.enabled {
            return Vec3::zeros();
        }
        self.velocity * delta_time
    }
    
    /// Get rotation delta for this frame
    pub fn get_rotation_delta(&self, delta_time: f32) -> Vec3 {
        if !self.enabled {
            return Vec3::zeros();
        }
        self.angular_velocity * delta_time
    }
    
    /// Stop all movement
    pub fn stop(&mut self) {
        self.velocity = Vec3::zeros();
        self.acceleration = Vec3::zeros();
        self.angular_velocity = Vec3::zeros();
        self.angular_acceleration = Vec3::zeros();
    }
}

impl Component for MovementComponent {}

impl Default for MovementComponent {
    fn default() -> Self {
        Self::new()
    }
}

/// Factory for creating movement components with common patterns
pub struct MovementFactory;

impl MovementFactory {
    /// Create a static object (no movement)
    pub fn create_static() -> MovementComponent {
        let mut movement = MovementComponent::new();
        movement.set_enabled(false);
        movement
    }
    
    /// Create a linearly moving object
    pub fn create_linear(velocity: Vec3, max_speed: f32) -> MovementComponent {
        let mut movement = MovementComponent::with_velocity(velocity);
        movement.set_max_speed(max_speed);
        movement
    }
    
    /// Create a rotating object
    pub fn create_rotating(angular_velocity: Vec3) -> MovementComponent {
        MovementComponent::with_rotation(angular_velocity)
    }
    
    /// Create an orbital movement pattern
    pub fn create_orbital(linear_velocity: Vec3, angular_velocity: Vec3) -> MovementComponent {
        MovementComponent {
            velocity: linear_velocity,
            acceleration: Vec3::zeros(),
            angular_velocity,
            angular_acceleration: Vec3::zeros(),
            max_speed: 0.0,
            linear_damping: 0.0,
            angular_damping: 0.0,
            enabled: true,
        }
    }
    
    /// Create movement with damping (for realistic physics)
    pub fn create_with_damping(
        velocity: Vec3, 
        linear_damping: f32, 
        angular_damping: f32
    ) -> MovementComponent {
        let mut movement = MovementComponent::with_velocity(velocity);
        movement.set_linear_damping(linear_damping);
        movement.set_angular_damping(angular_damping);
        movement
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_movement_component_creation() {
        let movement = MovementComponent::new();
        
        assert_eq!(movement.velocity, Vec3::zeros());
        assert_eq!(movement.acceleration, Vec3::zeros());
        assert_eq!(movement.angular_velocity, Vec3::zeros());
        assert!(movement.enabled);
        assert_eq!(movement.max_speed, 0.0);
    }

    #[test]
    fn test_velocity_operations() {
        let mut movement = MovementComponent::new();
        
        let velocity = Vec3::new(1.0, 2.0, 3.0);
        movement.set_velocity(velocity);
        assert_eq!(movement.velocity, velocity);
        
        let delta = Vec3::new(0.5, 0.5, 0.5);
        movement.add_velocity(delta);
        assert_eq!(movement.velocity, Vec3::new(1.5, 2.5, 3.5));
    }

    #[test]
    fn test_integration() {
        let mut movement = MovementComponent::new();
        movement.set_velocity(Vec3::new(1.0, 0.0, 0.0));
        movement.set_acceleration(Vec3::new(0.0, 1.0, 0.0));
        
        let delta_time = 0.1;
        movement.integrate(delta_time);
        
        // Velocity should include acceleration
        assert_eq!(movement.velocity, Vec3::new(1.0, 0.1, 0.0));
    }

    #[test]
    fn test_position_delta() {
        let mut movement = MovementComponent::new();
        movement.set_velocity(Vec3::new(2.0, 1.0, 0.5));
        
        let delta_time = 0.5;
        let position_delta = movement.get_position_delta(delta_time);
        
        assert_eq!(position_delta, Vec3::new(1.0, 0.5, 0.25));
    }

    #[test]
    fn test_max_speed_limit() {
        let mut movement = MovementComponent::new();
        movement.set_velocity(Vec3::new(10.0, 0.0, 0.0));
        movement.set_max_speed(5.0);
        
        movement.integrate(0.1);
        
        assert!(movement.velocity.magnitude() <= 5.0);
    }

    #[test]
    fn test_damping() {
        let mut movement = MovementComponent::new();
        movement.set_velocity(Vec3::new(1.0, 0.0, 0.0));
        movement.set_linear_damping(0.5);
        
        let initial_speed = movement.velocity.magnitude();
        movement.integrate(0.1);
        
        assert!(movement.velocity.magnitude() < initial_speed);
    }

    #[test]
    fn test_factory_methods() {
        // Test static creation
        let static_movement = MovementFactory::create_static();
        assert!(!static_movement.enabled);
        
        // Test linear creation
        let linear_movement = MovementFactory::create_linear(Vec3::new(1.0, 0.0, 0.0), 5.0);
        assert_eq!(linear_movement.velocity, Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(linear_movement.max_speed, 5.0);
        
        // Test rotating creation
        let rotating_movement = MovementFactory::create_rotating(Vec3::new(0.0, 1.0, 0.0));
        assert_eq!(rotating_movement.angular_velocity, Vec3::new(0.0, 1.0, 0.0));
    }

    #[test]
    fn test_stop() {
        let mut movement = MovementComponent::new();
        movement.set_velocity(Vec3::new(1.0, 2.0, 3.0));
        movement.set_acceleration(Vec3::new(0.5, 0.5, 0.5));
        movement.set_angular_velocity(Vec3::new(1.0, 0.0, 0.0));
        
        movement.stop();
        
        assert_eq!(movement.velocity, Vec3::zeros());
        assert_eq!(movement.acceleration, Vec3::zeros());
        assert_eq!(movement.angular_velocity, Vec3::zeros());
    }
}
