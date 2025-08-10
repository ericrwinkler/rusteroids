//! Lighting system

use crate::foundation::math::Vec3;

/// Light types
#[derive(Debug, Clone, Copy)]
pub enum LightType {
    /// Directional light (like sunlight)
    Directional,
    /// Point light (like a lightbulb)
    Point,
    /// Spot light (like a flashlight)
    Spot,
}

/// Light source
#[derive(Debug, Clone)]
pub struct Light {
    /// Light type
    pub light_type: LightType,
    /// Light position (for point/spot lights)
    pub position: Vec3,
    /// Light direction (for directional/spot lights)
    pub direction: Vec3,
    /// Light color
    pub color: Vec3,
    /// Light intensity
    pub intensity: f32,
    /// Light range (for point/spot lights)
    pub range: f32,
    /// Inner cone angle for spot lights (in radians)
    pub inner_cone_angle: f32,
    /// Outer cone angle for spot lights (in radians)
    pub outer_cone_angle: f32,
}

impl Light {
    /// Create a directional light
    pub fn directional(direction: Vec3, color: Vec3, intensity: f32) -> Self {
        Self {
            light_type: LightType::Directional,
            position: Vec3::zeros(),
            direction: direction.normalize(),
            color,
            intensity,
            range: 0.0,
            inner_cone_angle: 0.0,
            outer_cone_angle: 0.0,
        }
    }

    /// Create a point light
    pub fn point(position: Vec3, color: Vec3, intensity: f32, range: f32) -> Self {
        Self {
            light_type: LightType::Point,
            position,
            direction: Vec3::zeros(),
            color,
            intensity,
            range,
            inner_cone_angle: 0.0,
            outer_cone_angle: 0.0,
        }
    }

    /// Create a spot light
    pub fn spot(
        position: Vec3,
        direction: Vec3,
        color: Vec3,
        intensity: f32,
        range: f32,
        inner_cone_angle: f32,
        outer_cone_angle: f32,
    ) -> Self {
        Self {
            light_type: LightType::Spot,
            position,
            direction: direction.normalize(),
            color,
            intensity,
            range,
            inner_cone_angle,
            outer_cone_angle,
        }
    }
}

/// Lighting environment containing multiple lights
#[derive(Debug, Clone)]
pub struct LightingEnvironment {
    /// List of lights in the scene
    pub lights: Vec<Light>,
    /// Ambient light color
    pub ambient_color: Vec3,
    /// Ambient light intensity
    pub ambient_intensity: f32,
}

impl LightingEnvironment {
    /// Create a new empty lighting environment
    pub fn new() -> Self {
        Self {
            lights: Vec::new(),
            ambient_color: Vec3::new(1.0, 1.0, 1.0),
            ambient_intensity: 0.1,
        }
    }

    /// Add a light to the environment
    pub fn add_light(mut self, light: Light) -> Self {
        self.lights.push(light);
        self
    }

    /// Set ambient lighting
    pub fn with_ambient(mut self, color: Vec3, intensity: f32) -> Self {
        self.ambient_color = color;
        self.ambient_intensity = intensity;
        self
    }

    /// Create a warm indoor lighting environment
    pub fn indoor_warm() -> Self {
        Self::new()
            .with_ambient(Vec3::new(1.0, 0.9, 0.8), 0.2)
            .add_light(Light::directional(
                Vec3::new(-0.3, -1.0, -0.5),
                Vec3::new(1.0, 0.95, 0.8),
                0.8,
            ))
            .add_light(Light::point(
                Vec3::new(2.0, 3.0, 2.0),
                Vec3::new(1.0, 0.9, 0.7),
                1.0,
                10.0,
            ))
    }

    /// Create outdoor daylight environment
    pub fn outdoor_daylight() -> Self {
        Self::new()
            .with_ambient(Vec3::new(0.5, 0.7, 1.0), 0.3)
            .add_light(Light::directional(
                Vec3::new(-0.2, -1.0, -0.3),
                Vec3::new(1.0, 1.0, 0.9),
                1.0,
            ))
    }
}

impl Default for LightingEnvironment {
    fn default() -> Self {
        Self::new()
    }
}
