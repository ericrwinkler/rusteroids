# Graphics Pipeline Redesign: Uniform Buffers and Descriptor Sets

## Current Problems

1. **Push Constants Limitation**: Limited to ~128 bytes, cramming everything into small space
2. **Poor Scalability**: Can't add more lights, materials, or complex data
3. **Inefficient Updates**: Pushing data every draw call instead of updating when changed
4. **No Multi-Material Support**: Single material per draw call hardcoded
5. **Lighting Limitations**: Only one directional light supported

## Proper Graphics Pipeline Architecture

### Uniform Buffer Objects (UBOs)

#### 1. Camera/View UBO (Per-Frame)
```rust
#[repr(C)]
struct CameraUniformData {
    view_matrix: Mat4,           // 64 bytes
    projection_matrix: Mat4,     // 64 bytes  
    view_projection_matrix: Mat4, // 64 bytes - pre-computed for efficiency
    camera_position: Vec4,       // 16 bytes (Vec3 + padding)
    camera_direction: Vec4,      // 16 bytes (Vec3 + padding)
    viewport_size: Vec2,         // 8 bytes
    near_far: Vec2,             // 8 bytes (near, far planes)
}
// Total: 240 bytes - updated once per frame
```

#### 2. Lighting UBO (Per-Frame)
```rust
#[repr(C)]
struct LightingUniformData {
    ambient_color: Vec4,         // 16 bytes - RGB + intensity
    directional_lights: [DirectionalLight; 4], // 4 directional lights max
    point_lights: [PointLight; 8],     // 8 point lights max  
    spot_lights: [SpotLight; 4],       // 4 spot lights max
    num_dir_lights: u32,         // 4 bytes
    num_point_lights: u32,       // 4 bytes
    num_spot_lights: u32,        // 4 bytes
    _padding: u32,               // 4 bytes
}

#[repr(C)]
struct DirectionalLight {
    direction: Vec4,             // 16 bytes (Vec3 + padding)
    color: Vec4,                 // 16 bytes (Vec3 + intensity)
}

#[repr(C)]
struct PointLight {
    position: Vec4,              // 16 bytes (Vec3 + radius)
    color: Vec4,                 // 16 bytes (Vec3 + intensity)
    attenuation: Vec4,           // 16 bytes (constant, linear, quadratic, _pad)
}

#[repr(C)]
struct SpotLight {
    position: Vec4,              // 16 bytes
    direction: Vec4,             // 16 bytes  
    color: Vec4,                 // 16 bytes
    params: Vec4,                // 16 bytes (inner_angle, outer_angle, attenuation, intensity)
}
```

#### 3. Material UBO (Per-Material or Dynamic)
```rust
#[repr(C)]
struct MaterialUniformData {
    base_color: Vec4,            // 16 bytes - RGBA
    metallic_roughness: Vec4,    // 16 bytes - metallic, roughness, ao, _pad
    emission: Vec4,              // 16 bytes - RGB emission + strength  
    normal_scale: f32,           // 4 bytes
    use_base_color_texture: u32, // 4 bytes - boolean flags
    use_normal_texture: u32,     // 4 bytes
    use_metallic_roughness_texture: u32, // 4 bytes
}
// Total: 64 bytes per material
```

#### 4. Push Constants (Per-Draw, kept minimal)
```rust
#[repr(C)]
struct PushConstants {
    model_matrix: Mat4,          // 64 bytes
    normal_matrix: Mat3,         // 36 bytes (+ 12 padding = 48)
    material_id: u32,            // 4 bytes - index into material buffer
    _padding: [u32; 3],          // 12 bytes padding
}
// Total: 128 bytes - exactly at push constant limit
```

### Descriptor Set Layout

```rust
// Set 0: Per-frame data (updated once per frame)
Binding 0: Camera UBO
Binding 1: Lighting UBO  

// Set 1: Per-material data (updated when material changes)
Binding 0: Material UBO (or dynamic UBO with multiple materials)
Binding 1: Base color texture sampler
Binding 2: Normal texture sampler  
Binding 3: Metallic-roughness texture sampler

// Set 2: Per-object data (rarely changes)
// (Could be used for object-specific data like bone matrices for skeletal animation)
```

## Implementation Strategy

### Phase 1: Core Infrastructure
1. Create UBO wrapper types with proper alignment
2. Implement descriptor set management
3. Create buffer allocation and update system
4. Update shaders to use uniform buffers

### Phase 2: Renderer Redesign  
1. Separate per-frame from per-draw operations
2. Implement material system with multiple materials
3. Add multiple light support
4. Implement efficient data update patterns

### Phase 3: Advanced Features
1. Dynamic uniform buffers for batching
2. Texture array support for materials
3. Shadow mapping preparation
4. PBR material model

## Benefits

1. **Scalability**: Support unlimited materials and many lights
2. **Performance**: Reduce data uploads, batch similar objects  
3. **Flexibility**: Easy to add new uniform data without size limits
4. **Modern**: Standard approach used by real graphics engines
5. **Maintainable**: Clean separation of concerns

## Migration Path

1. Keep existing push constant system working
2. Implement UBO system alongside
3. Gradually migrate features to UBOs
4. Remove push constant dependencies
5. Optimize and clean up

This provides a proper foundation for a real graphics engine rather than a toy renderer.
