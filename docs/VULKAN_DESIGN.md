# Vulkan Rendering Engine Architecture Document

## Overview

This document defines the architectural principles, design rules, and implementation guidelines for the Rusteroids Vulkan rendering engine. This serves as our contract and rulebook for maintaining consistency and quality throughout development.

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

### Synchronization Rules

```rust
// Rule: Dual synchronization strategy for swapchain rendering
// Problem: With N swapchain images and M frames-in-flight, semaphore reuse conflicts occur
// Solution: Separate per-image semaphores from per-frame synchronization

pub struct VulkanRenderer {
    // Per-image synchronization objects (one per swapchain image)
    // Used for GPU-GPU synchronization between render and present
    image_sync_objects: Vec<FrameSync>,
    
    // Per-frame synchronization objects (for CPU-GPU synchronization) 
    // Used to prevent CPU from getting too far ahead of GPU
    frame_sync_objects: Vec<FrameSync>,
    
    current_frame: usize,
    max_frames_in_flight: usize, // Typically 2
}

// Rule: Each FrameSync contains all synchronization primitives needed for one frame
pub struct FrameSync {
    pub image_available: Semaphore,  // Signals when swapchain image is available
    pub render_finished: Semaphore,  // Signals when rendering is complete  
    pub in_flight: Fence,           // CPU waits on this for frame completion
}

impl VulkanRenderer {
    pub fn new(context: VulkanContext) -> VulkanResult<Self> {
        // Rule: Create sync objects based on swapchain image count AND frames in flight
        let image_count = context.swapchain().image_count() as usize;
        let max_frames_in_flight = 2;
        
        // Per-image sync objects for semaphores (prevents semaphore reuse conflicts)
        let mut image_sync_objects = Vec::new();
        for _ in 0..image_count {
            image_sync_objects.push(FrameSync::new(context.raw_device())?);
        }
        
        // Per-frame sync objects for CPU-GPU synchronization
        let mut frame_sync_objects = Vec::new(); 
        for _ in 0..max_frames_in_flight {
            frame_sync_objects.push(FrameSync::new(context.raw_device())?);
        }
        
        Ok(Self {
            image_sync_objects,
            frame_sync_objects, 
            current_frame: 0,
            max_frames_in_flight,
            // ... other fields
        })
    }
    
    // Rule: Mixed synchronization strategy in draw_frame()
    pub fn draw_frame(&mut self) -> VulkanResult<()> {
        // Step 1: Use frame sync for CPU-GPU synchronization (fence)
        let frame_sync = &self.frame_sync_objects[self.current_frame];
        frame_sync.in_flight.wait(u64::MAX)?;
        frame_sync.in_flight.reset()?;
        
        // Step 2: Acquire image with frame's semaphore (prevents acquire rate conflicts)  
        let acquire_semaphore = &frame_sync.image_available;
        let (image_index, _) = unsafe {
            swapchain_loader.acquire_next_image(
                swapchain,
                u64::MAX,
                acquire_semaphore.handle(),
                vk::Fence::null(),
            )?
        };
        
        // Step 3: Use image-specific sync for render finished (prevents reuse conflicts)
        let image_sync = &self.image_sync_objects[image_index as usize];
        
        // Step 4: Submit with mixed synchronization
        let submit_info = vk::SubmitInfo::builder()
            .wait_semaphores(&[acquire_semaphore.handle()])           // Wait on frame's acquire
            .signal_semaphores(&[image_sync.render_finished.handle()]) // Signal image's finished
            .command_buffers(&[command_buffer]);
            
        unsafe {
            device.queue_submit(
                graphics_queue,
                &[submit_info.build()],
                frame_sync.in_flight.handle(), // CPU sync on frame fence
            )?;
        }
        
        // Step 5: Present with image's render finished semaphore
        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(&[image_sync.render_finished.handle()])
            .swapchains(&[swapchain])
            .image_indices(&[image_index]);
            
        // Step 6: Advance frame counter for next iteration
        self.current_frame = (self.current_frame + 1) % self.max_frames_in_flight;
        
        Ok(())
    }
}

// Rule: Why this approach works
// 1. Each swapchain image gets dedicated render_finished semaphore (no reuse conflicts)
// 2. CPU controls acquire rate with per-frame acquire semaphores (prevents overrun)  
// 3. CPU-GPU sync uses per-frame fences (proper frame completion tracking)
// 4. Mixed strategy eliminates "semaphore still in use by swapchain" validation errors

// Common Synchronization Anti-Patterns to Avoid:
// ❌ Using only per-frame sync objects with more swapchain images than frames
// ❌ Using vk::Semaphore::null() to avoid the chicken-and-egg problem  
// ❌ Creating sync objects equal to swapchain image count without frame limiting
// ❌ Reusing the same semaphore for different swapchain images in same frame
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

## Synchronization Troubleshooting Guide

### Common Validation Errors and Solutions

#### Semaphore Reuse Validation Error
**Error Message**:
```
vkQueueSubmit(): pSubmits[0].pSignalSemaphores[0] (VkSemaphore 0x...) is being signaled by VkQueue, 
but it may still be in use by VkSwapchainKHR.
Here are the most recently acquired image indices: [0], 1, 2.
Swapchain image 0 was presented but was not re-acquired, so VkSemaphore may still be in use 
and cannot be safely reused with image index 2.
```

**Root Cause**: 
- Swapchain has 3 images (typical: min_image_count + 1)
- Renderer has only 2 sync objects (MAX_FRAMES_IN_FLIGHT) 
- Results in semaphore reuse while previous presentation operations are still pending

**Solution Applied**:
```rust
// BEFORE (Incorrect): Single sync strategy
struct VulkanRenderer {
    sync_objects: Vec<FrameSync>,  // Only 2 objects
    current_frame: usize,
}

pub fn draw_frame(&mut self) -> VulkanResult<()> {
    let sync = &self.sync_objects[self.current_frame]; // Problem: reuses semaphores
    // ... acquire with sync.image_available
    // ... signal with sync.render_finished  
    self.current_frame = (self.current_frame + 1) % 2; // Cycles too fast
}

// AFTER (Correct): Dual sync strategy  
struct VulkanRenderer {
    image_sync_objects: Vec<FrameSync>,  // One per swapchain image (3)
    frame_sync_objects: Vec<FrameSync>,  // One per frame in flight (2)
    current_frame: usize,
}

pub fn draw_frame(&mut self) -> VulkanResult<()> {
    let frame_sync = &self.frame_sync_objects[self.current_frame];
    // ... acquire with frame_sync.image_available
    let image_sync = &self.image_sync_objects[image_index as usize];  
    // ... signal with image_sync.render_finished (per-image, no conflicts)
    self.current_frame = (self.current_frame + 1) % 2;
}
```

**Verification**: 
- No validation errors after 10+ seconds of rendering
- Each swapchain image has dedicated render_finished semaphore
- CPU framerate limiting still works correctly with per-frame fences

#### Timing of the Error
- **Error appears "after about a second"**: Indicates the conflict happens when the swapchain cycles through all images and starts reusing semaphores
- **Error shows multiple image indices**: Confirms the swapchain is actively cycling through available images
- **Brackets around index [0]**: Vulkan validation indicating the specific semaphore/image causing the conflict

## Architecture Diagrams

The following Draw.io diagrams illustrate key architectural concepts:

### Resource Ownership Model
![Resource Ownership](./docs/diagrams/resource_ownership.drawio)

### Data Flow Architecture
![Data Flow](./docs/diagrams/data_flow.drawio)

### Module Dependencies
![Module Dependencies](./docs/diagrams/module_dependencies.drawio)

### Command Recording Flow
![Command Recording](./docs/diagrams/command_recording.drawio)

### Memory Management System
![Memory Management](./docs/diagrams/memory_management.drawio)

---

## Compliance and Enforcement

This document serves as our architectural contract. All code must comply with these rules, and any deviations must be documented and justified. Regular architectural reviews will ensure adherence to these principles.

**Last Updated**: August 10, 2025
**Version**: 1.1
**Status**: Active - Updated with Synchronization Troubleshooting
