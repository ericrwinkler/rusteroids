# Rusteroids Development Rules

## 🔴 **MANDATORY RULES**

### Build Validation
**Rule**: All commits must compile without errors or warnings
```bash
cargo build                    # Must succeed with zero warnings
cargo test                     # All tests must pass
cargo clippy -- -D warnings   # No clippy warnings allowed
```

### Screenshot Validation (CRITICAL)
**Rule**: Every commit touching rendering code MUST include screenshot validation
```cmd
# Before making changes
.\tools\validate_rendering.bat baseline

# After implementing changes  
.\tools\validate_rendering.bat validation
```
**Expected Results**: 
- ✅ RenderedScene classification
- ✅ >60% colored pixels (proper 3D content)
- ✅ No visual regressions

### Dependency Management
**Rule**: No new Cargo.toml dependencies without explicit approval
- We build our own libraries for asset loading, serialization
- Maintain control over ECS architecture compatibility  
- Justify any external dependencies with clear rationale

### Performance Requirements
**Rule**: Maintain minimum performance standards
- **60+ FPS** with current scene complexity 
- **Clean Vulkan validation** (zero errors in development)
- **Memory efficiency** (no leaks, proper RAII patterns)

## 🟡 **CODING STANDARDS**

### Rust Best Practices

#### Memory Safety & Ownership
```rust
// ✅ GOOD: Single ownership with RAII
pub struct VulkanBuffer {
    buffer: vk::Buffer,
    memory: vk::DeviceMemory,
}

impl Drop for VulkanBuffer {
    fn drop(&mut self) {
        // Automatic cleanup
        unsafe {
            self.device.destroy_buffer(self.buffer, None);
            self.device.free_memory(self.memory, None);
        }
    }
}

// ❌ AVOID: Manual memory management
unsafe {
    device.destroy_buffer(buffer, None);  // Easy to forget
}
```

#### Error Handling
```rust
// ✅ GOOD: Comprehensive error types
pub enum RenderError {
    VulkanError(ash::vk::Result),
    ShaderCompilation(String),
    ResourceCreation(String),
}

// ✅ GOOD: Result propagation
pub fn create_buffer(&self, size: u64) -> Result<Buffer, RenderError> {
    // Implementation
}

// ❌ AVOID: Unwrap in production code
let buffer = create_buffer(size).unwrap();  // Don't do this
```

#### Documentation Standards
```rust
/// Create a new uniform buffer for per-frame data
///
/// This buffer will be updated every frame with camera matrices
/// and lighting information.
///
/// # Arguments
/// * `size` - Buffer size in bytes
/// * `usage` - Vulkan buffer usage flags
///
/// # Returns
/// * `Ok(Buffer)` - Successfully created buffer
/// * `Err(RenderError)` - Creation failed
///
/// # Examples
/// ```rust
/// let camera_ubo = renderer.create_uniform_buffer(
///     std::mem::size_of::<CameraUBO>() as u64,
///     vk::BufferUsageFlags::UNIFORM_BUFFER
/// )?;
/// ```
pub fn create_uniform_buffer(&self, size: u64, usage: vk::BufferUsageFlags) -> Result<Buffer, RenderError> {
    // Implementation
}
```

### Architecture Principles

#### Separation of Concerns
```rust
// ✅ GOOD: Clear separation
pub struct Renderer {         // Only handles rendering
    vulkan_context: VulkanContext,
    pipeline_manager: PipelineManager,
}

pub struct World {            // Only handles ECS
    entities: EntityStorage,
    components: ComponentManager,
}

// ❌ AVOID: Mixed responsibilities
pub struct GameManager {      // Don't mix rendering + ECS + input
    renderer: Renderer,
    world: World,
    input_handler: InputHandler,
}
```

#### Data-Oriented Design
```rust
// ✅ GOOD: Component data layout
#[repr(C, align(16))]
pub struct TransformComponent {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

// ✅ GOOD: System processes components
impl MovementSystem {
    pub fn update(&mut self, world: &mut World, delta_time: f32) {
        for (entity, transform, velocity) in world.query::<(&mut TransformComponent, &VelocityComponent)>() {
            transform.position += velocity.value * delta_time;
        }
    }
}
```

#### Type Safety
```rust
// ✅ GOOD: Strong typing
pub struct EntityId(u32);
pub struct ComponentId(u32);
pub struct MaterialId(u32);

// ❌ AVOID: Primitive obsession
pub fn get_component(entity: u32, component: u32) -> Option<Component> {
    // Easy to mix up parameters
}

// ✅ GOOD: Type-safe API
pub fn get_component<T: Component>(&self, entity: EntityId) -> Option<&T> {
    // Cannot mix up types
}
```

## 🟢 **PERFORMANCE RULES**

### Vulkan Best Practices

#### Resource Management
```rust
// ✅ GOOD: Minimize state changes
for material_id in materials.keys() {
    renderer.bind_material(material_id);
    
    // Draw all objects with this material
    for object in objects_with_material(material_id) {
        renderer.draw_object(object);
    }
}

// ❌ AVOID: Excessive state changes
for object in all_objects {
    renderer.bind_material(object.material);  // Potentially redundant
    renderer.draw_object(object);
}
```

#### Memory Allocation
```rust
// ✅ GOOD: Pre-allocated, reused collections
pub struct RenderQueue {
    commands: Vec<RenderCommand>,  // Reused each frame
}

impl RenderQueue {
    pub fn prepare_frame(&mut self) {
        self.commands.clear();  // Reuse allocation
    }
}

// ❌ AVOID: Per-frame allocations
pub fn collect_render_commands() -> Vec<RenderCommand> {
    Vec::new()  // New allocation every frame
}
```

#### GPU Synchronization
```rust
// ✅ GOOD: Pipeline barriers for specific resources
cmd.pipeline_barrier(
    vk::PipelineStageFlags::VERTEX_SHADER,
    vk::PipelineStageFlags::FRAGMENT_SHADER,
    vk::DependencyFlags::empty(),
    &[],
    &[buffer_barrier],
    &[],
);

// ❌ AVOID: Device wait idle (kills parallelism)
device.device_wait_idle();  // Only for cleanup/shutdown
```

## 🔵 **TESTING RULES**

### Unit Testing
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_matrix_calculation() {
        let transform = Transform {
            position: Vec3::new(1.0, 2.0, 3.0),
            rotation: Quat::identity(),
            scale: Vec3::new(1.0, 1.0, 1.0),
        };
        
        let matrix = transform.to_matrix();
        
        // Verify translation
        assert_eq!(matrix.m14, 1.0);
        assert_eq!(matrix.m24, 2.0);
        assert_eq!(matrix.m34, 3.0);
    }
}
```

### Integration Testing
```rust
// Test application entry points
#[test]
fn test_teapot_app_initialization() {
    let app = TeapotApp::new().expect("App should initialize");
    assert!(app.renderer.is_some());
    assert!(app.camera.is_some());
}
```

### Performance Testing
```rust
#[test]
fn test_render_performance() {
    let mut renderer = create_test_renderer();
    let start = Instant::now();
    
    for _ in 0..60 {  // Simulate 60 frames
        renderer.render_frame().expect("Frame should render");
    }
    
    let duration = start.elapsed();
    assert!(duration.as_secs_f32() < 1.0, "Should maintain 60+ FPS");
}
```

## 🟠 **DEBUGGING RULES**

### Logging Standards
```rust
// ✅ GOOD: Structured logging
log::info!("Renderer initialized: pipeline_count={}, material_count={}", 
           pipeline_count, material_count);

log::debug!("Frame {}: drew {} objects in {:.2}ms", 
            frame_number, object_count, render_time_ms);

log::warn!("Material {} not found, using default", material_id);

log::error!("Vulkan error in buffer creation: {:?}", vk_result);
```

### Validation Layers
```rust
// Enable in debug builds
#[cfg(debug_assertions)]
const ENABLE_VALIDATION: bool = true;

#[cfg(not(debug_assertions))]
const ENABLE_VALIDATION: bool = false;
```

### Error Context
```rust
// ✅ GOOD: Contextual errors
self.create_buffer(size)
    .map_err(|e| RenderError::BufferCreation {
        size,
        usage: format!("{:?}", usage),
        source: Box::new(e),
    })?;

// ❌ AVOID: Generic errors
self.create_buffer(size).map_err(|_| "Buffer creation failed")?;
```

## 🟤 **COMMIT STANDARDS**

### Commit Message Format
```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

**Types**: feat, fix, docs, style, refactor, test, chore
**Scopes**: renderer, ecs, math, assets, tools

**Examples**:
```
feat(renderer): implement multiple objects rendering

Add command recording pattern for efficient multiple object rendering.
Replaces immediate mode with proper Vulkan command buffer recording.

Fixes #123
Performance: 10+ objects at 60+ FPS
```

### Pre-Commit Checklist
- [ ] `cargo build` passes
- [ ] `cargo test` passes  
- [ ] `cargo clippy` has no warnings
- [ ] Screenshot validation passes (for rendering changes)
- [ ] Documentation updated
- [ ] Commit message follows format

## 🔶 **REVIEW STANDARDS**

### Code Review Requirements
1. **Architecture Review**: Does change fit overall design?
2. **Performance Review**: Maintains performance requirements?
3. **Safety Review**: Proper error handling and memory safety?
4. **Testing Review**: Adequate test coverage?
5. **Documentation Review**: Public APIs documented?

### Review Checklist
```markdown
- [ ] Architecture: Follows separation of concerns
- [ ] Performance: No performance regressions
- [ ] Safety: Proper error handling, no unsafe code without justification
- [ ] Testing: New functionality has tests
- [ ] Documentation: Public APIs documented
- [ ] Style: Follows Rust idioms and project conventions
```
