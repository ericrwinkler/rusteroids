//! Lighting system for realistic 3D scene illumination
//! 
//! This module provides a comprehensive lighting system supporting multiple light types
//! and complex lighting environments. Designed to work with PBR materials for
//! physically accurate lighting calculations.
//! 
//! # Architecture Assessment: GOOD DESIGN WITH ROOM FOR IMPROVEMENT
//! 
//! This module demonstrates solid architectural principles with backend-agnostic design
//! and a focus on lighting domain expertise. However, there are some areas where the
//! current implementation could be enhanced.
//! 
//! ## Architectural Strengths:
//! 
//! ### Backend Independence ✅
//! The lighting system contains no references to specific rendering backends. Light
//! definitions are pure mathematical constructs that can be interpreted by any renderer,
//! whether Vulkan, OpenGL, DirectX, or software rendering.
//! 
//! ### Comprehensive Light Types ✅
//! Supports the three fundamental light types needed for realistic 3D scenes:
//! - **Directional**: Parallel light rays (sun, distant light sources)
//! - **Point**: Omnidirectional light from a point (light bulbs, fires)
//! - **Spot**: Directional cone of light (flashlights, stage lighting)
//! 
//! ### Physically Meaningful Parameters ✅
//! Light properties correspond to real-world lighting concepts:
//! - Position and direction for spatial light placement
//! - Color and intensity for light appearance and brightness  
//! - Range for light attenuation and performance optimization
//! - Cone angles for spot light beam shaping
//! 
//! ### Lighting Environment Abstraction ✅
//! The `LightingEnvironment` provides a higher-level abstraction for managing
//! multiple lights and ambient lighting as a cohesive scene lighting setup.
//! 
//! ### Preset Lighting Configurations ✅
//! Includes realistic preset lighting environments (indoor warm, outdoor daylight)
//! that provide good starting points for different scene types.
//! 
//! ## Current Implementation Limitations:
//! 
//! ### Limited Shader Integration
//! **RENDERER COUPLING ISSUE**: The renderer currently only supports directional lights
//! in the shader push constants, despite this module defining point and spot lights.
//! This creates a mismatch between the high-level lighting API and actual rendering
//! capabilities.
//! 
//! **FIXME in renderer**: Extend shader system to support all defined light types
//! or clearly document which light types are supported by each rendering backend.
//! 
//! ### No Shadow Support
//! Current light definitions don't include shadow-related properties:
//! - Shadow casting enable/disable flags
//! - Shadow map resolution preferences
//! - Shadow bias and filtering parameters
//! 
//! ### Missing Advanced Lighting Features
//! - **Area lights**: For realistic soft shadows from extended light sources
//! - **Image-based lighting**: For realistic environment reflections
//! - **Light probes**: For baked global illumination
//! - **Volumetric lighting**: For atmospheric effects (fog, dust, god rays)
//! 
//! ## Performance and Optimization Opportunities:
//! 
//! ### Light Culling Support
//! Light definitions could include bounding volume information to support
//! efficient light culling in renderers:
//! ```rust
//! pub struct Light {
//!     // ... existing fields ...
//!     pub bounding_sphere: Option<BoundingSphere>, // For culling
//!     pub priority: u32, // For importance-based light selection
//! }
//! ```
//! 
//! ### GPU Layout Considerations
//! For GPU-based lighting calculations, consider memory layout optimization:
//! - Structure-of-arrays layout for better GPU cache usage
//! - Explicit padding for GPU alignment requirements
//! - Separate storage for frequently vs rarely changed light data
//! 
//! ### Dynamic Light Management
//! Large scenes need efficient dynamic light management:
//! - Spatial data structures for light queries
//! - Level-of-detail for distant lights
//! - Temporal stability for animated lights
//! 
//! ## Future Enhancement Areas:
//! 
//! ### Advanced Light Types
//! ```rust
//! pub enum LightType {
//!     Directional,
//!     Point,
//!     Spot,
//!     Area { width: f32, height: f32 },           // Soft area lighting
//!     Environment { texture: TextureHandle },      // IBL environment maps
//!     Probe { position: Vec3, influence: f32 },   // Light probes for GI
//! }
//! ```
//! 
//! ### Shadow Configuration
//! ```rust
//! pub struct ShadowSettings {
//!     pub cast_shadows: bool,
//!     pub shadow_resolution: u32,
//!     pub shadow_bias: f32,
//!     pub shadow_filter: ShadowFilter,
//! }
//! ```
//! 
//! ### Light Animation System
//! Support for animated lights with interpolation:
//! - Keyframe-based light animation
//! - Procedural light effects (flickering, pulsing)
//! - Light scripting for complex behaviors
//! 
//! ## Design Goals Assessment:
//! 
//! 1. ✅ **Backend Agnostic**: No rendering backend dependencies
//! 2. ✅ **Physically Accurate**: Real-world lighting parameters
//! 3. ✅ **Easy to Use**: Builder patterns and preset configurations
//! 4. ⚠️ **Feature Complete**: Missing some advanced lighting features
//! 5. ⚠️ **Performance Optimized**: Could benefit from GPU layout consideration
//! 
//! Overall, this is a solid foundation for a lighting system that follows good
//! architectural principles and can be extended to support advanced lighting features
//! as the rendering system evolves.

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

    /// Get direction as array for Vulkan push constants
    /// 
    /// Helper method to convert Vec3 direction to array format required by shaders.
    pub fn direction_array(&self) -> [f32; 3] {
        [self.direction.x, self.direction.y, self.direction.z]
    }

    /// Get color as array for Vulkan push constants
    /// 
    /// Helper method to convert Vec3 color to array format required by shaders.
    pub fn color_array(&self) -> [f32; 3] {
        [self.color.x, self.color.y, self.color.z]
    }

    /// Get position as array for Vulkan push constants
    /// 
    /// Helper method to convert Vec3 position to array format required by shaders.
    pub fn position_array(&self) -> [f32; 3] {
        [self.position.x, self.position.y, self.position.z]
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

// ================================================================================================
// MULTI-LIGHT UBO SYSTEM (Phase 1 - Step 1.1)
// ================================================================================================

/// Multi-light UBO header structure following Vulkano tutorial and std140 alignment
#[repr(C)]
#[derive(Debug, Clone)]
pub struct MultiLightUBO {
    /// Ambient light color and intensity [R, G, B, intensity]
    pub ambient_color: [f32; 4],
    /// Number of active directional lights
    pub directional_light_count: u32,
    /// Number of active point lights
    pub point_light_count: u32,
    /// Number of active spot lights
    pub spot_light_count: u32,
    /// Padding for std140 alignment
    pub _padding: u32,
}

/// Directional light data for GPU uniform buffer
#[repr(C)]
#[derive(Debug, Clone, Copy)]  
pub struct DirectionalLightData {
    /// Light direction and intensity [x, y, z, intensity]
    pub direction: [f32; 4],
    /// Light color [r, g, b, padding]
    pub color: [f32; 4],
}

/// Point light data for GPU uniform buffer
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PointLightData {
    /// Light position and range [x, y, z, range]
    pub position: [f32; 4],
    /// Light color and intensity [r, g, b, intensity]
    pub color: [f32; 4],
    /// Attenuation factors [constant, linear, quadratic, padding]
    pub attenuation: [f32; 4],
}

/// Spot light data for GPU uniform buffer
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SpotLightData {
    /// Light position and range [x, y, z, range]
    pub position: [f32; 4],
    /// Light direction and intensity [x, y, z, intensity]
    pub direction: [f32; 4],
    /// Light color [r, g, b, padding]
    pub color: [f32; 4],
    /// Cone angles [inner_angle, outer_angle, unused, unused]
    pub cone_angles: [f32; 4],
}

/// Maximum number of directional lights supported
pub const MAX_DIRECTIONAL_LIGHTS: usize = 4;
/// Maximum number of point lights supported
pub const MAX_POINT_LIGHTS: usize = 8;
/// Maximum number of spot lights supported
pub const MAX_SPOT_LIGHTS: usize = 4;

/// Multi-light environment structure containing all light types
#[derive(Debug, Clone)]
pub struct MultiLightEnvironment {
    /// UBO header with ambient lighting and light counts
    pub header: MultiLightUBO,
    /// Array of directional light data
    pub directional_lights: [DirectionalLightData; MAX_DIRECTIONAL_LIGHTS],
    /// Array of point light data
    pub point_lights: [PointLightData; MAX_POINT_LIGHTS], 
    /// Array of spot light data
    pub spot_lights: [SpotLightData; MAX_SPOT_LIGHTS],
}

impl MultiLightEnvironment {
    /// Create new empty multi-light environment
    pub fn new() -> Self {
        Self {
            header: MultiLightUBO {
                ambient_color: [0.1, 0.1, 0.1, 0.1], // Default low ambient
                directional_light_count: 0,
                point_light_count: 0,
                spot_light_count: 0,
                _padding: 0,
            },
            directional_lights: [DirectionalLightData { 
                direction: [0.0, 0.0, 0.0, 0.0], 
                color: [0.0, 0.0, 0.0, 0.0] 
            }; MAX_DIRECTIONAL_LIGHTS],
            point_lights: [PointLightData { 
                position: [0.0, 0.0, 0.0, 0.0], 
                color: [0.0, 0.0, 0.0, 0.0], 
                attenuation: [1.0, 0.0, 0.0, 0.0] 
            }; MAX_POINT_LIGHTS],
            spot_lights: [SpotLightData { 
                position: [0.0, 0.0, 0.0, 0.0], 
                direction: [0.0, 0.0, 0.0, 0.0], 
                color: [0.0, 0.0, 0.0, 0.0], 
                cone_angles: [0.0, 0.0, 0.0, 0.0] 
            }; MAX_SPOT_LIGHTS],
        }
    }

    /// Convert from legacy LightingEnvironment to MultiLightEnvironment
    /// This function preserves exact lighting behavior for visual validation
    pub fn from_legacy_lighting_environment(legacy_env: &LightingEnvironment) -> Self {
        let mut multi_env = Self::new();
        
        // Set ambient lighting
        multi_env.header.ambient_color = [
            legacy_env.ambient_color.x,
            legacy_env.ambient_color.y, 
            legacy_env.ambient_color.z,
            legacy_env.ambient_intensity,
        ];
        
        // Convert lights (preserving exact coordinate values)
        let mut dir_count = 0;
        let mut point_count = 0;
        let mut spot_count = 0;
        
        for light in &legacy_env.lights {
            match light.light_type {
                LightType::Directional => {
                    if dir_count < MAX_DIRECTIONAL_LIGHTS {
                        multi_env.directional_lights[dir_count] = DirectionalLightData {
                            // CRITICAL: Preserve exact direction values for visual validation
                            direction: [
                                light.direction.x,
                                light.direction.y,
                                light.direction.z,
                                light.intensity,  // Store intensity in W component
                            ],
                            color: [
                                light.color.x,
                                light.color.y,
                                light.color.z,
                                0.0,  // Padding
                            ],
                        };
                        dir_count += 1;
                    }
                }
                LightType::Point => {
                    if point_count < MAX_POINT_LIGHTS {
                        multi_env.point_lights[point_count] = PointLightData {
                            position: [
                                light.position.x,
                                light.position.y,
                                light.position.z,
                                light.range,
                            ],
                            color: [
                                light.color.x,
                                light.color.y,
                                light.color.z,
                                light.intensity,
                            ],
                            attenuation: [1.0, 0.09, 0.032, 0.0], // Standard attenuation values
                        };
                        point_count += 1;
                    }
                }
                LightType::Spot => {
                    if spot_count < MAX_SPOT_LIGHTS {
                        multi_env.spot_lights[spot_count] = SpotLightData {
                            position: [
                                light.position.x,
                                light.position.y,
                                light.position.z,
                                light.range,
                            ],
                            direction: [
                                light.direction.x,
                                light.direction.y,
                                light.direction.z,
                                light.intensity,
                            ],
                            color: [
                                light.color.x,
                                light.color.y,
                                light.color.z,
                                0.0,
                            ],
                            cone_angles: [
                                light.inner_cone_angle,
                                light.outer_cone_angle,
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
        multi_env.header.directional_light_count = dir_count as u32;
        multi_env.header.point_light_count = point_count as u32;
        multi_env.header.spot_light_count = spot_count as u32;
        
        multi_env
    }
}
