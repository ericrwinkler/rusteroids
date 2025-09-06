# Lighting and Shader System Refactor Plan

## Executive Summary

This document outlines a comprehensive refactor of the Rusteroids lighting and shader systems to address current limitations and enable advanced rendering features. The plan includes shader organization, multi-light support, shadow mapping, and a modern shader management system.

## 🎯 Project Goals

### User Requirements
- **Multiple Light Support**: Support 4-8 lights of different types simultaneously
- **Directional Lights**: Enhanced directional lighting with proper intensity and color
- **Basic Shadow Mapping**: Teapot self-shadowing (spout/handle shadows on body)
- **Clear Shader Management**: Organized, maintainable shader system with descriptive names

### Technical Objectives
- Eliminate push constant limitations for lighting data
- Implement UBO-based multi-light architecture
- Create scalable shader pipeline with clear naming conventions
- Enable advanced PBR workflows with multiple light interactions
- Implement shadow mapping for realistic depth perception

## 🔍 Current System Analysis

### Existing Shader Problems
Looking at current shaders, the naming system is confusing and inconsistent:

- `frag_material_ubo.frag` vs `frag_ubo_simple.frag` - unclear differences
- `standard_pbr_frag.frag` - good name but isolated
- No clear vertex/fragment shader pairing system
- Feature overlap between similar shaders
- No logical organization or hierarchy

### Current Lighting Limitations
- **Single Light Only**: Limited to one directional light via push constants
- **Push Constant Bottleneck**: 160-byte limit prevents complex lighting
- **No Shadows**: No shadow casting or receiving capabilities
- **Limited Material Response**: Materials don't respond well to lighting variations

## 🏗️ Comprehensive Refactor Plan

### **Phase 1: Shader System Organization (Week 1)**

#### 1.1 New Shader Directory Structure
```
resources/shaders/
├── lighting/
│   ├── single_light.vert/frag       # Current single directional light
│   ├── multi_light.vert/frag        # New multiple light support
│   └── shadow_mapping.vert/frag     # Shadow casting shaders
├── materials/
│   ├── basic_material.vert/frag     # Simple diffuse materials
│   ├── pbr_material.vert/frag       # PBR metallic/roughness workflow
│   └── unlit_material.vert/frag     # No lighting calculations
├── post_processing/
│   ├── tonemap.vert/frag            # HDR tone mapping (future)
│   └── gamma_correct.vert/frag      # Gamma correction (future)
└── utility/
    ├── depth_only.vert/frag         # Shadow map generation
    └── debug_normals.vert/frag      # Debug visualization
```

#### 1.2 Shader Feature Matrix
| Shader Pair | Lighting | Materials | Shadows | Use Case |
|-------------|----------|-----------|---------|----------|
| `basic_material` | Single directional | Push constants | No | Simple objects |
| `pbr_material` | Multi-light | Material UBO | Yes | High quality rendering |
| `unlit_material` | None | Material UBO | No | UI, effects, emissive |
| `shadow_mapping` | Shadow generation | Minimal | Shadow casting | Depth pre-pass |

#### 1.3 Shader Management System
```rust
pub enum ShaderPipeline {
    BasicMaterial,    // Simple diffuse lighting
    PBRMaterial,      // Full PBR with multiple lights
    UnlitMaterial,    // No lighting
    ShadowMapping,    // Depth-only for shadows
}

impl ShaderPipeline {
    pub fn vertex_path(&self) -> &str {
        match self {
            Self::BasicMaterial => "shaders/materials/basic_material.vert",
            Self::PBRMaterial => "shaders/materials/pbr_material.vert",
            Self::UnlitMaterial => "shaders/materials/unlit_material.vert",
            Self::ShadowMapping => "shaders/utility/depth_only.vert",
        }
    }
    
    pub fn fragment_path(&self) -> &str { /* similar */ }
}

pub struct ShaderManager {
    pipelines: HashMap<ShaderPipeline, VulkanPipeline>,
    shader_cache: HashMap<String, CompiledShader>,
}

impl ShaderManager {
    pub fn get_pipeline(&self, material_type: MaterialType, has_shadows: bool) -> ShaderPipeline {
        match (material_type, has_shadows) {
            (MaterialType::StandardPBR, true) => ShaderPipeline::PBRMaterial,
            (MaterialType::StandardPBR, false) => ShaderPipeline::BasicMaterial,
            (MaterialType::Unlit, _) => ShaderPipeline::UnlitMaterial,
        }
    }
    
    pub fn watch_shader_changes(&mut self) -> Result<(), ShaderError> {
        // File watcher for automatic shader recompilation
        // Useful for rapid lighting iteration
    }
}
```

### **Phase 2: Multi-Light UBO Foundation (Week 2)**

#### 2.1 Enhanced LightingUBO Structure
```glsl
// New lighting UBO with multiple light support
layout(set = 0, binding = 1) uniform LightingUBO {
    vec4 ambient_color;                    // RGB + intensity
    
    // Directional lights (sun, moon, etc.)
    DirectionalLight directional_lights[4];
    
    // Point lights (bulbs, fires, etc.)  
    PointLight point_lights[8];
    
    // Spot lights (flashlights, stage lights)
    SpotLight spot_lights[4];
    
    // Light counts and shadow info
    uvec4 light_counts;                    // num_dir, num_point, num_spot, shadow_map_count
    mat4 shadow_matrices[4];               // Shadow map view-projection matrices
} lighting;
```

#### 2.2 Light Type Structures
```glsl
struct DirectionalLight {
    vec4 direction;        // xyz = direction, w = intensity
    vec4 color;           // rgb = color, a = shadow_map_index (-1 = no shadow)
};

struct PointLight {
    vec4 position;        // xyz = position, w = range
    vec4 color;           // rgb = color, a = intensity
    vec4 attenuation;     // x = constant, y = linear, z = quadratic, w = unused
};

struct SpotLight {
    vec4 position;        // xyz = position, w = range
    vec4 direction;       // xyz = direction, w = intensity
    vec4 color;           // rgb = color, a = shadow_map_index
    vec4 cone_angles;     // x = inner_angle, y = outer_angle, z = unused, w = unused
};
```

#### 2.3 Multi-Light Fragment Shader Loop
```glsl
vec3 calculateLighting(vec3 worldPos, vec3 normal, vec3 albedo, float metallic, float roughness) {
    vec3 totalLighting = lighting.ambient_color.rgb * lighting.ambient_color.a;
    
    // Process directional lights
    for (int i = 0; i < lighting.light_counts.x; i++) {
        DirectionalLight light = lighting.directional_lights[i];
        totalLighting += calculateDirectionalLight(light, normal, albedo, metallic, roughness);
    }
    
    // Process point lights
    for (int i = 0; i < lighting.light_counts.y; i++) {
        PointLight light = lighting.point_lights[i];
        totalLighting += calculatePointLight(light, worldPos, normal, albedo, metallic, roughness);
    }
    
    // Process spot lights
    for (int i = 0; i < lighting.light_counts.z; i++) {
        SpotLight light = lighting.spot_lights[i];
        totalLighting += calculateSpotLight(light, worldPos, normal, albedo, metallic, roughness);
    }
    
    return totalLighting;
}
```

### **Phase 3: Advanced Lighting Features (Week 3)**

#### 3.1 Point Light Implementation
```glsl
vec3 calculatePointLight(PointLight light, vec3 worldPos, vec3 normal, vec3 albedo, float metallic, float roughness) {
    vec3 lightPos = light.position.xyz;
    float lightRange = light.position.w;
    vec3 lightColor = light.color.rgb;
    float lightIntensity = light.color.a;
    
    // Distance-based attenuation
    vec3 lightDir = lightPos - worldPos;
    float distance = length(lightDir);
    lightDir = normalize(lightDir);
    
    // Attenuation calculation
    float attenuation = 1.0 / (
        light.attenuation.x +                          // constant
        light.attenuation.y * distance +               // linear
        light.attenuation.z * distance * distance      // quadratic
    );
    
    // Range cutoff
    attenuation *= clamp(1.0 - (distance / lightRange), 0.0, 1.0);
    
    // PBR lighting calculation
    return calculatePBR(albedo, metallic, roughness, normal, lightDir, viewDir, lightColor * lightIntensity * attenuation);
}
```

#### 3.2 Spot Light Implementation
```glsl
vec3 calculateSpotLight(SpotLight light, vec3 worldPos, vec3 normal, vec3 albedo, float metallic, float roughness) {
    vec3 lightPos = light.position.xyz;
    vec3 lightDir = normalize(lightPos - worldPos);
    vec3 spotDir = normalize(light.direction.xyz);
    
    // Spot cone calculation
    float cosTheta = dot(-lightDir, spotDir);
    float innerCone = cos(light.cone_angles.x);
    float outerCone = cos(light.cone_angles.y);
    
    // Smooth falloff between inner and outer cone
    float spotIntensity = clamp((cosTheta - outerCone) / (innerCone - outerCone), 0.0, 1.0);
    
    // Combine with point light calculation
    vec3 pointLightResult = calculatePointLight(
        PointLight(light.position, light.color, vec4(1.0, 0.0, 0.0, 0.0)), 
        worldPos, normal, albedo, metallic, roughness
    );
    
    return pointLightResult * spotIntensity;
}
```

### **Phase 4: Shadow Mapping Foundation (Week 4)**

#### 4.1 Shadow Map Resources
```glsl
// Shadow map textures - Set 0, Binding 2
layout(set = 0, binding = 2) uniform sampler2D shadow_maps[4];

// Shadow coordinate calculation in vertex shader
layout(location = 4) out vec4 fragShadowCoords[4];  // Up to 4 shadow maps

void main() {
    // Standard vertex transformation
    vec4 worldPosition = pushConstants.model_matrix * vec4(position, 1.0);
    gl_Position = camera.view_projection_matrix * worldPosition;
    
    // Calculate shadow coordinates for each shadow-casting light
    for (int i = 0; i < lighting.light_counts.w; i++) {
        fragShadowCoords[i] = lighting.shadow_matrices[i] * worldPosition;
    }
}
```

#### 4.2 Shadow Sampling with PCF
```glsl
float calculateShadow(int shadow_index, vec4 shadow_coord) {
    // Perspective divide
    vec3 projCoords = shadow_coord.xyz / shadow_coord.w;
    
    // Transform to [0,1] range
    projCoords = projCoords * 0.5 + 0.5;
    
    // Check if fragment is outside shadow map
    if (projCoords.z > 1.0 || projCoords.x < 0.0 || projCoords.x > 1.0 || 
        projCoords.y < 0.0 || projCoords.y > 1.0) {
        return 1.0; // No shadow
    }
    
    // PCF (Percentage Closer Filtering) for soft shadows
    float shadow = 0.0;
    vec2 texelSize = 1.0 / textureSize(shadow_maps[shadow_index], 0);
    float bias = 0.005; // Prevent shadow acne
    
    for (int x = -1; x <= 1; ++x) {
        for (int y = -1; y <= 1; ++y) {
            vec2 offset = vec2(x, y) * texelSize;
            float pcfDepth = texture(shadow_maps[shadow_index], projCoords.xy + offset).r;
            shadow += (projCoords.z - bias) > pcfDepth ? 0.0 : 1.0;
        }
    }
    
    return shadow / 9.0; // Average of 9 samples
}
```

#### 4.3 Teapot Self-Shadowing Integration
```glsl
vec3 calculateDirectionalLight(DirectionalLight light, vec3 normal, vec3 albedo, float metallic, float roughness) {
    vec3 lightDir = normalize(-light.direction.xyz);
    float lightIntensity = light.direction.w;
    vec3 lightColor = light.color.rgb;
    int shadowMapIndex = int(light.color.a);
    
    // Calculate PBR lighting
    vec3 lighting_result = calculatePBR(albedo, metallic, roughness, normal, lightDir, viewDir, lightColor * lightIntensity);
    
    // Apply shadow if this light casts shadows
    if (shadowMapIndex >= 0) {
        float shadow = calculateShadow(shadowMapIndex, fragShadowCoords[shadowMapIndex]);
        lighting_result *= shadow;
    }
    
    return lighting_result;
}
```

### **Phase 5: Shadow Quality & Polish (Week 5)**

#### 5.1 Depth-Only Render Pass
```rust
pub struct ShadowMapManager {
    shadow_maps: Vec<ShadowMap>,
    depth_render_pass: vk::RenderPass,
    shadow_resolution: u32,
}

impl ShadowMapManager {
    pub fn render_shadow_map(&mut self, light_index: usize, scene_objects: &[RenderObject]) -> VulkanResult<()> {
        // Render scene from light's perspective to depth texture
        // Use minimal vertex shader, no fragment shader output
        
        // Set up light's view-projection matrix
        let light_view_proj = self.calculate_light_matrix(light_index);
        
        // Render all shadow-casting objects
        for object in scene_objects {
            if object.casts_shadows {
                self.render_depth_only(object, &light_view_proj)?;
            }
        }
        
        Ok(())
    }
    
    fn calculate_light_matrix(&self, light_index: usize) -> Mat4 {
        // For directional lights: orthographic projection
        // Position camera far back in light direction
        // Use orthographic projection to cover scene bounds
    }
}
```

#### 5.2 Shadow Bias and Quality Improvements
```glsl
// Improved shadow calculation with slope-scale bias
float calculateShadowWithBias(int shadow_index, vec4 shadow_coord, vec3 normal, vec3 lightDir) {
    vec3 projCoords = shadow_coord.xyz / shadow_coord.w;
    projCoords = projCoords * 0.5 + 0.5;
    
    // Calculate bias based on surface angle to light
    float cosTheta = clamp(dot(normal, lightDir), 0.0, 1.0);
    float bias = 0.005 * tan(acos(cosTheta));
    bias = clamp(bias, 0.0, 0.01);
    
    // PCF with larger kernel for softer shadows
    float shadow = 0.0;
    vec2 texelSize = 1.0 / textureSize(shadow_maps[shadow_index], 0);
    int kernelSize = 2; // 5x5 kernel
    float samples = 0.0;
    
    for (int x = -kernelSize; x <= kernelSize; ++x) {
        for (int y = -kernelSize; y <= kernelSize; ++y) {
            vec2 offset = vec2(x, y) * texelSize;
            float pcfDepth = texture(shadow_maps[shadow_index], projCoords.xy + offset).r;
            shadow += (projCoords.z - bias) > pcfDepth ? 0.0 : 1.0;
            samples += 1.0;
        }
    }
    
    return shadow / samples;
}
```

### **Phase 6: Integration & Polish (Week 6)**

#### 6.1 Complete System Integration
- Integrate multi-light system with shadow mapping
- Implement shader hot reload for rapid iteration
- Add debug visualization tools (light volumes, shadow cascades)
- Performance profiling and optimization
- Comprehensive documentation and usage examples

#### 6.2 Performance Optimization
- Light culling (disable distant/weak lights)
- Shadow map resolution scaling based on light importance
- Early loop exits in fragment shaders
- Efficient UBO updates (only when lights change)

## 🎮 Visual Goals and Success Criteria

### Teapot Demo Scenarios
1. **Multiple Directional Lights**: Warm sun + cool sky lighting with clear material differences
2. **Point Light + Directional**: Interior scene simulation with window light + lamp
3. **Self-Shadowing**: Teapot handle and spout cast realistic shadows on main body
4. **Material Variation**: Chrome, gold, matte rubber respond differently to each light type
5. **Shadow Quality**: Soft, realistic shadows that enhance depth perception without artifacts

### Performance Targets
- **60+ FPS** with 4-6 active lights and 2048x2048 shadow maps
- **Smooth Light Management**: Add/remove lights without frame drops
- **Efficient Shadow Updates**: Only regenerate shadow maps when lights move
- **Memory Efficiency**: Reasonable GPU memory usage for textures and UBOs

### Quality Metrics
- **Material Distinction**: Clear visual differences between metallic/rough materials under multiple lights
- **Shadow Realism**: Convincing self-shadowing without shadow acne or light leaking
- **Lighting Response**: Materials respond appropriately to different light colors and intensities
- **Performance Stability**: Consistent frame times without hitches during light changes

## 🔧 Implementation Dependencies

### Prerequisites
- ✅ **Phase 5 Complete**: Material UBO system with Set 1 descriptors (COMPLETED)
- ✅ **UBO Architecture**: Set 0 descriptors for camera/lighting (COMPLETED)
- ✅ **Pipeline Management**: Multi-pipeline system (COMPLETED)
- ✅ **Vulkan Validation**: Clean validation with proper resource management (COMPLETED)

### Required Infrastructure Changes
- **UBO Expansion**: Expand Set 0 binding 1 for complex lighting data
- **Descriptor Set Management**: Handle dynamic light count changes
- **Shadow Map Resources**: Depth textures and framebuffer management
- **Shader Hot Reload**: Development iteration support

## 📋 Testing and Validation

### Screenshot Validation Requirements
Using the existing screenshot validation tool:
- **Baseline Capture**: Before each phase implementation
- **Feature Validation**: After each lighting feature addition
- **Regression Testing**: Ensure no visual quality degradation
- **Performance Monitoring**: Frame rate validation during development

### Manual Testing Scenarios
1. **Single Light Validation**: Ensure no regression from current system
2. **Multi-Light Progressive**: Add lights one by one, verify each addition
3. **Shadow Quality**: Test shadow artifacts, bias, and quality settings
4. **Material Response**: Verify all material types respond correctly to new lighting
5. **Performance Testing**: Monitor GPU/CPU usage under various light loads

This comprehensive refactor will transform the Rusteroids lighting system from a basic single-light setup to a sophisticated multi-light engine with shadows, providing the foundation for advanced rendering features while maintaining clean, maintainable code architecture.
