# Rusteroids Engine Architecture

## ⚠️ **CURRENT STATUS** 

**Architecture State**: Mixed - Core systems working, multiple objects rendering has critical performance issues
**Performance**: 60+ FPS single object, ~10 FPS multiple objects due to architectural problems
**Priority**: Fix multiple objects rendering (immediate mode → command recording pattern)

## Overview

Rusteroids is a modular ECS-based Vulkan rendering engine designed for 2D/3D games. The engine follows separation of concerns with distinct modules for ECS, rendering, input, audio, and asset management.

## Core Design Principles

### 1. **Separation of Concerns**
- **ECS Layer**: Entity management, component storage, system execution
- **Rendering Layer**: Vulkan backend, material system, pipeline management  
- **Foundation Layer**: Math utilities, coordinate systems, transforms
- **Platform Layer**: Window management, input handling, file I/O

### 2. **Safety & Performance**
- **Memory Safety**: RAII patterns, single ownership, automatic cleanup
- **Performance**: Zero-allocation render loops, GPU-efficient data layouts
- **Validation**: Comprehensive error handling, Vulkan validation layer integration

### 3. **Modularity**
- **Backend Agnostic**: Rendering abstracted through trait system
- **Asset Format Agnostic**: Support for multiple model/texture formats
- **Platform Agnostic**: Windows/Linux support through abstraction layers

## Project Structure

```
rusteroids/
├── crates/
│   ├── rust_engine/                 # Core engine library
│   │   └── src/
│   │       ├── ecs/                 # Entity-Component-System
│   │       ├── render/              # Vulkan rendering backend
│   │       ├── foundation/          # Math, transforms, utilities
│   │       ├── assets/              # Asset loading & management
│   │       ├── audio/               # Audio system (planned)
│   │       ├── input/               # Input handling
│   │       └── platform/            # Platform abstraction
│   └── asteroids/                   # Game-specific code
├── teapot_app/                      # Test application
├── tools/                           # Development utilities
├── resources/                       # Game assets
│   ├── models/                      # 3D models (.obj)
│   ├── shaders/                     # GLSL shaders
│   └── textures/                    # Image assets
└── docs/                           # Documentation
```

## Architecture Details

### Entity-Component-System (ECS)

#### Core Components
```rust
// Working ECS foundation
pub struct World {
    entities: EntityStorage,
    components: ComponentManager,
}

pub struct Entity {
    id: EntityId,
    generation: u32,
}

pub trait Component: 'static + Send + Sync {}
pub trait System {
    fn update(&mut self, world: &mut World, delta_time: f32);
}
```

#### Implemented Components
- **TransformComponent**: Position, rotation, scale in world space
- **LightComponent**: Light properties (directional, point, spot)
- **RenderableComponent**: Mesh and material references (planned)

#### Working Systems
- **LightingSystem**: Processes light entities, sends data to renderer
- **CoordinateValidation**: Ensures coordinate system consistency

### Rendering Architecture 

#### ✅ **Working Systems**

**Vulkan Backend**:
```rust
pub struct Renderer {
    backend: Box<dyn RenderBackend>,
    material_manager: MaterialManager,
    current_camera: Option<Camera>,
    current_lighting: Option<LightingEnvironment>,
}
```

**Pipeline Management**:
- StandardPBR, Unlit, TransparentPBR, TransparentUnlit pipelines
- Automatic shader compilation (GLSL → SPIR-V)
- Material-based pipeline selection

**UBO Architecture**:
- **Set 0**: Per-frame data (camera matrices, lighting)
- **Set 1**: Per-material data (material properties, textures)

#### ⚠️ **Critical Issue: Multiple Objects Rendering**

**Problem**: Immediate mode rendering pattern prevents efficient multiple object rendering.

**Current Broken Pattern**:
```rust
// Each call submits frame immediately - prevents batching
pub fn draw_mesh_3d(&mut self, mesh: &Mesh, transform: &Mat4, material: &Material) -> Result<()> {
    self.backend.update_mesh(&mesh.vertices, &mesh.indices)?;  // Upload every call
    self.backend.set_model_matrix(transform_array);
    self.backend.draw_frame()?;  // ⚠️ SUBMITS IMMEDIATELY
    Ok(())
}
```

**Required Fix**: Command recording pattern per Vulkan tutorial:
```rust
// Proper Vulkan multiple objects pattern
pub fn begin_frame(&mut self) -> Result<()> {
    // Start command buffer recording
}

pub fn draw_object(&mut self, object: &GameObject) -> Result<()> {
    // Record draw command (no submission)
    self.command_buffer.bind_descriptor_set(&object.descriptor_set);
    self.command_buffer.draw_indexed(mesh.index_count);
}

pub fn end_frame(&mut self, window: &mut Window) -> Result<()> {
    // Submit all commands at once
}
```

### Coordinate System

#### World Space: Y-Up Right-Handed
```
Y (up)
|
|
+------ X (right)
/
Z (toward viewer)
```

#### Transformation Pipeline
```
Local → World → View → Vulkan Clip → NDC
  ↓       ↓       ↓         ↓        ↓
Model   World   View   Projection  Screen
Matrix  Matrix  Matrix   Matrix    Coords
```

**Mathematical Implementation**: Follows Johannes Unterguggenberger's academic approach for Vulkan coordinate transformation.

### Material System

#### Material Types
```rust
pub enum MaterialType {
    StandardPBR,     // Physically-based rendering
    Unlit,           // No lighting calculations
    TransparentPBR,  // Alpha blended PBR
    TransparentUnlit,// Alpha blended unlit
}
```

#### Material Properties
```rust
#[repr(C, align(16))]
pub struct StandardMaterialParams {
    pub base_color: Vec3,
    pub alpha: f32,
    pub metallic: f32,
    pub roughness: f32,
    pub normal_scale: f32,
    pub occlusion_strength: f32,
}
```

### Performance Characteristics

#### Current Benchmarks
- **Single Object**: 60+ FPS with 18,960 vertex teapot
- **Multiple Objects**: ~10 FPS due to architectural issues  
- **Memory**: Efficient RAII-based resource management
- **Validation**: Clean Vulkan validation (zero errors)

#### Resource Management
- **RAII Pattern**: Automatic cleanup via Drop implementations
- **Single Ownership**: Clear ownership hierarchy
- **Buffer Management**: Efficient GPU memory allocation

## Build System

### Shader Compilation
```rust
// build.rs - automatic GLSL → SPIR-V compilation
fn compile_shaders() {
    // Input: resources/shaders/*.{vert,frag}
    // Output: target/shaders/*.spv
}
```

### Dependencies
**Core Dependencies**:
- `ash`: Vulkan bindings
- `nalgebra`: Linear algebra
- `glfw`: Window management
- `bytemuck`: Safe transmutation

**Policy**: Minimal external dependencies, prefer custom implementations for control and ECS compatibility.

## Development Standards

### Performance Requirements
- **60+ FPS** minimum with current scene complexity
- **Clean Vulkan validation** in development builds
- **Memory efficiency** - no leaks, proper resource cleanup

### Code Quality
- **Memory Safety**: Leverage Rust's ownership system
- **Error Handling**: Comprehensive Result types
- **Documentation**: Public APIs fully documented
- **Testing**: Unit tests for core systems

## Future Architecture Plans

### Short Term (Immediate)
1. **Fix Multiple Objects Rendering**: Implement command recording pattern
2. **Per-Object Uniform Buffers**: Enable different transforms per object
3. **Performance Optimization**: Achieve 60+ FPS with 10+ objects

### Medium Term
1. **Asset Pipeline**: Comprehensive asset loading system
2. **Scene Management**: High-level scene graph abstraction
3. **Threading**: Multi-threaded system execution

### Long Term
1. **Multiple Backends**: DirectX 12, Metal support
2. **Advanced Rendering**: PBR lighting, shadows, post-processing
3. **Game Framework**: Complete game development toolkit
