# Rusteroids Engine Architecture

## ⚠️ **CURRENT STATUS** 

**Architecture State**: Core systems working, single-mesh dynamic system architecture finalized
**Performance**: 60+ FPS single object, implementing high-performance single-mesh pools
**Priority**: Implement Single-Mesh Pool System for optimal batching performance

## Overview

Rusteroids is a modular ECS-based Vulkan rendering engine designed for 2D/3D games. The engine follows separation of concerns with distinct modules for ECS, rendering, input, audio, and asset management. The architecture supports both static scene objects and dynamic temporary objects through separate but integrated rendering pathways.

## Core Design Principles

### 1. **Separation of Concerns**
- **ECS Layer**: Entity management, component storage, system execution
- **Rendering Layer**: Dual-path rendering (static objects + dynamic objects)
- **Foundation Layer**: Math utilities, coordinate systems, transforms
- **Platform Layer**: Window management, input handling, file I/O

### 2. **Memory Safety & Performance**
- **Memory Safety**: RAII patterns, single ownership, automatic cleanup
- **Performance**: Zero-allocation render loops, GPU-efficient data layouts
- **Resource Management**: Pre-allocated pools for dynamic objects, permanent allocation for static objects
- **Validation**: Comprehensive error handling, Vulkan validation layer integration

### 3. **Single-Mesh Pool Architecture**
- **Mesh-Specific Pools**: Separate DynamicObjectManager for each mesh type (teapots, spheres, cubes)
- **Optimal Batching**: Each pool uses single instanced draw call for maximum performance
- **Pool Management**: MeshPoolManager coordinates multiple single-mesh pools
- **Performance**: O(1) rendering per mesh type, minimal state changes

## Project Structure

```
rusteroids/
├── crates/
│   ├── rust_engine/                 # Core engine library
│   │   └── src/
│   │       ├── ecs/                 # Entity-Component-System
│   │       ├── render/              # Dual-path Vulkan rendering
│   │       │   ├── dynamic/         # Single-mesh pool rendering system
│   │       │   │   ├── pool_manager.rs      # MeshPoolManager - coordinates multiple pools
│   │       │   │   ├── object_manager.rs    # DynamicObjectManager - single mesh type
│   │       │   │   ├── instance_renderer.rs # Batch rendering pipeline
│   │       │   │   ├── resource_pool.rs     # Memory pools
│   │       │   │   └── mod.rs              # Public interface
│   │       │   ├── game_object.rs   # Static object system
│   │       │   ├── shared_resources.rs     # Common rendering resources
│   │       │   └── material/        # Material management
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
    ├── ARCHITECTURE.md              # This document
    ├── TODO.md                      # Implementation roadmap
    └── TROUBLESHOOTING.md           # Known issues and solutions
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
- **DynamicRenderComponent**: Handle to dynamic object in pool system
- **RenderableComponent**: Mesh and material references for static objects

#### Working Systems
- **LightingSystem**: Processes light entities, sends data to renderer
- **DynamicRenderSystem**: Updates dynamic objects and manages lifecycle
- **CoordinateValidation**: Ensures coordinate system consistency

### Dual-Path Rendering Architecture

#### Static Object Rendering (GameObject System)

**Purpose**: Permanent scene objects with persistent GPU resources

```rust
pub struct GameObject {
    // Transform properties
    pub position: Vec3,
    pub rotation: Vec3,
    pub scale: Vec3,
    
    // Persistent GPU resources (one per frame-in-flight)
    pub uniform_buffers: Vec<Buffer>,
    pub uniform_mapped: Vec<*mut u8>,
    pub descriptor_sets: Vec<vk::DescriptorSet>,
    
    // Material for this object
    pub material: Material,
}
```

**Characteristics**:
- ✅ Permanent GPU resource allocation at creation
- ✅ Individual uniform buffers and descriptor sets per object
- ✅ Suitable for static scene geometry, UI elements, persistent entities
- ✅ Command recording pattern for batch rendering

**API Usage**:
```rust
// Create static objects (buildings, terrain, UI)
let static_object = graphics_engine.create_static_object(position, rotation, scale, material)?;

// Render multiple static objects efficiently
graphics_engine.begin_frame()?;
graphics_engine.bind_shared_resources(&shared_resources)?;
for object in &static_objects {
    graphics_engine.draw_object(object, frame_index)?;
}
graphics_engine.end_frame(&mut window)?;
```

#### Single-Mesh Pool System

**Purpose**: High-performance rendering with separate pools per mesh type

```rust
pub struct MeshPoolManager {
    // Multiple single-mesh pools
    pools: HashMap<MeshType, DynamicObjectManager>,
    
    // Shared per-frame resources
    camera_ubo: Buffer,
    lighting_ubo: Buffer,
}

pub struct DynamicObjectManager {
    // Single mesh type for this pool
    mesh_type: MeshType,
    shared_resources: SharedRenderingResources, // Specific to this mesh
    
    // Pre-allocated object pools
    object_pool: ObjectPool<DynamicRenderData>,
    transform_pool: Pool<TransformComponent>,
    material_pool: Pool<MaterialInstance>,
    
    // Active dynamic objects this frame
    active_objects: Vec<DynamicObjectHandle>,
    
    // Instance data for GPU upload (all same mesh type)
    instance_buffer: Buffer,
    max_instances: usize, // Pre-allocated limit (e.g., 100 per mesh type)
}
```

**Characteristics**:
- ✅ One DynamicObjectManager per mesh type (teapots, spheres, cubes)
- ✅ Each pool has dedicated SharedRenderingResources for optimal batching
- ✅ Single instanced draw call per mesh type for maximum performance  
- ✅ O(1) rendering complexity per mesh type regardless of object count
- ✅ Minimal state changes - only vertex buffer binding between mesh types

**API Usage**:
```rust
// Create separate pools for each mesh type
let mut pool_manager = MeshPoolManager::new();
pool_manager.create_pool(MeshType::Teapot, teapot_shared_resources)?;
pool_manager.create_pool(MeshType::Sphere, sphere_shared_resources)?;

// Spawn objects into appropriate pools
let teapot_handle = pool_manager.spawn_object(
    MeshType::Teapot, 
    position, rotation, scale, 
    material_properties, lifetime
)?;

// Automatic optimal rendering
pool_manager.render_all_pools(frame_index)?;
```

**Data Structures**:
```rust
#[repr(C)]
pub struct DynamicInstanceData {
    model_matrix: Mat4,
    normal_matrix: Mat3,
    material_index: u32,
    _padding: [f32; 3],
}

pub struct DynamicRenderObject {
    // Logical properties
    entity_id: Option<Entity>,
    transform: TransformComponent,
    material_instance: MaterialInstance,
    
    // Render state
    pool_handle: PoolHandle,
    instance_index: u32,
    is_active: bool,
    
    // Lifecycle
    spawn_time: Instant,
    lifetime: f32,
}

pub struct MaterialInstance {
    base_material: MaterialId,
    instance_properties: MaterialProperties,
    descriptor_set_offset: u32,
}
```

**Characteristics**:
- ✅ Pre-allocated resource pools (no runtime allocation)
- ✅ Handle-based access with generation counters
- ✅ Automatic lifecycle management with timeout cleanup
- ✅ Batch rendering with instanced draws
- ✅ Suitable for particles, projectiles, temporary effects, dynamic spawning

**API Usage**:
```rust
// Spawn dynamic objects (particles, projectiles, temporary entities)
let handle = graphics_engine.spawn_dynamic_object(
    position, rotation, scale, 
    material_properties,
    lifetime_seconds
)?;

// Automatic batch rendering workflow
graphics_engine.begin_dynamic_frame()?;
// Objects spawn/update/despawn automatically
graphics_engine.record_dynamic_draws()?;
graphics_engine.end_dynamic_frame()?;
```

### Memory Management Strategy

#### Static Objects (GameObject System)
```rust
// Memory allocation pattern
GameObject::new() → create_resources() → permanent GPU allocation
    ↓
Per-object uniform buffers × frames_in_flight
Per-object descriptor sets × frames_in_flight
Persistent until explicit destruction
```

#### Dynamic Objects (Pool System)
```rust
// Memory allocation pattern
Startup: Pre-allocate pools for max_objects (e.g., 100)
    ↓
Runtime: Handle-based allocation from pools
    ↓
Cleanup: Automatic return to pool on lifetime expiration
```

**Pool Configuration**:
```rust
pub struct PoolConfiguration {
    max_dynamic_objects: usize = 100,    // Per mesh type capacity
    max_materials: usize = 20,           # Material variety per mesh type
    frames_in_flight: usize = 2,         # Standard double buffering
    
    // Calculated buffer sizes per pool
    instance_buffer_size: usize = max_dynamic_objects * size_of::<DynamicInstanceData>(),
    uniform_buffer_size: usize = max_dynamic_objects * MAX_UNIFORM_SIZE,
}
```

#### Resource Lifecycle Management
```rust
pub enum ResourceState {
    Available,      // In pool, ready for allocation
    Active,         // Assigned to dynamic object
    PendingCleanup, // Marked for return to pool next frame
}

pub struct ResourceHandle {
    pool_id: PoolId,
    index: u32,
    generation: u32,  // Prevents use-after-free
}
```

### Vulkan Backend Implementation

#### Descriptor Set Layouts
```rust
// Set 0: Per-frame data (shared by all objects)
layout(set = 0, binding = 0) uniform CameraUBO {
    mat4 view_matrix;
    mat4 projection_matrix;
    vec4 camera_position;
    // ...
} camera;

layout(set = 0, binding = 1) uniform LightingUBO {
    vec4 ambient_color;
    vec4 directional_light_direction;
    vec4 directional_light_color;
    // ...
} lighting;

// Set 1: Per-object data (different for static vs dynamic)
// Static: Individual descriptor sets per GameObject
// Dynamic: Instance buffer with dynamic offsets
```

#### Command Buffer Recording Patterns

**Static Objects**:
```rust
fn record_static_objects() -> VulkanResult<()> {
    // Bind per-frame descriptor set (camera + lighting)
    cmd_bind_descriptor_sets(set_0_frame_data);
    
    // For each static object
    for object in static_objects {
        // Bind per-object descriptor set
        cmd_bind_descriptor_sets(object.descriptor_set);
        // Record draw command
        cmd_draw_indexed(object.index_count);
    }
}
```

**Single-Mesh Pools**:
```rust
fn record_single_mesh_pool(pool: &DynamicObjectManager) -> VulkanResult<()> {
    // Bind per-frame descriptor set (camera + lighting)
    cmd_bind_descriptor_sets(set_0_frame_data);
    
    // Bind mesh-specific vertex buffer (teapots, spheres, or cubes)
    cmd_bind_vertex_buffers(pool.shared_resources.vertex_buffer);
    cmd_bind_index_buffer(pool.shared_resources.index_buffer);
    
    // Upload instance data to GPU (all same mesh type)
    update_instance_buffer(pool.active_objects);
    
    // Single optimized instanced draw call for this mesh type
    cmd_draw_indexed_instanced(pool.instance_count);
}

fn render_all_pools() -> VulkanResult<()> {
    for (mesh_type, pool) in &mesh_pools {
        record_single_mesh_pool(pool)?; // One draw call per mesh type
    }
}
```

### Performance Characteristics

#### Target Metrics
- **Static Objects**: 60+ FPS with 100+ persistent objects
- **Dynamic Objects**: 60+ FPS with 50+ temporary objects  
- **Memory Usage**: Bounded by pool sizes, no runtime allocation
- **GPU Memory**: Efficient batching, minimal descriptor set allocations

#### Scalability Analysis
```rust
// Static Objects: O(n) where n = number of objects
// - Memory: n × uniform_buffer_size × frames_in_flight
// - Performance: n × descriptor_set_bind + n × draw_call

// Single-Mesh Pools: O(m) where m = number of active mesh types
// - Memory: m × pool_size × uniform_buffer_size (pre-allocated)
// - Performance: m × vertex_buffer_bind + m × instanced_draw_call
// - Objects per pool: O(1) rendering regardless of count (up to pool limit)
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

#### Static Material Properties
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

#### Dynamic Material Instances
```rust
pub struct MaterialInstance {
    base_material: MaterialId,
    instance_properties: MaterialProperties,
    descriptor_set_offset: u32,
}

pub struct MaterialProperties {
    base_color: Vec3,
    metallic: f32,
    roughness: f32,
    emission: Vec3,
    // Runtime-modifiable properties for dynamic objects
}
```

## Integration Patterns

### Single-Mesh Pool + Static Rendering Workflow
```rust
fn render_frame() -> Result<()> {
    // Begin frame setup
    graphics_engine.begin_frame()?;
    
    // Set per-frame data (camera, lighting)
    graphics_engine.set_camera(&camera)?;
    graphics_engine.set_lighting(&lighting_environment)?;
    
    // Render static objects (buildings, terrain, UI)
    graphics_engine.render_static_objects(&static_scene)?;
    
    // Render each mesh type pool efficiently
    graphics_engine.render_teapot_pool()?;  // Single draw call for all teapots
    graphics_engine.render_sphere_pool()?;  // Single draw call for all spheres  
    graphics_engine.render_cube_pool()?;    // Single draw call for all cubes
    
    // Submit commands and present
    graphics_engine.end_frame(&mut window)?;
}
```

### ECS Integration
```rust
// Components
#[derive(Component)]
pub struct StaticRenderComponent {
    game_object: GameObject,
    material_override: Option<Material>,
}

#[derive(Component)]
pub struct DynamicRenderComponent {
    mesh_type: MeshType,              // Which pool this object belongs to
    object_handle: DynamicObjectHandle,
    material_override: Option<MaterialInstance>,
    render_flags: RenderFlags,
}

// Systems
pub struct StaticRenderSystem { /* ... */ }
pub struct DynamicRenderSystem { 
    pool_manager: MeshPoolManager,    // Manages multiple single-mesh pools
}

impl System for DynamicRenderSystem {
    fn update(&mut self, world: &mut World, delta_time: f32) {
        // Process entities with DynamicRenderComponent
        // Route to appropriate mesh pool based on mesh_type
        // Handle spawning/despawning with optimal batching per mesh type
    }
}
```

### Error Handling and Safety

#### Resource Exhaustion Handling
```rust
pub enum DynamicRenderError {
    PoolExhausted { pool_type: PoolType, requested: usize, available: usize },
    InvalidHandle { handle: DynamicObjectHandle, reason: String },
    VulkanError(VulkanError),
}

impl DynamicObjectManager {
    pub fn spawn_object(&mut self, params: SpawnParams) -> Result<DynamicObjectHandle, DynamicRenderError> {
        // Check pool availability
        if self.object_pool.available_count() == 0 {
            return Err(DynamicRenderError::PoolExhausted {
                pool_type: PoolType::Object,
                requested: 1,
                available: 0,
            });
        }
        
        // Safe allocation with error handling
        let handle = self.object_pool.allocate()?;
        // ...
    }
}
```

#### Vulkan Validation Integration
```rust
// Debug builds enable comprehensive validation
#[cfg(debug_assertions)]
const ENABLE_VALIDATION_LAYERS: bool = true;

// Validation layer configuration for dynamic objects
fn setup_dynamic_object_validation() {
    // Track descriptor set allocations/deallocations
    // Monitor buffer usage patterns
    // Validate resource lifecycle management
}
```

## Build System & Development

### Shader Compilation
```rust
// build.rs - automatic GLSL → SPIR-V compilation
fn compile_shaders() {
    // Input: resources/shaders/*.{vert,frag}
    // Output: target/shaders/*.spv
    // Compile variants for static vs dynamic rendering
}
```

### Dependencies
**Core Dependencies**:
- `ash`: Vulkan bindings
- `nalgebra`: Linear algebra
- `glfw`: Window management
- `bytemuck`: Safe transmutation

**Policy**: Minimal external dependencies, prefer custom implementations for control and ECS compatibility.

### Testing Strategy
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_dynamic_object_lifecycle() {
        // Test spawn → update → despawn cycle
        // Verify pool state consistency
        // Check resource cleanup
    }
    
    #[test]
    fn test_resource_pool_exhaustion() {
        // Test graceful handling of pool limits
        // Verify error reporting
        // Check recovery behavior
    }
    
    #[test]
    fn test_mixed_static_dynamic_rendering() {
        // Test simultaneous static + dynamic objects
        // Verify performance characteristics
        // Check visual correctness
    }
}
```

## Performance Analysis

### Current Benchmarks
- **Single Static Object**: 60+ FPS with 18,960 vertex teapot
- **Multiple Static Objects**: Target 60+ FPS with optimized command recording
- **Dynamic Objects**: Target 60+ FPS with 50+ simultaneous objects
- **Memory**: Bounded allocation, predictable usage patterns

### Memory Usage Projections
```rust
// Static Objects (conservative estimate)
const STATIC_OBJECTS: usize = 100;
const STATIC_MEMORY_PER_OBJECT: usize = 1024; // 1KB uniform buffer
const STATIC_TOTAL_MEMORY: usize = STATIC_OBJECTS * STATIC_MEMORY_PER_OBJECT * FRAMES_IN_FLIGHT;

// Dynamic Objects (pool-based)
const DYNAMIC_POOL_SIZE: usize = 100;
const DYNAMIC_MEMORY_PER_SLOT: usize = 256; // 256B instance data
const DYNAMIC_TOTAL_MEMORY: usize = DYNAMIC_POOL_SIZE * DYNAMIC_MEMORY_PER_SLOT;

// Total GPU memory: ~200KB for object rendering (excluding mesh data)
```

### Performance Optimization Strategies

#### CPU-Side Optimizations
1. **Pool Allocation**: O(1) allocation/deallocation using free lists
2. **Handle Validation**: Generation counters prevent use-after-free without runtime overhead
3. **Batch Updates**: Single memory copy for all active instance data per frame

#### GPU-Side Optimizations  
1. **Instanced Rendering**: Single draw call for multiple dynamic objects
2. **Descriptor Set Reuse**: Minimize descriptor set allocations and bindings
3. **Memory Locality**: Contiguous instance data for efficient GPU cache usage

## Development Standards

### Performance Requirements
- **60+ FPS** minimum with mixed static/dynamic scenes
- **Clean Vulkan validation** in development builds  
- **Memory efficiency** - bounded allocation, no leaks, proper resource cleanup
- **Predictable performance** - no frame time spikes from dynamic allocation

### Code Quality Standards
- **Memory Safety**: Leverage Rust's ownership system for automatic resource management
- **Error Handling**: Comprehensive Result types with detailed error information
- **Documentation**: Public APIs fully documented with usage examples
- **Testing**: Unit tests for core systems, integration tests for rendering pipelines

### Architecture Enforcement
- **API Boundaries**: Clear separation between static and dynamic object interfaces
- **Resource Management**: RAII patterns with automatic cleanup
- **Performance Validation**: Benchmarks to detect performance regressions

## Future Architecture Evolution

### Short Term (Immediate Implementation)
1. **Dynamic Object Manager**: Core pool-based allocation system
2. **Instance Renderer**: Batch rendering pipeline for dynamic objects  
3. **Material Instance System**: Runtime-modifiable material properties
4. **Static Object Optimization**: Command recording for multiple static objects

### Medium Term (Next Quarter)
1. **Advanced ECS Integration**: Dynamic render components and systems
2. **Cross-Platform Validation**: Windows/Linux compatibility testing
3. **Performance Profiling Tools**: Runtime metrics and debugging visualization
4. **Asset Pipeline Integration**: Seamless static/dynamic asset loading

### Long Term (Future Versions)
1. **Advanced Instancing**: GPU-driven rendering with indirect draws
2. **LOD System Integration**: Level-of-detail for both static and dynamic objects
3. **Multi-threaded Rendering**: Parallel command buffer recording
4. **Advanced Memory Management**: GPU memory pools and virtual allocation

## Conclusion

The Rusteroids engine architecture provides a robust foundation for high-performance game rendering through its dual-path approach. The static GameObject system handles permanent scene elements efficiently, while the dynamic object system enables memory-safe temporary object rendering with predictable performance characteristics.

This architecture follows established game engine patterns from "Game Engine Architecture" while leveraging Rust's safety guarantees to prevent common rendering issues like resource leaks and use-after-free errors. The clear separation of concerns ensures that both static and dynamic rendering pathways can be optimized independently while sharing common infrastructure.

The implementation roadmap provides a clear path from the current single-object rendering to a production-ready system capable of handling complex mixed static/dynamic scenes at 60+ FPS with bounded memory usage.
