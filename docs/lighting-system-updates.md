# Lighting System - Updates and Future Work

## ✅ Completed
- **Normal Matrix Calculation**: Fixed inverse-transpose calculation with proper matrix transposition
- **Coordinate Space Alignment**: Ensured normals and light direction are both in world space
- **GLSL Matrix Storage**: Properly converted from nalgebra to GLSL column-major format
- **Documentation**: Added comprehensive comments and external documentation
- **Integration with New Coordinate System**: Works correctly with Johannes Unterguggenberger guide compliance
- **Aspect Ratio Compatibility**: Lighting remains consistent during window resizing with dynamic viewport system

## 🔍 Current System Status

### Working Features ✅
- **World-Space Directional Lighting**: Single directional light with correct intensity and color
- **Proper Normal Transformations**: Inverse-transpose normal matrix with GLSL compatibility
- **Material Color Support**: Per-object material colors with proper lighting calculations
- **Ambient Lighting**: Configurable ambient light intensity
- **Push Constant Integration**: Efficient 160-byte push constant layout including lighting data

### Current Push Constants Layout
```rust
#[repr(C)]
struct PushConstants {
    mvp_matrix: [[f32; 4]; 4],      // 64 bytes - includes coordinate transform
    normal_matrix: [[f32; 4]; 3],   // 48 bytes - inverse-transpose with padding
    material_color: [f32; 4],       // 16 bytes - RGBA
    light_direction: [f32; 3],      // 12 bytes - world space directional light
    light_intensity: f32,           // 4 bytes
    light_color: [f32; 3],          // 12 bytes - RGB
    ambient_intensity: f32,         // 4 bytes
}
// Total: 160 bytes - near push constant limit but working efficiently
```

## 🔍 Immediate Considerations

### 1. Performance Optimization
**Current**: Normal matrix calculated every frame
**Future**: Cache calculation when model matrix unchanged

```rust
// Add to VulkanRenderer struct
cached_model_matrix: Option<[[f32; 4]; 4]>,
cached_normal_matrix: [[f32; 4]; 3],

// In set_model_matrix():
if Some(model) == self.cached_model_matrix {
    return; // Use cached normal matrix
}
```

### 2. Validation Layer Cleanup
**Current**: Vulkan validation errors on shutdown (command buffer cleanup)
**Impact**: Low (doesn't affect rendering)
**Priority**: Medium

## 🚧 Future Enhancements

### 1. Multiple Light Support (NEXT PRIORITY)
**Current**: Single directional light limited by push constant space
**Future**: Multiple light types using uniform buffer objects

```rust
// Planned UBO structure for multiple lights
#[repr(C)]
struct LightingUniformData {
    ambient_color: Vec4,                    // 16 bytes
    directional_lights: [DirectionalLight; 4], // 4 lights max
    point_lights: [PointLight; 8],         // 8 lights max  
    spot_lights: [SpotLight; 4],           // 4 lights max
    num_dir_lights: u32,                   // 4 bytes
    num_point_lights: u32,                 // 4 bytes
    num_spot_lights: u32,                  // 4 bytes
    _padding: u32,                         // 4 bytes
}

#[repr(C)]
struct DirectionalLight {
    direction: Vec4,        // 16 bytes (world space direction + padding)
    color: Vec4,            // 16 bytes (RGB color + intensity)
}

#[repr(C)]
struct PointLight {
    position: Vec4,         // 16 bytes (world space position + radius)
    color: Vec4,            // 16 bytes (RGB color + intensity)
    attenuation: Vec4,      // 16 bytes (constant, linear, quadratic, _pad)
}
```

### 2. Advanced Material System
**Current**: Single material color per object
**Future**: Full PBR (Physically Based Rendering) materials

```rust
// Planned material system
#[repr(C)]
struct PBRMaterial {
    base_color: Vec4,           // 16 bytes - RGBA albedo
    metallic_roughness: Vec4,   // 16 bytes - metallic, roughness, AO, _pad
    emission: Vec4,             // 16 bytes - RGB emission + strength
    normal_scale: f32,          // 4 bytes - normal map strength
    texture_flags: u32,         // 4 bytes - which textures are active
    _padding: [u32; 2],         // 8 bytes
}
// Total: 64 bytes per material
```

### 3. Shadow Mapping
**Current**: No shadows
**Future**: Dynamic shadow generation

- **Shadow map render passes**: Depth-only rendering from light perspective
- **Shadow sampling**: PCF (Percentage Closer Filtering) for soft shadows
- **Cascade shadow maps**: For large outdoor scenes with directional lights

### 4. Advanced Lighting Models
**Current**: Basic Lambert diffuse + ambient
**Future**: Physically-based lighting

- **Cook-Torrance BRDF**: Microfacet-based reflectance model
- **Image-Based Lighting**: Environment maps for realistic reflections
- **Fresnel effects**: View-angle dependent reflectance
- **Subsurface scattering**: For materials like skin, wax, marble

## 🔧 Code Maintenance

### Files That May Need Updates

1. **`renderer.rs`**: 
   - Add normal matrix caching
   - Extend for multiple lights
   - Add material system support

2. **Shaders**:
   - `vert.vert`: May need instance data support
   - `frag.vert`: Extend for multiple lights and materials

3. **Push Constants**:
   - Expand size limit considerations (currently 160 bytes)
   - Consider using uniform buffer objects for larger data

## 🎯 Architecture Decisions to Maintain

### 1. Coordinate Space Consistency
- **Keep**: All lighting calculations in world space
- **Reason**: Simplifies multi-light and shadow systems

### 2. Push Constants vs Uniform Buffers
- **Current**: Push constants for per-object data
- **Future**: Consider UBOs when data exceeds 256 bytes

### 3. Shader Modularity
- **Keep**: Separate vertex and fragment shaders
- **Future**: Consider compute shaders for advanced lighting

## 📝 Testing Checklist

When making future changes to the lighting system:

- [ ] Lighting direction stays fixed in world space during object rotation
- [ ] Normal transformations handle non-uniform scaling correctly  
- [ ] No visual artifacts during object rotation or camera movement
- [ ] Aspect ratio changes don't affect lighting calculations
- [ ] Performance acceptable for target frame rate with multiple lights
- [ ] Vulkan validation passes cleanly
- [ ] Works with different mesh geometries and normal configurations
- [ ] Material colors blend correctly with lighting calculations
- [ ] Ambient lighting provides proper base illumination
- [ ] Light intensity and color changes are immediately visible

## 📚 Related Systems

### Dependencies
- **Mesh Loading**: OBJ loader provides vertex normals for lighting calculations
- **Camera System**: View matrix and coordinate transforms affect final lighting appearance
- **Transform System**: Model matrix drives normal matrix calculation for proper light interaction
- **Coordinate System**: Works with Johannes Unterguggenberger guide-compliant projection matrices
- **Dynamic Viewport**: Integrates correctly with aspect ratio and window resizing systems

### Affected By
- **Asset Pipeline**: Normal quality depends on mesh generation and OBJ file vertex normals
- **Scene Management**: Object transforms affect lighting through normal matrix calculations  
- **Rendering Pipeline**: Depth testing, face culling, and viewport changes affect lighting visibility
- **Material System**: Material colors interact multiplicatively with lighting calculations

## 🎨 Visual Quality Considerations

### Current Quality: Excellent ✅
- Proper world-space lighting that stays consistent during object rotation
- Correct normal transformation handling all transform types including scaling
- Clean teapot rendering with realistic lighting falloff
- Proper ambient + directional lighting combination
- Material colors correctly modulated by lighting

### Future Quality Improvements:
- **Multiple Light Sources**: Scenes with complex lighting setups (key + fill + rim lighting)
- **Physically-Based Materials**: More realistic material appearance with metallic/roughness workflow
- **Dynamic Shadows**: Proper occlusion and depth for realistic scenes  
- **Image-Based Lighting**: Environment reflections for convincing material rendering
- **Advanced BRDF**: Cook-Torrance or similar for accurate light scattering
