// src/render/lighting.rs
use crate::util::math::Vec3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LightType {
    Directional,
    Point,
    Spot,
}

#[derive(Debug, Clone)]
pub struct Light {
    pub light_type: LightType,
    pub position: Vec3,
    pub direction: Vec3,
    pub color: Vec3,
    pub intensity: f32,
    pub range: f32, // For point/spot lights
    pub inner_cone_angle: f32, // For spot lights
    pub outer_cone_angle: f32, // For spot lights
    pub enabled: bool,
}

impl Light {
    pub fn directional(direction: Vec3, color: Vec3, intensity: f32) -> Self {
        Self {
            light_type: LightType::Directional,
            position: Vec3::zero(),
            direction: direction.normalize(),
            color,
            intensity,
            range: 0.0,
            inner_cone_angle: 0.0,
            outer_cone_angle: 0.0,
            enabled: true,
        }
    }

    pub fn point(position: Vec3, color: Vec3, intensity: f32, range: f32) -> Self {
        Self {
            light_type: LightType::Point,
            position,
            direction: Vec3::zero(),
            color,
            intensity,
            range,
            inner_cone_angle: 0.0,
            outer_cone_angle: 0.0,
            enabled: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LightingEnvironment {
    pub ambient_color: Vec3,
    pub ambient_intensity: f32,
    pub lights: Vec<Light>,
    pub max_lights: usize,
}

impl LightingEnvironment {
    pub fn new() -> Self {
        Self {
            ambient_color: Vec3::new(0.1, 0.1, 0.1), // Dark gray ambient
            ambient_intensity: 0.2,
            lights: Vec::new(),
            max_lights: 8, // Typical limit for real-time rendering
        }
    }

    pub fn set_ambient(&mut self, color: Vec3, intensity: f32) {
        self.ambient_color = color;
        self.ambient_intensity = intensity.max(0.0);
    }

    pub fn add_light(&mut self, light: Light) -> Option<usize> {
        if self.lights.len() < self.max_lights {
            let id = self.lights.len();
            self.lights.push(light);
            Some(id)
        } else {
            None // Too many lights
        }
    }
    
    // Create indoor warm lighting
    pub fn indoor_warm() -> Self {
        let mut env = Self::new();
        
        // Overhead warm light
        let ceiling_light = Light::point(
            Vec3::new(0.0, 3.0, 0.0),
            Vec3::new(1.0, 0.9, 0.8), // Warm white
            2.0,
            10.0,
        );
        env.add_light(ceiling_light);
        
        // Warm ambient
        env.set_ambient(Vec3::new(0.8, 0.7, 0.6), 0.1);
        
        env
    }
}
