# Rusteroids Development Rules & Best Practices

## 🔴 MANDATORY RULES

### Screenshot Validation (CRITICAL)
**Rule**: Every commit touching rendering code MUST include screenshot validation

```cmd
:: Before making changes
.\tools\validate_rendering.bat baseline

:: After implementing changes  
.\tools\validate_rendering.bat validation

:: For specific change types
.\tools\validate_rendering.bat material   # Material system changes
.\tools\validate_rendering.bat pipeline   # Pipeline management
.\tools\validate_rendering.bat shader     # Shader modifications
.\tools\validate_rendering.bat ubo        # UBO structure changes
```

**Expected Results**: 
- ✅ RenderedScene classification
- ✅ >60% colored pixels (proper 3D content)
- ✅ No regression in visual quality

### Build Validation
**Rule**: All commits must compile without errors or warnings

```cmd
cargo build                    # Must succeed
cargo test                     # All tests must pass
```

**Zero Tolerance**: No warnings allowed in production code

### Dependency Management
**Rule**: No new Cargo.toml dependencies without explicit approval

- We build our own libraries for asset loading, serialization
- Maintain control over ECS architecture compatibility
- Justify any external dependencies with clear rationale

## 🟡 CODING STANDARDS

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
        // Cleanup resources
    }
}

// ❌ BAD: Raw handles without ownership
pub fn get_raw_buffer() -> vk::Buffer { /* */ }
```

#### Error Handling
```rust
// ✅ GOOD: Explicit error propagation
pub fn create_buffer() -> Result<VulkanBuffer, VulkanError> {
    // Implementation
}

// ❌ BAD: Unwrap or panic in library code
pub fn create_buffer() -> VulkanBuffer {
    unsafe_operation().unwrap() // Never do this
}
```

#### Interface Design
```rust
// ✅ GOOD: Immutable interfaces with builder pattern
impl MaterialConfig {
    pub fn new() -> Self { /* */ }
    pub fn with_color(mut self, color: Vec3) -> Self { /* */ }
}

// ❌ BAD: Mutable public fields
pub struct MaterialConfig {
    pub color: Vec3,  // Direct mutation breaks invariants
}
```

### Vulkan Patterns

#### Resource Management
```rust
// ✅ GOOD: Proper synchronization with barriers
fn update_buffer(&mut self) -> VulkanResult<()> {
    self.cmd_pipeline_barrier(
        vk::PipelineStageFlags::VERTEX_INPUT,
        vk::PipelineStageFlags::TRANSFER,
        &[memory_barrier]
    );
    // Update buffer
}

// ❌ BAD: No synchronization
fn update_buffer(&mut self) -> VulkanResult<()> {
    // Direct buffer update without barriers
}
```

#### GPU Parallelism Best Practices
```rust
// ✅ GOOD: SIMT-friendly uniform execution
if (material_type == STANDARD_PBR) {
    // All threads in wavefront execute this
}

// ❌ BAD: Branch divergence hurts performance  
if (thread_id % 2 == 0) {
    // Half the wavefront idles while other half executes
}
```

#### Coordinate Systems
```rust
// ✅ GOOD: Explicit coordinate system handling
let normal_matrix = if let Some(inverse) = mat3_model.try_inverse() {
    inverse.transpose()  // Proper inverse-transpose for normals
} else {
    Mat3::identity()
};

// ❌ BAD: Identity matrix for normals
let normal_matrix = Mat3::identity(); // Breaks world-space lighting
```

## 🟢 DEVELOPMENT WORKFLOWS

### Feature Development Process

#### 1. Planning Phase
- [ ] Read existing architecture documentation
- [ ] Understand ECS integration requirements
- [ ] Consider Vulkan compatibility
- [ ] Plan backward compatibility strategy

#### 2. Implementation Phase
- [ ] Create baseline screenshot before changes
- [ ] Implement feature in small, testable increments
- [ ] Build and test frequently (`cargo build`)
- [ ] Document architectural decisions

#### 3. Validation Phase
- [ ] Screenshot validation with multiple test cases
- [ ] Performance testing (maintain 60+ FPS)
- [ ] Vulkan validation (zero errors)
- [ ] Code review for safety and idioms

#### 4. Integration Phase
- [ ] Update documentation
- [ ] Clean up dead code
- [ ] Verify backward compatibility
- [ ] Final screenshot validation

### Debugging Workflow

#### Visual Issues
1. **Screenshot Comparison**: Use baseline vs current screenshots
2. **Validation Layers**: Enable Vulkan validation for detailed errors
3. **Frame Debugging**: Use RenderDoc or similar tools when needed
4. **Component Testing**: Isolate rendering components

#### Performance Issues
1. **FPS Monitoring**: Maintain 60+ FPS requirement
2. **GPU Profiling**: Use Vulkan timeline semaphores
3. **Memory Usage**: Monitor GPU memory allocation
4. **Command Buffer Analysis**: Review command submission patterns

### Testing Standards

#### Unit Testing
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_material_creation() {
        let material = MaterialConfig::new()
            .with_color(Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(material.color(), Vec3::new(1.0, 0.0, 0.0));
    }
}
```

#### Integration Testing
- Test with real Vulkan context
- Validate shader compilation
- Test pipeline state changes
- Verify resource cleanup

## ⚡ PERFORMANCE REQUIREMENTS

### Frame Rate
- **Minimum**: 60 FPS with 18,960 vertex teapot
- **Target**: 120+ FPS for simple scenes
- **Stress Test**: Multiple lights with shadows

### Memory Usage
- **GPU Memory**: Efficient allocation/deallocation
- **CPU Memory**: Minimal unnecessary allocations
- **Leak Detection**: Zero memory leaks in release builds

### Vulkan Validation
- **Development**: All validation layers enabled
- **Production**: Clean validation with zero errors
- **Synchronization**: Proper pipeline barriers and dependencies

## � GPU PARALLELISM AND CONCURRENCY RULES

### Critical GPU Synchronization Requirements
**Rule**: All GPU operations must follow explicit synchronization patterns

#### Memory Barrier Requirements
```rust
// ✅ REQUIRED: Proper UBO update synchronization
self.cmd_pipeline_barrier(
    vk::PipelineStageFlags::HOST,
    vk::PipelineStageFlags::VERTEX_SHADER | vk::PipelineStageFlags::FRAGMENT_SHADER,
    &[vk::MemoryBarrier {
        src_access_mask: vk::AccessFlags::HOST_WRITE,
        dst_access_mask: vk::AccessFlags::UNIFORM_READ,
        ..Default::default()
    }]
);

// ❌ FORBIDDEN: Direct buffer updates without barriers
buffer.write_data(data)?; // No synchronization - RACE CONDITION
```

#### Thread Safety Requirements
**Rule**: All Vulkan resources must be thread-safe or explicitly documented as single-threaded

```rust
// ✅ REQUIRED: Per-thread command pools
struct ThreadSafeCommandPool {
    pools: Vec<Mutex<CommandPool>>,
    current_index: AtomicUsize,
}

// ❌ FORBIDDEN: Shared command pools without protection
struct CommandPool {
    // Shared between threads - UNSAFE
}
```

#### Performance Critical Rules
- **FORBIDDEN**: `device_wait_idle()` except during cleanup/shutdown
- **REQUIRED**: Frame-in-flight resource isolation (3x buffering minimum)
- **REQUIRED**: Proper fence-based CPU-GPU synchronization

### GPU Memory Management Rules
**Rule**: Follow SIMT-friendly memory access patterns

#### Buffer Allocation Strategy
```rust
// ✅ GOOD: Large buffer with offsets (GPU-friendly)
let large_buffer = allocate_buffer(materials.len() * size_of::<MaterialUBO>());

// ❌ BAD: Many small allocations (inefficient for GPU)
for material in materials {
    let buffer = allocate_buffer(size_of::<MaterialUBO>());
}
```

#### Synchronization Point Discipline
- **Per-Frame**: Maximum one fence wait at frame start
- **Per-Pass**: Pipeline barriers only between render passes  
- **Never**: Synchronous GPU operations in render loop

## �🔧 TOOL USAGE

### Screenshot Tool
```cmd
:: Manual screenshot with analysis
cd tools\screenshot_tool
cargo run -- --analyze "path\to\screenshot.png"

:: Expected output for success:
:: Content Classification: RenderedScene
:: Colored Pixels: 98.2%
:: Average Brightness: 151/255
```

### Build Scripts
- **Shader Compilation**: Automatic via build.rs
- **Asset Processing**: Use dedicated tools in `tools/` folder
- **Dependency Updates**: Regular Cargo.toml maintenance

### Validation Commands
```cmd
:: Standard validation sequence
cargo build                                    # Compilation check
.\tools\validate_rendering.bat baseline        # Visual baseline
# Make changes
.\tools\validate_rendering.bat validation      # Visual validation
git add . && git commit -m "descriptive message"
```

## 📚 DOCUMENTATION STANDARDS

### Code Documentation
- **Public APIs**: Comprehensive rustdoc comments
- **Complex Logic**: Inline explanations for non-obvious code
- **Architecture**: High-level design documentation
- **Examples**: Working code examples in documentation

### Commit Messages
```
Format: <type>: <description>

Examples:
fix: correct normal matrix calculation for world-space lighting
feat: add multi-light support with UBO backend
refactor: consolidate shader management systems
docs: update architecture documentation
test: add screenshot validation for material system
```

### Architecture Updates
- Update `ARCHITECTURE.md` for major changes
- Document breaking changes and migration paths
- Maintain TODO.md with current priorities
- Record troubleshooting procedures

## 🚫 FORBIDDEN PRACTICES

### Never Do This
- ❌ Raw `unwrap()` calls in library code
- ❌ Direct Vulkan handle exposure outside owning structs
- ❌ Hardcoded paths bypassing configuration systems
- ❌ Commits without screenshot validation (rendering changes)
- ❌ Adding dependencies without architectural review
- ❌ Breaking backward compatibility without migration plan
- ❌ Ignoring Vulkan validation errors
- ❌ Performance regressions without justification
- ❌ **`device_wait_idle()` in render loops (destroys GPU parallelism)**
- ❌ **Unsynchronized buffer updates (race conditions)**
- ❌ **Shared command pools without thread safety**
- ❌ **Missing memory barriers for resource transitions**

### Code Smells
- 🚨 Duplicate code across modules
- 🚨 Large functions (>50 lines)
- 🚨 Deeply nested conditionals
- 🚨 Mutable global state
- 🚨 Resource leaks or cleanup failures
- 🚨 Inconsistent error handling patterns
- 🚨 **Excessive CPU-GPU synchronization points**
- 🚨 **Fine-grained memory allocations**
- 🚨 **Branch divergence in shaders**
