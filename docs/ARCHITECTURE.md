# Rusteroids Architecture Documentation

## Overview

This document provides comprehensive technical architecture documentation for the Rusteroids Vulkan rendering engine. It consolidates all architectural decisions, implementation patterns, and technical design details into a single authoritative source.

## Current System Status

### Infrastructure Complete ✅
The Rusteroids project has successfully completed its foundational infrastructure phase with:
- **Material System**: Complete modular architecture with MaterialType enum, MaterialUBO structs, MaterialManager
- **Pipeline Management**: PipelineManager supporting multiple pipeline types (StandardPBR, Unlit, TransparentPBR, TransparentUnlit)
- **UBO Architecture**: Per-frame camera and lighting UBOs integrated with descriptor sets
- **Vulkan Validation**: Clean validation with no errors, application runs successfully

### Performance & Quality ✅
- **Frame Rate**: Smooth 60+ FPS with 18,960 vertex teapot model
- **Academic Standards**: Johannes Unterguggenberger guide-compliant projection matrices
- **Aspect Ratio System**: Perfect dynamic viewport handling during window resizing
- **World-Space Lighting**: Fixed directional lighting with proper normal matrix transformations

## Core Design Principles

### 1. Ownership and Resource Management
- **Single Owner Rule**: Every Vulkan resource has exactly one owner
- **RAII Pattern**: All resources wrapped in structs with Drop implementations
- **No Raw Handles**: Never expose raw Vulkan handles outside their owning struct
- **Move Semantics**: Resources are moved, not cloned or shared

### 2. Data Flow and Interface Design
- **Immutable Interfaces**: Public APIs take `&self` or `&mut self`
- **Typed Parameters**: Strong typing instead of generic parameters where possible
- **Error Propagation**: All Vulkan operations return `Result<T, VulkanError>`
- **Builder Pattern**: Complex object creation with validation

### 3. Module Separation
- **Single Responsibility**: Each module handles exactly one concern
- **Dependency Injection**: Higher-level modules depend on abstractions
- **No Circular Dependencies**: Strict dependency hierarchy
- **Interface Segregation**: Small, focused traits rather than large interfaces

## System Architecture

### Multi-Crate Workspace Structure
```
rusteroids/
├── crates/
│   ├── rust_engine/           # Generic engine (rendering, ECS, etc.)
│   └── asteroids/             # Game-specific implementation
├── teapot_app/               # Rendering test application
├── tools/                    # Development utilities
├── resources/                # Assets (shaders, models, textures)
└── docs/                     # Architecture documentation
```

### Material System Architecture
```
crates/rust_engine/src/render/material/
├── material_type.rs      # MaterialType enum with 4 types
├── material_ubo.rs       # UBO structures for GPU data
├── material_manager.rs   # Resource management system
├── texture_manager.rs    # Placeholder for texture system
└── mod.rs               # Public API exports
```

**Material Types**:
- `StandardPBR`: Full physically-based rendering with metallic-roughness workflow
- `Unlit`: No lighting calculations, direct color output
- `TransparentPBR`: PBR with alpha blending support
- `TransparentUnlit`: Unlit with transparency

### Pipeline Management System
```
crates/rust_engine/src/render/pipeline/
├── pipeline_manager.rs   # Multi-pipeline management
├── pipeline_config.rs    # Configuration and shader paths
└── mod.rs               # Public API exports
```

**Pipeline Architecture**:
```rust
pub struct PipelineManager {
    pipelines: HashMap<MaterialType, GraphicsPipeline>,
    active_pipeline: Option<MaterialType>,
}

pub enum MaterialType {
    StandardPBR,
    Unlit,
    TransparentPBR,
    TransparentUnlit,
}
```

### UBO Architecture (Operational)

#### Set 0: Per-Frame Data (Implemented)
```glsl
layout(set = 0, binding = 0) uniform CameraUBO {
    mat4 view_matrix;
    mat4 projection_matrix;
    vec2 near_far;
} camera;

layout(set = 0, binding = 1) uniform LightingUBO {
    vec4 ambient_color;
    vec4 light_direction;
    vec4 light_color;
    float light_intensity;
} lighting;
```

#### Set 1: Material Data (Prepared for Implementation)
```glsl
layout(set = 1, binding = 0) uniform MaterialUBO {
    vec4 base_color;
    vec4 metallic_roughness;
    vec4 emission;
    float normal_scale;
    uint texture_flags;
} material;

layout(set = 1, binding = 1) uniform sampler2D base_color_texture;
layout(set = 1, binding = 2) uniform sampler2D normal_texture;
layout(set = 1, binding = 3) uniform sampler2D metallic_roughness_texture;
```

## Vulkan Implementation Details

### Coordinate System Standards
Following Johannes Unterguggenberger's academic guide for proper Vulkan projection matrices:

1. **View Space**: Standard Y-up coordinate system
2. **Vulkan Coordinate Transform**: Intermediate transform matrix for Vulkan conventions
3. **Complete Transform Chain**: `C = P × X × V × M`

```rust
// Proper projection matrix (guide-compliant)
result[(1, 1)] = 1.0 / tan_half_fovy;        // Positive Y
result[(2, 2)] = far / (far - near);         // Correct depth to [0,1]
result[(2, 3)] = -(near * far) / (far - near); // Correct depth offset
result[(3, 2)] = 1.0;                        // Perspective divide trigger

// Coordinate transform for Vulkan conventions
fn vulkan_coordinate_transform() -> Mat4 {
    // X matrix - flips Y and Z for Vulkan conventions
}
```

### Lighting System Implementation

#### World-Space Lighting Architecture
- **Normal Matrix**: Proper inverse-transpose calculation with GLSL compatibility
- **Coordinate Space Consistency**: All lighting calculations in world space
- **Matrix Storage**: Explicit transpose for nalgebra to GLSL conversion

```rust
// Normal matrix calculation (fixed implementation)
let normal_matrix = if let Some(inverse) = mat3_model.try_inverse() {
    inverse.transpose()
} else {
    Matrix3::identity()
};

// Matrix storage with proper transposition
let transposed = normal_matrix.transpose();
self.push_constants.normal_matrix = [
    [transposed[(0, 0)], transposed[(1, 0)], transposed[(2, 0)], 0.0],
    [transposed[(0, 1)], transposed[(1, 1)], transposed[(2, 1)], 0.0],
    [transposed[(0, 2)], transposed[(1, 2)], transposed[(2, 2)], 0.0],
];
```

#### Current Lighting Limitations
- **Single Light Only**: Limited to one directional light via push constants
- **Push Constant Bottleneck**: 160-byte limit prevents complex lighting
- **No Shadows**: No shadow casting or receiving capabilities

### Aspect Ratio and Viewport Management

#### Dynamic Viewport System (Implemented)
- **Problem Solved**: Graphics pipeline with static viewport caused visual squishing
- **Solution**: Dynamic viewport/scissor setting during rendering

```rust
// Dynamic viewport configuration
let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
let dynamic_state = vk::PipelineDynamicStateCreateInfo::builder()
    .dynamic_states(&dynamic_states);

// Runtime viewport setting
let viewport = vk::Viewport::builder()
    .x(0.0)
    .y(0.0)
    .width(extent.width as f32)
    .height(extent.height as f32)
    .min_depth(0.0)
    .max_depth(1.0);

device.cmd_set_viewport(command_buffer, 0, &[viewport.build()]);
```

## Resource Management Patterns

### RAII Implementation
All Vulkan resources use Rust's RAII patterns:

```rust
pub struct VulkanBuffer {
    buffer: vk::Buffer,
    memory: vk::DeviceMemory,
    device: Arc<ash::Device>,
}

impl Drop for VulkanBuffer {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_buffer(self.buffer, None);
            self.device.free_memory(self.memory, None);
        }
    }
}
```

### Synchronization Architecture
Based on Hans-Kristian Arntzen's Vulkan synchronization best practices:

#### Frame-in-Flight Management
```rust
pub struct FrameSync {
    fence: vk::Fence,
    command_buffer: vk::CommandBuffer,
    image_available: vk::Semaphore,
    render_finished: vk::Semaphore,
}

// Per-frame synchronization (for frames in flight)
frame_sync_objects: Vec<FrameSync>,
// Per-image synchronization (one per swapchain image)
image_sync_objects: Vec<FrameSync>,
```

#### Pipeline Barriers
Proper execution and memory dependencies:
```rust
// Example: Buffer update synchronization
recorder.cmd_pipeline_barrier(
    vk::PipelineStageFlags::VERTEX_INPUT,        // srcStageMask: wait for vertex reads
    vk::PipelineStageFlags::TRANSFER,            // dstStageMask: unblock transfers
    vk::DependencyFlags::empty(),
    &[],
    &[memory_barrier],
    &[],
);
```

## Rendering Pipeline Flow

### Complete Frame Rendering Process
1. **Frame Setup**: Update camera/lighting UBOs with current frame data
2. **Pipeline Selection**: Choose pipeline based on material type
3. **Material Binding**: Set active pipeline and bind descriptor sets
4. **Object Rendering**: Draw objects with model matrix push constants
5. **Present**: Swapchain presentation with proper synchronization

### Push Constants Layout (Current)
```rust
#[repr(C)]
struct PushConstants {
    mvp_matrix: [[f32; 4]; 4],      // 64 bytes - model-view-projection
    normal_matrix: [[f32; 4]; 3],   // 48 bytes - normal transformation
    material_color: [f32; 4],       // 16 bytes - basic material color
    light_direction: [f32; 4],      // 16 bytes - world-space light direction
    light_color: [f32; 4],          // 16 bytes - light color and intensity
}
// Total: 160 bytes - near push constant limit
```

## Performance Characteristics

### Current Benchmarks
- **Frame Rate**: Consistent 60+ FPS with 18,960 vertex teapot
- **Memory Usage**: Efficient Vulkan resource management with minimal allocations
- **Validation**: Clean Vulkan validation (only performance warnings about small allocations)
- **Resize Performance**: Smooth window resizing with proper aspect ratio maintenance

### Optimization Opportunities
1. **Normal Matrix Caching**: Avoid recalculation when model matrix unchanged
2. **Batch Rendering**: Group objects by material type for fewer pipeline switches
3. **UBO Expansion**: Move to uniform buffers to eliminate push constant limitations
4. **Light Culling**: Disable distant/weak lights in complex lighting scenarios

## Future Architecture Considerations

### Multi-Light System Expansion
Planned UBO-based architecture for multiple light sources:

```glsl
layout(set = 0, binding = 1) uniform LightingUBO {
    vec4 ambient_color;
    DirectionalLight directional_lights[4];
    PointLight point_lights[8];
    SpotLight spot_lights[4];
    uvec4 light_counts;
    mat4 shadow_matrices[4];
} lighting;
```

### Shadow Mapping Foundation
Infrastructure for dynamic shadow generation:
- **Shadow Map Resources**: Depth textures and framebuffer management
- **Shadow Coordinate Calculation**: Light-space transform matrices
- **PCF Implementation**: Percentage Closer Filtering for soft shadows

### Advanced Material Workflows
Complete PBR (Physically Based Rendering) implementation:
- **Cook-Torrance BRDF**: Microfacet-based reflectance model
- **Image-Based Lighting**: Environment maps for realistic reflections
- **Texture System**: GPU texture loading with atlas support
- **Material Properties**: Full metallic-roughness workflow

## Architecture Quality Metrics

### Infrastructure Strengths ✅
- **Modularity**: Each system can be developed/tested independently
- **Scalability**: Pipeline system supports unlimited material types
- **Standards Compliance**: Academic projection matrices and Vulkan best practices
- **Maintainability**: Clear ownership patterns and comprehensive documentation
- **Performance**: Efficient for current feature set with room for optimization

### Current Limitations
- Push constant size limitations (160-byte limit)
- Single light source constraint
- No shadow mapping capabilities
- Placeholder texture system

### Success Criteria
- Clean Vulkan validation at all development stages
- 60+ FPS maintained through feature additions
- Academic compliance and industry best practices
- Clear ownership and responsibility patterns

This architecture provides a solid foundation for advanced rendering features while maintaining clean, maintainable code patterns and industry-standard Vulkan implementation practices.
