# Rusteroids Vulkan Rendering Engine Design

## Overview

This document defines the complete architectural design for the Rusteroids Vulkan rendering engine, covering both high-level architectural principles and detailed implementation specifications. It serves as our contract and rulebook for maintaining consistency, quality, and performance throughout development.

The design emphasizes memory safety, performance, and maintainability through strict resource ownership patterns and modular architecture.

## Core Design Principles

### 1. Ownership and Resource Management Rules

- **Single Owner Rule**: Every Vulkan resource (buffers, images, pipelines) has exactly one owner
- **RAII Pattern**: All resources must be wrapped in structs that implement Drop for automatic cleanup
- **No Raw Handles**: Never expose raw Vulkan handles (vk::Buffer, vk::Image) outside their owning struct
- **Explicit Lifetimes**: Use lifetime parameters where resource dependencies exist
- **Move Semantics**: Resources are moved, not cloned or shared (except for Arc/Rc when necessary)

### 2. Data Flow and Interface Rules

- **Immutable Interfaces**: Public APIs take `&self` or `&mut self`, never consume `self` unless transferring ownership
- **Typed Parameters**: Use strong typing instead of generic parameters where possible
- **Error Propagation**: All Vulkan operations return `Result<T, VulkanError>` - never panic in library code
- **Const Correctness**: Mark all non-mutating methods as `&self`
- **Builder Pattern**: Use builders for complex object creation with validation

### 3. Module Separation Rules

- **Single Responsibility**: Each module handles exactly one concern
- **Dependency Injection**: Higher-level modules depend on abstractions, not concrete implementations
- **No Circular Dependencies**: Enforce strict dependency hierarchy
- **Interface Segregation**: Small, focused traits rather than large interfaces
- **Data/Logic Separation**: Separate data structures from behavior

## Module Architecture

```rust
// Proposed module structure with clear separation of concerns

pub mod vulkan {
    pub mod context;      // Low-level Vulkan context management
    pub mod device;       // Device selection and logical device creation
    pub mod swapchain;    // Swapchain management and recreation
    pub mod memory;       // Memory allocation and management
    pub mod sync;         // Synchronization primitives (fences, semaphores)
}

pub mod resources {
    pub mod buffer;       // Buffer creation and management
    pub mod image;        // Image/texture creation and management
    pub mod mesh;         // Mesh data structures and loading
    pub mod material;     // Material definitions and properties
    pub mod shader;       // Shader compilation and management
}

pub mod pipeline {
    pub mod graphics;     // Graphics pipeline creation
    pub mod compute;      // Future: compute pipeline support
    pub mod layout;       // Pipeline layout management
    pub mod cache;        // Pipeline caching for performance
}

pub mod rendering {
    pub mod renderer;     // High-level rendering interface
    pub mod commands;     // Command buffer recording
    pub mod passes;       // Render pass management
    pub mod queue;        // Render queue and batching
}

pub mod scene {
    pub mod camera;       // Camera systems and matrices
    pub mod transform;    // Transform hierarchies and matrices
    pub mod lighting;     // Light definitions and management
    pub mod objects;      // Renderable object management
}

pub mod assets {
    pub mod loader;       // Asset loading system
    pub mod cache;        // Asset caching and sharing
    pub mod formats;      // Support for different file formats
}
```

## Critical Design Rules

### Buffer Management Rules

```rust
// Rule: All buffers must be owned by a specific manager
pub struct BufferManager {
    device: Arc<LogicalDevice>,
    allocator: Arc<Mutex<GpuAllocator>>,
    buffers: HashMap<BufferId, OwnedBuffer>,
}

// Rule: Buffers are accessed via handles, never direct references
#[derive(Copy, Clone, Hash, Eq, PartialEq)]
pub struct BufferId(u64);

// Rule: Buffer creation returns a handle and result
impl BufferManager {
    pub fn create_vertex_buffer(&mut self, data: &[Vertex]) -> Result<BufferId, BufferError> { }
    pub fn create_index_buffer(&mut self, data: &[u32]) -> Result<BufferId, BufferError> { }
    pub fn get_buffer(&self, id: BufferId) -> Option<&OwnedBuffer> { }
    pub fn destroy_buffer(&mut self, id: BufferId) -> Result<(), BufferError> { }
}
```

### Command Recording Rules

```rust
// Rule: Command recording is type-safe and structured
pub struct CommandRecorder<'a> {
    buffer: vk::CommandBuffer,
    device: &'a LogicalDevice,
    current_pass: Option<RenderPassToken>,
}

// Rule: Render passes use RAII tokens to ensure proper begin/end
pub struct ActiveRenderPass<'a> {
    recorder: &'a mut CommandRecorder<'a>,
    _token: RenderPassToken,
}

impl<'a> CommandRecorder<'a> {
    pub fn begin_render_pass(&mut self, pass: &RenderPass) -> ActiveRenderPass<'_> { }
}

impl<'a> ActiveRenderPass<'a> {
    pub fn bind_pipeline(&mut self, pipeline: &GraphicsPipeline) { }
    pub fn bind_vertex_buffer(&mut self, buffer_id: BufferId) { }
    pub fn draw_indexed(&mut self, index_count: u32) { }
}
// Auto-drop ends render pass
```

### Resource Lifecycle Rules

```rust
// Rule: All resources implement a common lifecycle pattern
pub trait VulkanResource {
    type Handle;
    type CreateInfo;
    type Error;
    
    fn create(device: &LogicalDevice, info: Self::CreateInfo) -> Result<Self, Self::Error> 
    where Self: Sized;
    
    fn handle(&self) -> Self::Handle;
    fn is_valid(&self) -> bool;
}

// Rule: Resource cleanup is deterministic
pub trait ResourceCleanup {
    fn cleanup(&mut self, device: &LogicalDevice);
}
```

### Error Handling Rules

```rust
// Rule: Comprehensive error types for all operations
#[derive(Debug, thiserror::Error)]
pub enum VulkanError {
    #[error("Vulkan API error: {0:?}")]
    Api(vk::Result),
    
    #[error("Resource not found: {id}")]
    ResourceNotFound { id: u64 },
    
    #[error("Invalid operation: {reason}")]
    InvalidOperation { reason: String },
    
    #[error("Out of memory: {requested} bytes")]
    OutOfMemory { requested: usize },
}

// Rule: All public APIs return Results
pub type VulkanResult<T> = Result<T, VulkanError>;
```

## Data Consistency Rules

### Matrix and Transform Rules

```rust
// Rule: Consistent matrix representation and operations
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Mat4 {
    // Column-major storage (GPU-friendly)
    pub cols: [[f32; 4]; 4],
}

// Rule: All transforms go through a central system
pub struct TransformSystem {
    matrices: Vec<Mat4>,
    dirty: BitSet,
    hierarchy: Vec<Option<TransformId>>,
}

// Rule: Camera matrices are managed centrally
pub struct CameraSystem {
    view_matrix: Mat4,
    projection_matrix: Mat4,
    view_projection: Mat4,
    dirty: bool,
}
```

### Shader Interface Rules

```rust
// Rule: Shader interfaces are strongly typed
#[repr(C)]
pub struct StandardVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

// Rule: Push constants have explicit layouts
#[repr(C)]
pub struct StandardPushConstants {
    pub mvp_matrix: Mat4,
    pub model_matrix: Mat4,
    pub material_index: u32,
    pub _padding: [u32; 3],
}

// Rule: Descriptor sets are managed by layout
pub struct DescriptorSetManager {
    layouts: HashMap<DescriptorSetLayoutId, vk::DescriptorSetLayout>,
    pools: Vec<vk::DescriptorPool>,
    sets: HashMap<DescriptorSetId, vk::DescriptorSet>,
}
```

## Performance and Safety Rules

### Memory Management Rules

```rust
// Rule: Use a custom allocator for all GPU memory
pub struct GpuAllocator {
    device: Arc<LogicalDevice>,
    blocks: Vec<MemoryBlock>,
    free_regions: BTreeMap<usize, Vec<MemoryRegion>>,
}

// Rule: All allocations track their usage
#[derive(Debug)]
pub struct Allocation {
    pub block_index: usize,
    pub offset: vk::DeviceSize,
    pub size: vk::DeviceSize,
    pub memory_type: u32,
    pub usage: AllocationUsage,
}
```

### Validation Rules

```rust
// Rule: Debug builds enable comprehensive validation
#[cfg(debug_assertions)]
pub fn enable_validation_layers() -> Vec<*const i8> {
    vec![
        b"VK_LAYER_KHRONOS_validation\0".as_ptr() as *const i8,
    ]
}

// Rule: Release builds are optimized but retain error checking
pub fn create_instance(enable_validation: bool) -> VulkanResult<Instance> {
    // Implementation with conditional validation
}
```

## Resource Management Architecture

![Resource Ownership Model](diagrams/resource_ownership.svg)

Our rendering system follows a strict resource ownership model:

### Core Principles

1. **Single Owner Rule**: Each GPU resource has exactly one owner (Manager)
2. **Handle-Based Access**: Resources accessed through type-safe handles, never raw pointers
3. **RAII Patterns**: Automatic resource cleanup through Rust's Drop trait
4. **No Raw Handles**: Vulkan handles never exposed to application code
5. **Compile-Time Safety**: Resource lifetime verified at compile time

### Resource Managers

Resource managers are the single owners of GPU resources:

```rust
pub struct BufferManager {
    device: Arc<Device>,
    allocator: Arc<Mutex<VmaAllocator>>,
    buffers: SlotMap<BufferKey, BufferResource>,
}

pub struct ImageManager {
    device: Arc<Device>,
    allocator: Arc<Mutex<VmaAllocator>>,
    images: SlotMap<ImageKey, ImageResource>,
}
```

### Type-Safe Handles

Handles provide safe access without ownership:

```rust
pub struct BufferHandle<T> {
    key: BufferKey,
    _phantom: PhantomData<T>,
}

pub struct ImageHandle<T> {
    key: ImageKey,
    _phantom: PhantomData<T>,
}
```

### RAII Resource Guards

For temporary access patterns:

```rust
pub struct MappedBuffer<'a, T> {
    buffer: &'a BufferResource,
    mapped_ptr: *mut T,
    _lifetime: PhantomData<&'a ()>,
}

impl<T> Drop for MappedBuffer<'_, T> {
    fn drop(&mut self) {
        // Automatic unmap on drop
        unsafe { self.buffer.unmap() };
    }
}
```

## Memory Management

### Vulkan Memory Allocator (VMA) Integration

We use VMA for efficient memory management:

```rust
pub struct MemoryManager {
    allocator: VmaAllocator,
    device_local_pool: MemoryPool,
    host_visible_pool: MemoryPool,
    staging_pool: MemoryPool,
}
```

### Memory Pools

- **Device Local**: High-performance GPU memory for frequent access
- **Host Visible**: CPU-mappable memory for data uploads
- **Staging**: Temporary buffers for GPU transfers

### Memory Lifecycle

1. **Allocate**: Request memory from appropriate pool
2. **Map**: Make memory accessible to CPU (if host-visible)
3. **Use**: Read/write data as needed
4. **Unmap**: Remove CPU access
5. **Free**: Return memory to pool

## Command Recording

### Command Buffer Management

```rust
pub struct CommandRecorder {
    pool: vk::CommandPool,
    buffers: Vec<vk::CommandBuffer>,
    current_buffer: Option<vk::CommandBuffer>,
}

impl CommandRecorder {
    pub fn begin_recording(&mut self) -> Result<(), RenderError> {
        // Begin command buffer recording
    }
    
    pub fn end_recording(&mut self) -> Result<vk::CommandBuffer, RenderError> {
        // End recording and return buffer for submission
    }
}
```

### Recording Patterns

1. **Primary Commands**: Main rendering commands
2. **Secondary Commands**: Reusable command sequences
3. **Parallel Recording**: Multi-threaded command generation
4. **Command Batching**: Minimize submission overhead

## Pipeline Management

### Graphics Pipelines

```rust
pub struct GraphicsPipeline {
    pipeline: vk::Pipeline,
    layout: vk::PipelineLayout,
    render_pass: vk::RenderPass,
    vertex_input: VertexInputDescription,
}

pub struct PipelineBuilder {
    vertex_shader: Option<ShaderModule>,
    fragment_shader: Option<ShaderModule>,
    vertex_input: VertexInputDescription,
    // ... other state
}
```

### Shader Management

```rust
pub struct ShaderModule {
    module: vk::ShaderModule,
    stage: vk::ShaderStageFlags,
    entry_point: CString,
}

pub struct ShaderManager {
    device: Arc<Device>,
    modules: HashMap<String, Arc<ShaderModule>>,
    hot_reload_watcher: Option<FileWatcher>,
}
```

## Rendering Pipeline

### High-Level Flow

1. **Scene Preparation**: Gather renderable objects
2. **Culling**: Remove non-visible objects
3. **Sorting**: Order draws for efficiency
4. **Command Recording**: Generate GPU commands
5. **Submission**: Send commands to GPU
6. **Synchronization**: Ensure proper ordering

### Render Passes

```rust
pub struct RenderPass {
    handle: vk::RenderPass,
    attachments: Vec<AttachmentDescription>,
    subpasses: Vec<SubpassDescription>,
}

pub enum RenderPassType {
    Forward,
    Deferred,
    Shadow,
    PostProcess,
}
```

### Frame Synchronization

```rust
pub struct FrameSync {
    image_available: vk::Semaphore,
    render_finished: vk::Semaphore,
    in_flight_fence: vk::Fence,
}
```

## Error Handling

### Error Types

```rust
#[derive(Error, Debug)]
pub enum RenderError {
    #[error("Vulkan error: {0}")]
    Vulkan(#[from] vk::Result),
    
    #[error("Memory allocation failed: {0}")]
    Memory(String),
    
    #[error("Resource not found: {0}")]
    ResourceNotFound(String),
    
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
}
```

### Error Recovery

- **Graceful Degradation**: Reduce quality when resources unavailable
- **Fallback Resources**: Use default assets when loading fails
- **Device Recovery**: Handle device lost scenarios
- **Validation**: Comprehensive error checking in debug builds

## Performance Considerations

### Optimization Strategies

1. **Command Batching**: Minimize draw calls
2. **Resource Streaming**: Async loading of large assets
3. **Memory Pools**: Reduce allocation overhead
4. **GPU Profiling**: Measure bottlenecks
5. **CPU Parallelism**: Multi-threaded command recording

### Memory Budget Management

```rust
pub struct MemoryBudget {
    total_device_memory: u64,
    used_device_memory: u64,
    total_host_memory: u64,
    used_host_memory: u64,
}

impl MemoryBudget {
    pub fn can_allocate(&self, size: u64, memory_type: MemoryType) -> bool {
        // Check if allocation would exceed budget
    }
}
```

## Debug and Validation

### Debug Tools

- **Vulkan Validation Layers**: Runtime error checking
- **Debug Markers**: GPU event labeling
- **Memory Leak Detection**: Resource tracking
- **Performance Counters**: GPU metrics

### Development Features

```rust
#[cfg(debug_assertions)]
pub struct DebugRenderer {
    wireframe_pipeline: GraphicsPipeline,
    bounding_box_renderer: BoundingBoxRenderer,
    performance_overlay: PerformanceOverlay,
}
```

## Testing Strategy

### Unit Tests

- Resource manager operations
- Handle lifetime management
- Memory allocation patterns
- Command recording logic

### Integration Tests

- Full rendering pipeline
- Multi-threaded scenarios
- Error recovery paths
- Performance benchmarks

### Validation

- Memory leak detection
- Resource handle validation
- Command buffer verification
- GPU state consistency

## Implementation Status

### ✅ Completed Design
- [ ] Resource ownership model
- [ ] Memory management architecture
- [ ] Command recording patterns
- [ ] Error handling strategy

### 🚧 Implementation Phase
- [ ] Core resource managers
- [ ] Vulkan context setup
- [ ] Basic command recording
- [ ] Memory allocation system

### 📋 Future Work
- [ ] Advanced rendering techniques
- [ ] Performance optimization
- [ ] Debug tool integration
- [ ] Comprehensive testing

## Implementation Phase Rules

### Phase 1: Foundation (Current Focus)
- [ ] Implement core resource management (Buffer, Image, Pipeline)
- [ ] Create proper error handling throughout
- [ ] Establish memory allocation system
- [ ] Build command recording framework

### Phase 2: Rendering Pipeline
- [ ] Implement render pass management
- [ ] Create material and shader systems
- [ ] Build scene graph foundation
- [ ] Add basic lighting support

### Phase 3: Advanced Features
- [ ] Asset loading and caching
- [ ] Multi-threading support
- [ ] Performance profiling integration
- [ ] Advanced rendering techniques

## Code Review Checklist

Before any commit, verify:
- [ ] All Vulkan resources are properly owned and cleaned up
- [ ] No raw Vulkan handles exposed in public APIs
- [ ] All operations return `Result` types
- [ ] Error messages are descriptive and actionable
- [ ] Memory allocations are tracked and bounded
- [ ] Thread safety is explicitly handled
- [ ] Documentation covers ownership and lifetime rules

## Architecture Diagrams

The following diagrams illustrate key architectural concepts:

### Resource Ownership Model
![Resource Ownership](diagrams/resource_ownership.svg)

### Data Flow Architecture
![Data Flow](diagrams/data_flow.svg)

### Module Dependencies
![Module Dependencies](diagrams/module_dependencies.svg)

### Engine Architecture Overview
![Engine Architecture](diagrams/engine_architecture.svg)

---

## Compliance and Enforcement

This document serves as our architectural contract. All code must comply with these rules, and any deviations must be documented and justified. Regular architectural reviews will ensure adherence to these principles.

**Last Updated**: August 3, 2025
**Version**: 2.0
**Status**: Active

This design provides a solid foundation for a memory-safe, high-performance Vulkan rendering system that can scale from simple 2D graphics to complex 3D scenes while maintaining clean architecture and excellent performance.
