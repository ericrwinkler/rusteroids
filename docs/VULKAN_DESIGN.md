# Vulkan Rendering Engine Architecture Document

## Recent Major Updates (August 2025)

### ✅ Aspect Ratio Fix Implementation
**Status**: COMPLETED
**Problem**: Teapot appeared visually squished during window resizing despite correct aspect ratio calculations
**Root Cause**: Graphics pipeline created with static viewport/scissor, never updated during swapchain recreation
**Solution**: 
- Added dynamic viewport/scissor setting during rendering using current swapchain extent
- Configured graphics pipeline with `VK_DYNAMIC_STATE_VIEWPORT` and `VK_DYNAMIC_STATE_SCISSOR` support
- Removed extent parameter from pipeline creation since viewport is now set dynamically

**Technical Changes**:
```rust
// In renderer.rs draw_frame():
let viewport = vk::Viewport::builder()
    .x(0.0)
    .y(0.0)
    .width(extent.width as f32)
    .height(extent.height as f32)
    .min_depth(0.0)
    .max_depth(1.0);

let scissor = vk::Rect2D::builder()
    .offset(vk::Offset2D { x: 0, y: 0 })
    .extent(extent);

device.cmd_set_viewport(command_buffer, 0, &[viewport.build()]);
device.cmd_set_scissor(command_buffer, 0, &[scissor.build()]);

// In shader.rs pipeline creation:
let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
let dynamic_state = vk::PipelineDynamicStateCreateInfo::builder()
    .dynamic_states(&dynamic_states);
```

### ✅ Johannes Unterguggenberger Guide Compliance
**Status**: COMPLETED
**Problem**: Our projection matrix implementation didn't follow academic/industry standard Vulkan practices
**Reference**: [Setting Up a Proper Vulkan Projection Matrix](https://johannesugb.github.io/gpu-programming/setting-up-a-proper-vulkan-projection-matrix/)
**Changes Made**:

1. **Corrected Projection Matrix Formula**:
```rust
// OLD (incorrect):
result[(1, 1)] = -1.0 / tan_half_fovy;  // Manual Y-flip
result[(2, 2)] = far / (near - far);    // Wrong depth mapping
result[(3, 2)] = -1.0;                  // Wrong sign

// NEW (guide-compliant):
result[(1, 1)] = 1.0 / tan_half_fovy;        // Positive Y
result[(2, 2)] = far / (far - near);         // Correct depth to [0,1]
result[(2, 3)] = -(near * far) / (far - near); // Correct depth offset
result[(3, 2)] = 1.0;                        // Perspective divide trigger
```

2. **Added Intermediate Coordinate Transform**:
```rust
fn vulkan_coordinate_transform() -> Mat4 {
    // X matrix from the guide - flips Y and Z for Vulkan conventions
    Mat4::new(
        1.0,  0.0,  0.0, 0.0,
        0.0, -1.0,  0.0, 0.0,  // Y-flip: up -> down
        0.0,  0.0, -1.0, 0.0,  // Z-flip: forward -> into screen
        0.0,  0.0,  0.0, 1.0,
    )
}
```

3. **Updated Matrix Chain to C = P × X × V × M**:
```rust
// Follow guide's complete transformation chain
let mvp = projection_matrix * coord_transform * view_matrix * *model_matrix;
```

4. **Corrected Camera Coordinate System**:
```rust
// Camera now uses standard Y-up in view space
up: Vec3::new(0.0, 1.0, 0.0),  // Standard Y-up convention
position: Vec3::new(0.0, 3.0, 3.0),  // Positive Y = above
```

**Benefits**:
- Mathematical correctness matching academic standards
- Clear separation between view space (Y-up) and Vulkan specifics (coordinate transform)
- Easier to maintain and adapt for other graphics APIs
- Industry-standard approach used in real graphics engines

### ✅ Previous Lighting System Fix
**Status**: COMPLETED (documented)
**Problem**: Lighting appeared to rotate with objects instead of staying fixed in world space
**Solution**: Proper normal matrix calculation using inverse-transpose with correct matrix transposition for GLSL compatibility

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

### Face Winding and Asset Compatibility Rules

```rust
// Rule: Different 3D assets may have different face winding conventions
// Problem: OBJ files from different modeling software use inconsistent winding orders
// Solution: Configure pipeline based on asset requirements, document expectations

pub enum WindingOrder {
    CounterClockwise,  // OpenGL/Blender convention (most common)
    Clockwise,         // Some modeling software, CAD programs
}

// Rule: Pipeline configuration should match asset expectations
impl GraphicsPipeline {
    pub fn new_for_asset_type(winding: WindingOrder) -> PipelineBuilder {
        let front_face = match winding {
            WindingOrder::CounterClockwise => vk::FrontFace::COUNTER_CLOCKWISE,
            WindingOrder::Clockwise => vk::FrontFace::CLOCKWISE,
        };
        
        PipelineBuilder::new()
            .rasterization(vk::PipelineRasterizationStateCreateInfo::builder()
                .cull_mode(vk::CullModeFlags::BACK)  // Always cull back faces for performance
                .front_face(front_face)              // Match asset winding order
            )
    }
}

// Diagnostic Rule: When faces disappear unexpectedly, check winding order
// Symptoms:
// - Missing/invisible triangles when viewing model from certain angles
// - Interior faces visible through model surface  
// - Model appears "inside-out" or with holes
// 
// Debug Process:
// 1. Temporarily set cull_mode(vk::CullModeFlags::NONE) to see all faces
// 2. If model renders correctly with no culling, winding order mismatch confirmed
// 3. Toggle front_face between CLOCKWISE and COUNTER_CLOCKWISE
// 4. Re-enable back face culling once correct winding determined
//
// Asset-Specific Notes:
// - Utah Teapot OBJ: Uses CLOCKWISE winding (confirmed fix)
// - Blender exports: Typically COUNTER_CLOCKWISE  
// - 3ds Max exports: Often CLOCKWISE
// - CAD software: Varies by vendor
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

## Coordinate System Standardization

### Engine-Wide Coordinate System Policy

**RULE: All coordinate-related code must use consistent Vulkan-native coordinates throughout the entire engine pipeline.**

Our engine standardizes on **Vulkan's native coordinate system** to eliminate conversion confusion and ensure predictable rendering behavior. This system is used consistently across all modules: math utilities, asset loading, rendering, camera positioning, lighting, and applications.

#### Vulkan Coordinate System (Engine Standard)
- **+X axis**: Points to the right
- **+Y axis**: Points downward (screen-space convention)
- **+Z axis**: Points into the screen (away from viewer) 
- **NDC Z range**: [0.0, 1.0] (Vulkan's native depth range)
- **Face winding**: Depends on asset type (configurable in pipeline)

#### Coordinate System Rules by Module

##### 1. Foundation Math Module (`crates/rust_engine/src/foundation/math.rs`)
- **RULE**: All matrix operations use Vulkan-compliant conventions following Johannes Unterguggenberger's guide
- **Projection matrices**: Mathematically correct Vulkan perspective matrices
- **View matrices**: Standard right-handed look-at with coordinate transform handling Vulkan conventions
- **Transforms**: Proper coordinate space separation between view space and Vulkan requirements

```rust
// UPDATED: Vulkan-compliant perspective matrix (follows academic guide)
fn perspective(fov_y: f32, aspect: f32, near: f32, far: f32) -> Mat4 {
    // Implements P matrix from Johannes Unterguggenberger's guide exactly
    let tan_half_fovy = (fov_y * 0.5).tan();
    let mut result = Mat4::zeros();
    
    result[(0, 0)] = 1.0 / (aspect * tan_half_fovy);  // a⁻¹/tan(φ/2)
    result[(1, 1)] = 1.0 / tan_half_fovy;             // 1/tan(φ/2) - no Y-flip here
    result[(2, 2)] = far / (far - near);              // f/(f-n) - maps Z to [0,1]
    result[(2, 3)] = -(near * far) / (far - near);    // -nf/(f-n) - depth offset
    result[(3, 2)] = 1.0;                             // Perspective divide trigger
    
    result
}

// NEW: Intermediate coordinate system transformation (X matrix from guide)
fn vulkan_coordinate_transform() -> Mat4 {
    // Handles conversion from standard Y-up view space to Vulkan Y-down conventions
    Mat4::new(
        1.0,  0.0,  0.0, 0.0,  // X unchanged
        0.0, -1.0,  0.0, 0.0,  // Y flipped: up -> down
        0.0,  0.0, -1.0, 0.0,  // Z flipped: forward -> into screen
        0.0,  0.0,  0.0, 1.0,
    )
}

// UPDATED: Standard look-at matrix (Y-up view space)
fn look_at(eye: Vec3, target: Vec3, up: Vec3) -> Mat4 {
    // Standard right-handed look-at, coordinate transform handles Vulkan specifics
    let forward = (target - eye).normalize();
    let right = forward.cross(&up).normalize();
    let camera_up = right.cross(&forward);
    // ... standard implementation
}
```

##### 2. Render Module (`crates/rust_engine/src/render/mod.rs`)
- **RULE**: Camera uses standard Y-up conventions, relies on coordinate transform for Vulkan compatibility
- **Matrix Chain**: Implements complete C = P × X × V × M transformation from guide
- **Separation of Concerns**: View space logic separate from Vulkan-specific transformations

```rust
// UPDATED: Camera with standard Y-up convention
impl Camera {
    pub fn perspective(position: Vec3, fov_degrees: f32, aspect: f32, near: f32, far: f32) -> Self {
        Self {
            position,
            target: Vec3::zeros(),
            up: Vec3::new(0.0, 1.0, 0.0),  // Standard Y-up in view space
            // coordinate transform handles Vulkan conventions
        }
    }
    
    // NEW: Proper matrix chain following guide
    pub fn get_view_projection_matrix(&self) -> Mat4 {
        let view_matrix = self.get_view_matrix();
        let coord_transform = Mat4::vulkan_coordinate_transform();
        let projection_matrix = self.get_projection_matrix();
        
        // C = P * X * V (complete transformation from guide)
        projection_matrix * coord_transform * view_matrix
    }
}

// UPDATED: MVP calculation with coordinate transform
let mvp = projection_matrix * coord_transform * view_matrix * *model_matrix;
```

##### 3. Vulkan Renderer (`crates/rust_engine/src/render/vulkan/renderer.rs`)
- **RULE**: Dynamic viewport/scissor for proper aspect ratio handling
- **Pipeline Configuration**: Supports dynamic state for viewport updates during swapchain recreation
- **Resource Management**: Handles window resizing without pipeline recreation

```rust
// NEW: Dynamic viewport/scissor setting
pub fn draw_frame(&mut self) -> VulkanResult<()> {
    let extent = self.context.swapchain().extent();
    
    // Set viewport dynamically based on current swapchain extent
    let viewport = vk::Viewport::builder()
        .x(0.0).y(0.0)
        .width(extent.width as f32)
        .height(extent.height as f32)
        .min_depth(0.0).max_depth(1.0);
    
    let scissor = vk::Rect2D::builder()
        .offset(vk::Offset2D { x: 0, y: 0 })
        .extent(extent);
    
    unsafe {
        device.cmd_set_viewport(command_buffer, 0, &[viewport.build()]);
        device.cmd_set_scissor(command_buffer, 0, &[scissor.build()]);
    }
}
```

##### 4. Graphics Pipeline (`crates/rust_engine/src/render/vulkan/shader.rs`)
- **RULE**: Pipeline configured for dynamic viewport/scissor state
- **Winding Order**: Configurable based on asset requirements (clockwise for Utah teapot)
- **Face Culling**: Back-face culling enabled with proper winding order detection

```rust
// NEW: Dynamic state configuration
let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
let dynamic_state = vk::PipelineDynamicStateCreateInfo::builder()
    .dynamic_states(&dynamic_states);

// Pipeline creation with dynamic state support
let pipeline_info = vk::GraphicsPipelineCreateInfo::builder()
    .stages(&shader_stages)
    .vertex_input_state(&vertex_input_info)
    .input_assembly_state(&input_assembly)
    .viewport_state(&viewport_state)  // Count only, actual viewport set dynamically
    .rasterization_state(&rasterizer)
    .multisample_state(&multisampling)
    .depth_stencil_state(&depth_stencil)
    .color_blend_state(&color_blending)
    .dynamic_state(&dynamic_state)  // Enable dynamic viewport/scissor
    .layout(layout)
    .render_pass(render_pass)
    .subpass(0);
```

##### 2. Asset Loading Module (`crates/rust_engine/src/assets/obj_loader.rs`)
- **RULE**: Convert all loaded assets to Vulkan coordinate system on import
- **Vertex positions**: Apply coordinate conversion during OBJ parsing
- **Normals**: Convert normal vectors to match Vulkan handedness
- **Face winding**: Detect and convert to expected pipeline winding

```rust
// STANDARDIZED: Asset coordinate conversion
impl ObjLoader {
    pub fn load_obj<P: AsRef<Path>>(path: P) -> Result<Mesh, ObjError> {
        // ... parse OBJ file ...
        
        // CONVERSION RULE: Transform all positions to Vulkan coordinates
        for vertex in &mut vertices {
            vertex.position = Self::convert_to_vulkan_coords(vertex.position);
            vertex.normal = Self::convert_normal_to_vulkan(vertex.normal);
        }
        
        // WINDING RULE: Convert indices for consistent face winding
        if needs_winding_conversion {
            Self::convert_face_winding(&mut indices);
        }
        
        Ok(Mesh::new(vertices, indices))
    }
    
    /// Convert position from asset coordinate system to Vulkan
    fn convert_to_vulkan_coords(pos: [f32; 3]) -> [f32; 3] {
        // Example: OBJ Y-up to Vulkan Y-down conversion
        [pos[0], -pos[1], pos[2]]  // Flip Y coordinate
    }
    
    /// Convert normals to Vulkan coordinate system  
    fn convert_normal_to_vulkan(normal: [f32; 3]) -> [f32; 3] {
        // Convert normal vectors to match coordinate system change
        [normal[0], -normal[1], normal[2]]  // Flip Y component
    }
    
    /// Convert face winding if necessary
    fn convert_face_winding(indices: &mut Vec<u32>) {
        // Reverse triangle winding order if needed
        for triangle in indices.chunks_mut(3) {
            triangle.swap(1, 2);  // Convert CCW to CW or vice versa
        }
    }
}
```

##### 3. Camera System (`crates/rust_engine/src/render/mod.rs`)
- **RULE**: Camera positioning uses Vulkan coordinate conventions
- **Position vectors**: Vulkan Y-down, Z-into-screen coordinate space
- **Look-at calculations**: Consistent with Vulkan handedness

```rust
// STANDARDIZED: Camera in Vulkan coordinates
impl Camera {
    /// Create camera with Vulkan coordinate assumptions
    pub fn perspective(position: Vec3, fov_degrees: f32, aspect: f32, near: f32, far: f32) -> Self {
        Self {
            position,  // Direct Vulkan coordinates
            target: Vec3::zeros(),
            up: Vec3::new(0.0, -1.0, 0.0),  // Y-down for Vulkan
            fov: utils::deg_to_rad(fov_degrees),
            aspect,
            near,
            far,
        }
    }
    
    /// Position camera above object looking down (Vulkan coordinates)
    pub fn look_down_at_origin(height: f32, distance: f32) -> Self {
        Self::perspective(
            Vec3::new(0.0, -height, distance),  // Negative Y = above in Vulkan
            45.0,
            16.0/9.0,
            0.1,
            100.0
        )
    }
}
```

##### 4. Vulkan Rendering Pipeline (`crates/rust_engine/src/render/vulkan/`)
- **RULE**: Pipeline expects Vulkan-native coordinates (no additional conversion)
- **Viewport**: Standard Vulkan viewport (no Y-flip needed)
- **Face winding**: Configured based on standardized asset loading
- **Depth range**: Native Vulkan [0,1] range

```rust
// STANDARDIZED: Vulkan pipeline configuration
impl GraphicsPipeline {
    pub fn new(/* ... */) -> VulkanResult<Self> {
        // Standard Vulkan viewport (no flipping needed)
        let viewport = vk::Viewport::builder()
            .x(0.0)
            .y(0.0)
            .width(extent.width as f32)
            .height(extent.height as f32)
            .min_depth(0.0)  // Vulkan native depth range
            .max_depth(1.0)
            .build();
        
        // Face winding configured for standardized assets
        let rasterizer = vk::PipelineRasterizationStateCreateInfo::builder()
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)  // Standardized after asset conversion
            .cull_mode(vk::CullModeFlags::BACK);
    }
}
```

##### 5. Application Layer (e.g., `teapot_app/src/main.rs`)
- **RULE**: All camera positioning and scene setup uses Vulkan coordinates
- **Camera placement**: Vulkan Y-down, Z-into-screen conventions  
- **Lighting direction**: Consistent with Vulkan coordinate system

```rust
// STANDARDIZED: Application using Vulkan coordinates throughout
impl IntegratedApp {
    pub fn new() -> Self {
        // Camera positioned using Vulkan coordinates
        let mut camera = Camera::perspective(
            Vec3::new(4.0, -6.0, 8.0),  // Vulkan: -Y=up, +Z=away from object
            45.0,
            800.0 / 600.0,
            0.1,
            100.0
        );
        
        // Look at origin with Vulkan coordinate assumptions
        camera.look_at(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, -1.0, 0.0));
        
        // Lighting using Vulkan coordinate system
        let lighting_env = LightingEnvironment::new()
            .add_light(Light::directional(
                Vec3::new(-0.5, -1.0, -0.3),  // Vulkan: -Y = from above
                Vec3::new(1.0, 0.95, 0.9),
                2.0,
            ));
    }
}
```

#### Migration Strategy

When standardizing existing coordinate systems:

1. **Audit all coordinate usage** in each module
2. **Convert at asset load time** (not during rendering)
3. **Update camera positioning** to use Vulkan conventions
4. **Verify pipeline configuration** matches standardized assets
5. **Test rendering output** for correct orientation

#### Debugging Coordinate Issues

Common signs of coordinate system inconsistencies:
- **Upside-down models**: Y-axis coordinate system mismatch
- **Inside-out models**: Face winding or normal direction issues
- **Camera positioning confusion**: Mixed coordinate system assumptions
- **Lighting artifacts**: Light direction inconsistent with coordinate system

**Debug Process:**
1. Verify asset loading applies coordinate conversion
2. Check camera positioning uses consistent coordinate system
3. Confirm pipeline face winding matches converted assets
4. Validate matrix calculations use same coordinate conventions throughout

#### Coordinate System Testing

```rust
#[cfg(test)]
mod coordinate_tests {
    use super::*;
    
    #[test]
    fn test_vulkan_coordinate_consistency() {
        // Verify camera positioned "above" object has negative Y in Vulkan coords
        let camera = Camera::perspective(Vec3::new(0.0, -5.0, 3.0), 45.0, 1.0, 0.1, 100.0);
        assert!(camera.position.y < 0.0, "Camera 'above' should have negative Y in Vulkan");
        
        // Verify projection matrix produces correct NDC
        let proj = Mat4::perspective(45.0_f32.to_radians(), 1.0, 0.1, 100.0);
        // Test point transformations...
    }
    
    #[test] 
    fn test_asset_coordinate_conversion() {
        // Test that OBJ loading converts coordinates properly
        let mesh = ObjLoader::load_obj("test_assets/cube.obj").unwrap();
        // Verify vertices are in expected coordinate system...
    }
}

#### 5. Matrix Transformations

Our rendering pipeline performs these coordinate transformations:
1. **World → View**: `nalgebra::Matrix4::look_at_rh()` (right-handed view matrix)
2. **View → Clip**: `nalgebra::Matrix4::new_perspective()` (handles Vulkan NDC conventions)
3. **Clip → Screen**: Vulkan viewport transformation

#### 6. Debugging Coordinate Issues

When objects appear upside-down or from wrong angles:

1. **Check Z-axis direction**: Ensure camera Z position makes sense for right-handed coords
2. **Verify viewing direction**: Use negative Z for "forward" movement in right-handed system
3. **Confirm up vector**: Usually `Vec3::new(0.0, 1.0, 0.0)` for Y-up orientation
4. **Test with known positions**: Start with `(0, 0, -10)` looking at `(0, 0, 0)` for simple forward view

#### 7. Integration Notes

- **nalgebra**: Handles right-handed world/view coordinates
- **Vulkan**: Receives properly converted matrices from nalgebra
- **Our Engine**: Uses right-handed conventions throughout, relies on nalgebra for GPU compatibility

---

## Compliance and Enforcement

This document serves as our architectural contract. All code must comply with these rules, and any deviations must be documented and justified. Regular architectural reviews will ensure adherence to these principles.

**Last Updated**: August 10, 2025
**Version**: 1.2
**Status**: Active - Updated with Coordinate System Documentation
