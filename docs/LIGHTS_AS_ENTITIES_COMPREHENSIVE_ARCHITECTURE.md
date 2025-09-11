# Rusteroids Architecture Documentation: Lights as Entities System

## Executive Summary

This document consolidates the complete architecture for implementing lights as entities in the Rusteroids Vulkan rendering engine. It combines lessons from Game Engine Architecture and Vulkano tutorial documentation with existing engine infrastructure to provide a comprehensive implementation guide.

## Current System Status: Infrastructure Complete ✅

The Rusteroids project has a solid foundation ready for lights-as-entities implementation:

### Existing Infrastructure ✅
- **Material System**: Complete modular architecture with MaterialType enum, MaterialUBO structs, MaterialManager
- **Pipeline Management**: PipelineManager supporting multiple pipeline types (StandardPBR, Unlit, TransparentPBR, TransparentUnlit)  
- **UBO Architecture**: Set 0 (camera/lighting) and Set 1 (materials) descriptor layouts operational
- **Performance**: Smooth 60+ FPS with 18,960 vertex teapot model
- **Academic Standards**: Johannes Unterguggenberger guide-compliant projection matrices
- **World-Space Lighting**: Fixed directional lighting with proper normal matrix transformations
- **Quality Assurance**: Screenshot validation tool with automated regression testing

### Current Lighting Limitations
- **Single Light Only**: Limited to one directional light via push constants
- **Push Constant Bottleneck**: 160-byte limit prevents multiple lights
- **Hardcoded Lighting**: Lights defined in teapot app initialization, not as entities
- **No Runtime Flexibility**: Cannot add, remove, or modify lights during gameplay

## Architecture Design: Lights as Entities

### Design Principles

Following Game Engine Architecture and Vulkano tutorial patterns:

**Game Engine Architecture Compliance:**
- ✅ Lights are first-class entities with Transform + LightComponent
- ✅ Components are reusable and data-focused  
- ✅ Systems process entities, not hardcoded data
- ✅ Runtime flexibility for adding/removing/modifying lights

**Vulkano Tutorial Compliance:**
- ✅ Multiple lights via uniform buffers, not push constants
- ✅ Structured light data (DirectionalLight, PointLight arrays)
- ✅ Shader loops processing multiple lights
- ✅ GPU-efficient data layout with proper alignment

## Coordinate System Standards

### Primary Coordinate System: World Space Y-Up Right-Handed

All components use consistent **World Space Y-Up Right-Handed** coordinates:

```
World Space (Y-Up Right-Handed):
  Y
  ↑
  |
  |____→ X
 /
Z (toward viewer)

- X-axis: Right (positive) / Left (negative)  
- Y-axis: Up (positive) / Down (negative)
- Z-axis: Toward viewer (positive) / Away from viewer (negative)
```

**Rationale:**
- Consistent with existing engine and Johannes Unterguggenberger guide
- Standard in modeling tools (Blender, Maya) and OBJ file format
- Maintains compatibility with existing renderer coordinate transformation chain

### Coordinate Flow Through Pipeline

```
Local Space → World Space → View Space → Vulkan Space → Clip Space
     ↓             ↓           ↓            ↓           ↓
   Model        View      Vulkan X    Projection   Screen
  Matrix       Matrix     Matrix       Matrix      Space

Complete chain: Clip = Projection × Vulkan_X × View × Model × Local
```

**Lighting Coordinate Space**: All lighting calculations performed in **World Space**
- Light directions naturally specified in world coordinates
- Consistent with entity transform system
- Camera-independent lighting behavior
- Simplified multi-light calculations

## Component System Architecture

### 1. Entity Components

```rust
// crates/rust_engine/src/ecs/components/light_component.rs
#[derive(Debug, Clone)]
pub struct LightComponent {
    pub light: Light,  // Reuses existing Light struct
    pub enabled: bool,
    pub cast_shadows: bool,
}

impl LightComponent {
    pub fn directional(direction: Vec3, color: Vec3, intensity: f32) -> Self {
        Self {
            light: Light::directional(direction, color, intensity),
            enabled: true,
            cast_shadows: false,
        }
    }
    
    pub fn point(position: Vec3, color: Vec3, intensity: f32, range: f32) -> Self {
        Self {
            light: Light::point(position, color, intensity, range),
            enabled: true, 
            cast_shadows: false,
        }
    }
    
    pub fn spot(position: Vec3, direction: Vec3, color: Vec3, intensity: f32, range: f32, inner_angle: f32, outer_angle: f32) -> Self {
        Self {
            light: Light::spot(position, direction, color, intensity, range, inner_angle, outer_angle),
            enabled: true,
            cast_shadows: false,
        }
    }
}

// crates/rust_engine/src/ecs/components/mesh_component.rs  
#[derive(Debug, Clone)]
pub struct MeshComponent {
    pub mesh: Mesh,
    pub material: Material,
    pub visible: bool,
    pub cast_shadows: bool,
    pub receive_shadows: bool,
}
```

### 2. Transform Component Coordinates

```rust
// Transform component for both light and mesh entities
pub struct Transform {
    pub position: Vec3,    // World space position
    pub rotation: Quat,    // World space rotation quaternion
    pub scale: Vec3,       // World space scale factors
}

// Light direction calculation from transform:
impl Transform {
    pub fn forward_direction(&self) -> Vec3 {
        // Default forward = negative Z (away from viewer)
        self.rotation * Vec3::new(0.0, 0.0, -1.0)
    }
    
    pub fn matrix(&self) -> Mat4 {
        // T * R * S transformation matrix
        Mat4::from_translation(self.position) * 
        Mat4::from_quat(self.rotation) * 
        Mat4::from_scale(self.scale)
    }
}
```

### 3. Entity Coordinate Systems

#### Directional Light Entities
```rust
pub struct DirectionalLightEntity {
    pub transform: Transform,        // Position irrelevant, rotation defines direction
    pub light_component: LightComponent,
}

// Light direction: transform.forward_direction()
// Represents direction FROM light TO surface (what light illuminates)
```

#### Point Light Entities  
```rust
pub struct PointLightEntity {
    pub transform: Transform,        // Position defines light location
    pub light_component: LightComponent,
}

// Light position: transform.position
// Affects surfaces within range radius from this position
```

#### Mesh Entities (Teapot)
```rust
pub struct TeapotEntity {
    pub transform: Transform,        // World space transform
    pub mesh_component: MeshComponent,
}

// Teapot transformation: transform.matrix() applied to local vertices
```

## Multi-Light Renderer Interface

### Current vs Target Renderer Interface

**Current Limitation (Push Constants):**
```rust
// Limited to 160 bytes total
struct PushConstants {
    mvp_matrix: [[f32; 4]; 4],      // 64 bytes
    normal_matrix: [[f32; 4]; 3],   // 48 bytes  
    ambient_intensity: f32,         // 4 bytes
    light_direction: [f32; 3],      // 12 bytes - SINGLE LIGHT ONLY
    light_color: [f32; 3],          // 12 bytes
    light_intensity: f32,           // 4 bytes
    // Total: 160 bytes (near limit, no room for multiple lights)
}
```

**Target Solution (Uniform Buffer Objects):**
```rust
// Multi-light UBO structure following Vulkano tutorial
#[repr(C)]
pub struct MultiLightUBO {
    pub ambient_color: [f32; 4],           // 16 bytes (RGB + intensity)
    pub directional_light_count: u32,      // 4 bytes  
    pub point_light_count: u32,            // 4 bytes
    pub spot_light_count: u32,             // 4 bytes
    pub _padding: u32,                     // 4 bytes (std140 alignment)
    // Total header: 32 bytes
}

#[repr(C)]
pub struct DirectionalLightData {
    pub direction: [f32; 4],               // 16 bytes (xyz + intensity)
    pub color: [f32; 4],                   // 16 bytes (rgb + padding)
    // Total: 32 bytes per light
}

#[repr(C)]
pub struct PointLightData {
    pub position: [f32; 4],                // 16 bytes (xyz + range)
    pub color: [f32; 4],                   // 16 bytes (rgb + intensity)
    pub attenuation: [f32; 4],             // 16 bytes (constant, linear, quadratic, padding)
    // Total: 48 bytes per light
}

#[repr(C)]
pub struct SpotLightData {
    pub position: [f32; 4],                // 16 bytes (xyz + range)
    pub direction: [f32; 4],               // 16 bytes (xyz + intensity)  
    pub color: [f32; 4],                   // 16 bytes (rgb + padding)
    pub cone_angles: [f32; 4],             // 16 bytes (inner, outer, unused, unused)
    // Total: 64 bytes per light
}

// Support limits following Vulkano tutorial recommendations
pub const MAX_DIRECTIONAL_LIGHTS: usize = 4;
pub const MAX_POINT_LIGHTS: usize = 8;
pub const MAX_SPOT_LIGHTS: usize = 4;

pub struct MultiLightEnvironment {
    pub header: MultiLightUBO,
    pub directional_lights: [DirectionalLightData; MAX_DIRECTIONAL_LIGHTS],
    pub point_lights: [PointLightData; MAX_POINT_LIGHTS], 
    pub spot_lights: [SpotLightData; MAX_SPOT_LIGHTS],
}
```

### Enhanced Renderer Interface

```rust
// crates/rust_engine/src/render/mod.rs - Enhanced interface
impl Renderer {
    // Replace single light method with multi-light UBO
    pub fn set_lighting_environment(&mut self, lighting: &MultiLightEnvironment) -> Result<(), RendererError>;
    
    // Keep existing material/mesh methods unchanged for compatibility
    pub fn set_material(&mut self, material: &Material) -> Result<(), RendererError>;
    pub fn render_mesh(&mut self, mesh: &Mesh, transform: &Mat4) -> Result<(), RendererError>;
    
    // Enhanced methods for entity support
    pub fn render_mesh_entity(&mut self, mesh_comp: &MeshComponent, transform: &Transform) -> Result<(), RendererError> {
        self.set_material(&mesh_comp.material)?;
        self.render_mesh(&mesh_comp.mesh, &transform.matrix())
    }
}
```

## Shader System Architecture

### Descriptor Set Layout (Enhanced)

**Set 0: Per-Frame Data (Enhanced for Multiple Lights)**
```glsl
layout(set = 0, binding = 0) uniform CameraUBO {
    mat4 view_matrix;
    mat4 projection_matrix;
    vec3 camera_position;
    float _padding;
} camera;

layout(set = 0, binding = 1) uniform LightingUBO {
    vec4 ambient_color;                    // RGB + intensity
    uint directional_light_count;
    uint point_light_count;
    uint spot_light_count;
    uint _padding;
    
    DirectionalLightData directional_lights[MAX_DIRECTIONAL_LIGHTS];
    PointLightData point_lights[MAX_POINT_LIGHTS];
    SpotLightData spot_lights[MAX_SPOT_LIGHTS];
} lighting;
```

**Set 1: Material Data (Existing)**
```glsl
layout(set = 1, binding = 0) uniform MaterialUBO {
    vec4 base_color;
    float metallic;
    float roughness;
    uint texture_flags;
    uint _padding;
} material;

layout(set = 1, binding = 1) uniform sampler2D base_color_texture;
layout(set = 1, binding = 2) uniform sampler2D normal_texture;
layout(set = 1, binding = 3) uniform sampler2D metallic_roughness_texture;
```

### Multi-Light Fragment Shader

```glsl
// resources/shaders/lighting/multi_light.frag
#version 450

// Light type structures
struct DirectionalLightData {
    vec4 direction;        // xyz = direction, w = intensity
    vec4 color;           // rgb = color, a = unused
};

struct PointLightData {
    vec4 position;        // xyz = position, w = range
    vec4 color;           // rgb = color, a = intensity
    vec4 attenuation;     // x = constant, y = linear, z = quadratic, w = unused
};

struct SpotLightData {
    vec4 position;        // xyz = position, w = range
    vec4 direction;       // xyz = direction, w = intensity
    vec4 color;           // rgb = color, a = unused
    vec4 cone_angles;     // x = inner_angle, y = outer_angle, z = unused, w = unused
};

// Fragment inputs (world space)
layout(location = 0) in vec3 world_position;
layout(location = 1) in vec3 world_normal;  
layout(location = 2) in vec2 uv;

// Fragment output
layout(location = 0) out vec4 out_color;

// UBO bindings
layout(set = 0, binding = 0) uniform CameraUBO { /*...*/ } camera;
layout(set = 0, binding = 1) uniform LightingUBO { /*...*/ } lighting;
layout(set = 1, binding = 0) uniform MaterialUBO { /*...*/ } material;

// PBR lighting calculation
vec3 calculatePBR(vec3 albedo, float metallic, float roughness, vec3 N, vec3 L, vec3 V, vec3 lightColor) {
    // Cook-Torrance BRDF implementation
    // (existing PBR code from material system)
}

void main() {
    vec3 N = normalize(world_normal);
    vec3 V = normalize(camera.camera_position - world_position);
    vec3 albedo = material.base_color.rgb;
    
    // Start with ambient
    vec3 color = lighting.ambient_color.rgb * lighting.ambient_color.a * albedo;
    
    // Process directional lights
    for (uint i = 0u; i < lighting.directional_light_count; ++i) {
        DirectionalLightData light = lighting.directional_lights[i];
        vec3 L = normalize(-light.direction.xyz);  // Light-to-surface direction
        vec3 lightColor = light.color.rgb * light.direction.w; // intensity in w component
        
        color += calculatePBR(albedo, material.metallic, material.roughness, N, L, V, lightColor);
    }
    
    // Process point lights  
    for (uint i = 0u; i < lighting.point_light_count; ++i) {
        PointLightData light = lighting.point_lights[i];
        vec3 lightPos = light.position.xyz;
        float range = light.position.w;
        vec3 L = normalize(lightPos - world_position);
        
        float distance = length(lightPos - world_position);
        if (distance < range) {
            float attenuation = 1.0 / (light.attenuation.x + light.attenuation.y * distance + light.attenuation.z * distance * distance);
            vec3 lightColor = light.color.rgb * light.color.a * attenuation;
            
            color += calculatePBR(albedo, material.metallic, material.roughness, N, L, V, lightColor);
        }
    }
    
    // Process spot lights
    for (uint i = 0u; i < lighting.spot_light_count; ++i) {
        SpotLightData light = lighting.spot_lights[i];
        vec3 lightPos = light.position.xyz;
        vec3 lightDir = normalize(light.direction.xyz);
        vec3 L = normalize(lightPos - world_position);
        
        float theta = dot(L, -lightDir);
        float epsilon = cos(light.cone_angles.x) - cos(light.cone_angles.y);
        float intensity = clamp((theta - cos(light.cone_angles.y)) / epsilon, 0.0, 1.0);
        
        if (intensity > 0.0) {
            float distance = length(lightPos - world_position);
            float range = light.position.w;
            
            if (distance < range) {
                float attenuation = 1.0 / (1.0 + 0.09 * distance + 0.032 * distance * distance);
                vec3 lightColor = light.color.rgb * light.direction.w * attenuation * intensity;
                
                color += calculatePBR(albedo, material.metallic, material.roughness, N, L, V, lightColor);
            }
        }
    }
    
    out_color = vec4(color, material.base_color.a);
}
```

## Entity System Implementation

### Simple Entity Management for Teapot App

```rust
// teapot_app/src/main.rs - Entity-based teapot application
pub struct EntityTeapotApp {
    // Core systems (unchanged)
    window: WindowHandle,
    renderer: Renderer, 
    camera: Camera,
    
    // Entity system
    world: GameWorld,
    
    // Entity IDs
    teapot_entity: EntityId,
    sun_light_entity: EntityId,
    lamp_light_entity: EntityId,
    
    // Runtime state
    start_time: Instant,
    total_rotation: f32,
}

pub struct GameWorld {
    next_entity_id: u32,
    light_components: HashMap<EntityId, LightComponent>,
    mesh_components: HashMap<EntityId, MeshComponent>,
    transforms: HashMap<EntityId, Transform>,
}

impl GameWorld {
    pub fn new() -> Self {
        Self {
            next_entity_id: 1,
            light_components: HashMap::new(),
            mesh_components: HashMap::new(), 
            transforms: HashMap::new(),
        }
    }
    
    pub fn create_light_entity(&mut self, transform: Transform, light: LightComponent) -> EntityId {
        let entity_id = self.next_entity_id;
        self.next_entity_id += 1;
        
        self.transforms.insert(entity_id, transform);
        self.light_components.insert(entity_id, light);
        
        entity_id
    }
    
    pub fn create_mesh_entity(&mut self, transform: Transform, mesh: MeshComponent) -> EntityId {
        let entity_id = self.next_entity_id;
        self.next_entity_id += 1;
        
        self.transforms.insert(entity_id, transform);
        self.mesh_components.insert(entity_id, mesh);
        
        entity_id
    }
    
    pub fn get_light_entities(&self) -> Vec<(EntityId, &Transform, &LightComponent)> {
        self.light_components.iter().map(|(id, light)| {
            let transform = &self.transforms[id];
            (*id, transform, light)
        }).collect()
    }
    
    pub fn get_mesh_entities(&self) -> Vec<(EntityId, &Transform, &MeshComponent)> {
        self.mesh_components.iter().map(|(id, mesh)| {
            let transform = &self.transforms[id];
            (*id, transform, mesh)
        }).collect()
    }
}

impl EntityTeapotApp {
    pub fn new() -> Self {
        let mut world = GameWorld::new();
        
        // Create light entities following the existing teapot app lighting
        let sun_light = world.create_light_entity(
            Transform::identity(), // Position irrelevant for directional light
            LightComponent::directional(
                Vec3::new(-0.7, -1.0, 0.3),     // Same direction as existing teapot app
                Vec3::new(1.0, 0.95, 0.9),      // Warm white color  
                1.5                              // Strong intensity
            )
        );
        
        // Add second light - point light for multiple light demo
        let lamp_light = world.create_light_entity(
            Transform::from_position(Vec3::new(2.0, 1.0, 0.0)),
            LightComponent::point(
                Vec3::new(2.0, 1.0, 0.0),       // Position to the right and above
                Vec3::new(0.8, 0.4, 0.2),       // Orange/red color like fire
                5.0,                             // Intensity
                10.0                             // Range
            )
        );
        
        // Create teapot entity (load mesh same as existing teapot app)
        let teapot_mesh = /* load teapot mesh same as existing app */;
        let teapot_material = /* create material same as existing app */;
        let teapot_entity = world.create_mesh_entity(
            Transform::identity(),
            MeshComponent {
                mesh: teapot_mesh,
                material: teapot_material,
                visible: true,
                cast_shadows: false,
                receive_shadows: true,
            }
        );
        
        Self { world, sun_light, lamp_light, teapot_entity, /* ... */ }
    }
    
    pub fn update(&mut self, delta_time: f32) -> Result<(), Box<dyn std::error::Error>> {
        // Update teapot rotation (same as existing app)
        self.total_rotation += delta_time * 0.5;
        if let Some(teapot_transform) = self.world.transforms.get_mut(&self.teapot_entity) {
            teapot_transform.rotation = Quat::from_rotation_y(self.total_rotation);
        }
        
        // Animate the point light position for demonstration  
        let time = self.start_time.elapsed().as_secs_f32();
        if let Some(lamp_transform) = self.world.transforms.get_mut(&self.lamp_light_entity) {
            lamp_transform.position.x = 2.0 + (time * 0.5).sin() * 1.5;
        }
        
        // Collect lights from entities and create multi-light environment
        let light_entities = self.world.get_light_entities();
        let multi_light_env = self.build_multi_light_environment(light_entities);
        self.renderer.set_lighting_environment(&multi_light_env)?;
        
        // Render mesh entities 
        for (entity_id, transform, mesh_comp) in self.world.get_mesh_entities() {
            if mesh_comp.visible {
                self.renderer.render_mesh_entity(mesh_comp, transform)?;
            }
        }
        
        Ok(())
    }
}
```

### Entity-to-Renderer Conversion

```rust
impl EntityTeapotApp {
    fn build_multi_light_environment(&self, entities: Vec<(EntityId, &Transform, &LightComponent)>) -> MultiLightEnvironment {
        let mut env = MultiLightEnvironment::new();
        
        for (_, transform, light_comp) in entities {
            if !light_comp.enabled {
                continue;
            }
            
            match light_comp.light.light_type {
                LightType::Directional => {
                    // Convert entity transform to world direction
                    let world_direction = transform.forward_direction();
                    env.add_directional_light(DirectionalLightData {
                        direction: [world_direction.x, world_direction.y, world_direction.z, light_comp.light.intensity],
                        color: [light_comp.light.color.x, light_comp.light.color.y, light_comp.light.color.z, 0.0],
                    });
                }
                LightType::Point => {
                    // Use entity world position directly
                    env.add_point_light(PointLightData {
                        position: [transform.position.x, transform.position.y, transform.position.z, light_comp.light.range],
                        color: [light_comp.light.color.x, light_comp.light.color.y, light_comp.light.color.z, light_comp.light.intensity],
                        attenuation: [1.0, 0.09, 0.032, 0.0], // Standard attenuation values
                    });
                }
                LightType::Spot => {
                    // Position + direction from entity transform
                    let world_direction = transform.forward_direction();
                    env.add_spot_light(SpotLightData {
                        position: [transform.position.x, transform.position.y, transform.position.z, light_comp.light.range],
                        direction: [world_direction.x, world_direction.y, world_direction.z, light_comp.light.intensity],
                        color: [light_comp.light.color.x, light_comp.light.color.y, light_comp.light.color.z, 0.0],
                        cone_angles: [light_comp.light.inner_cone_angle, light_comp.light.outer_cone_angle, 0.0, 0.0],
                    });
                }
            }
        }
        
        env
    }
}
```

## Implementation Strategy

This architecture requires careful incremental implementation to avoid coordinate transformation errors and maintain system stability. See **[LIGHTS_AS_ENTITIES_INCREMENTAL_IMPLEMENTATION_PLAN.md](LIGHTS_AS_ENTITIES_INCREMENTAL_IMPLEMENTATION_PLAN.md)** for detailed step-by-step implementation with validation procedures.

### Implementation Overview

**Phase 0: Clean Up and Baseline (2 days)**
- Fix current build issues and establish reference metrics
- Coordinate validation: No changes, pure compilation fixes

**Phase 1: Multi-Light UBO Foundation (3 days)**  
- Create multi-light data structures and shaders
- Replace push constants with UBOs while maintaining identical visual output
- Coordinate validation: Entity coordinates must match hardcoded coordinates exactly

**Phase 2: Entity System Foundation (3 days)**
- Replace hardcoded lights with entities in teapot app
- Validate all coordinate transformations with unit tests
- Coordinate validation: Entity system produces identical lighting to hardcoded system

**Phase 3: Multiple Light Demonstration (2 days)**
- Add second light entity and demonstrate runtime flexibility  
- Performance validation with multiple lights (60+ FPS target)
- Visual validation: Multiple lights visibly affecting teapot surfaces

### Critical Success Metrics

Each phase includes immediate validation to catch coordinate errors:

**Visual Validation**: Teapot app renders identically with console value verification  
**Performance**: Maintain 60+ FPS throughout implementation  
**Coordinate Accuracy**: ±0.001 tolerance verified through console output comparison
**Immediate Error Detection**: Any visual change to teapot = STOP, fix immediately

### Visual Validation Strategy

**Immediate Feedback Approach**:
- Add temporary validation functions to teapot app for real-time verification
- Console output shows exact value comparisons during conversion
- Visual regression detection through direct teapot app observation  
- **Cleanup Requirement**: Remove all verification bloat immediately after validation

**Validation Commands**:
```cmd
cargo build          # Must compile cleanly
cargo run            # Teapot must look identical + console shows value matches
```

See the incremental implementation plan for detailed validation procedures and risk mitigation strategies.

## Quality Assurance Integration

### Screenshot Validation Workflow

Following existing screenshot validation procedures:

```cmd
# Before implementing lights-as-entities changes
.\tools\validate_rendering.bat baseline

# After Phase 1 (Renderer Upgrade)
.\tools\validate_rendering.bat renderer_multilight

# After Phase 2 (Entity System) 
.\tools\validate_rendering.bat entity_system

# After Phase 3 (Multiple Lights)
.\tools\validate_rendering.bat multiple_lights
```

**Expected Results**:
- Content Classification: RenderedScene ✅
- Colored Pixels: >60% (proper 3D rendering) ✅
- No visual regressions from baseline ✅
- Multiple light effects visible in final phase ✅

### Development Guidelines

1. **Before Each Phase**: Capture baseline screenshot
2. **After Each Phase**: Validate visual output and performance
3. **Coordinate System Validation**: Verify world space consistency
4. **Performance Monitoring**: Maintain 60+ FPS throughout implementation
5. **Vulkan Validation**: Ensure clean validation layers (no errors)

## Technical Architecture Summary

This architecture successfully addresses the original request:

**✅ Game Engine Architecture Compliance**
- Lights as first-class entities with Transform + LightComponent
- Entity-component-system design patterns
- Runtime flexibility for light management
- Clean separation between game logic and rendering

**✅ Vulkano Tutorial Compliance**  
- Multiple lights via uniform buffers (not push constants)
- Structured light data with proper GPU alignment
- Shader loops processing light arrays efficiently
- GPU-optimized data layout following std140 rules

**✅ Existing Infrastructure Integration**
- Maintains compatibility with current material system
- Uses existing UBO architecture (Set 0/Set 1 pattern)
- Preserves coordinate system standards
- Leverages existing screenshot validation workflow

**✅ Measurable Success Criteria**
- Transform from single hardcoded light to multiple light entities  
- Visual demonstration of multiple lights affecting teapot rendering
- Runtime light modification capabilities
- Performance maintained at 60+ FPS

This comprehensive architecture provides a complete roadmap for implementing lights as entities while maintaining the high-quality infrastructure already established in the Rusteroids project.
