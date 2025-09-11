# Rusteroids Troubleshooting Guide

## 🚨 Common Issues & Quick Solutions

### Visual Rendering Problems

#### Issue: Lighting Rotates with Object
**Symptoms**: Light appears to move with teapot rotation instead of staying fixed in world space
**Root Cause**: Incorrect normal matrix calculation
**Quick Fix**:
```rust
// Check in command_recorder.rs
let normal_matrix = if let Some(inverse) = mat3_model.try_inverse() {
    inverse.transpose()  // Proper inverse-transpose
} else {
    Mat3::identity()     // Fallback for degenerate matrices
};
```
**Validation**: Use screenshot tool to verify lighting stays fixed during rotation

#### Issue: Performance Drops or GPU Stalls
**Symptoms**: Frame rate suddenly drops below 60 FPS, stuttering, or complete freezes
**Root Cause**: Synchronization bottlenecks destroying GPU parallelism
**Diagnostic Steps**:
1. Check for excessive `device_wait_idle()` calls
2. Look for missing memory barriers causing race conditions
3. Verify proper frame-in-flight isolation

**Critical Patterns to Fix**:
```rust
// ❌ PERFORMANCE KILLER - Remove immediately
device.device_wait_idle(); // In render loop

// ✅ PROPER PATTERN - Use fence-based sync
fence.wait(timeout);
```

#### Issue: Race Conditions and Crashes
**Symptoms**: Random crashes, validation errors, data corruption
**Root Cause**: Thread safety violations in Vulkan resource access
**Common Locations**:
- Command pool sharing between threads
- Descriptor set concurrent updates
- Buffer writes without memory barriers

**Diagnostic Commands**:
```cmd
# Enable all Vulkan validation layers
set VK_LAYER_PATH=%VULKAN_SDK%\Bin
set VK_INSTANCE_LAYERS=VK_LAYER_KHRONOS_validation:VK_LAYER_KHRONOS_synchronization2

# Look for threading and synchronization errors
cargo run 2>&1 | findstr /C:"THREADING" /C:"SYNC"
```

#### Issue: Black/Blank Screen
**Symptoms**: Application window shows solid black or no rendering
**Diagnostic Steps**:
1. Check Vulkan validation layers for errors
2. Verify shader compilation: `cargo build` should show no shader errors
3. Confirm teapot model loads correctly
4. Check descriptor set bindings

**Common Causes**:
- Shader compilation failures
- Missing or incorrect UBO data
- Descriptor set binding errors
- Pipeline state mismatches

#### Issue: Screenshot Tool Shows "UnknownPattern"
**Symptoms**: Screenshot analysis reports non-RenderedScene content
**Diagnostic Commands**:
```cmd
cd tools\screenshot_tool
cargo run -- --analyze "path\to\screenshot.png"
```
**Expected vs Actual**:
- ✅ Expected: `Content Classification: RenderedScene`, >60% colored pixels
- ❌ Actual: `Content Classification: UnknownPattern`, low colored pixels

**Solutions**:
1. Increase wait time: `--wait 6000` for slower initialization
2. Check application is actually rendering (not just blank window)
3. Verify window focus and visibility

### Build and Compilation Issues

#### Issue: Shader Compilation Failures
**Symptoms**: Build fails with shader compilation errors
**Diagnostic**: Check build output for `glslc` errors
**Common Fixes**:
```cmd
# Verify Vulkan SDK is installed and in PATH
where glslc

# Check for shader syntax errors
glslc --version

# Clean and rebuild
cargo clean
cargo build
```

#### Issue: Missing Dependencies
**Symptoms**: Compilation errors about missing crates or features
**Solution**: Verify all workspace crates are properly configured
```cmd
# Build all workspace members
cargo build --workspace

# Check dependency resolution
cargo tree
```

#### Issue: Vulkan Validation Errors
**Symptoms**: Runtime validation layer messages in console
**Diagnostic**: Enable full validation in debug builds
**Common Validation Errors**:

1. **WRITE_AFTER_READ Hazard**:
   ```
   Error: Pipeline barrier needed between vertex reads and buffer updates
   ```
   **Fix**: Add proper pipeline barriers in command recorder

2. **Descriptor Set Binding Errors**:
   ```
   Error: Descriptor set not bound or out of date
   ```
   **Fix**: Verify descriptor set creation and binding order

3. **Memory Barrier Issues**:
   ```
   Error: Missing memory barrier for buffer access
   ```
   **Fix**: Add appropriate access masks to pipeline barriers

### Performance Issues

#### Issue: Low Frame Rate (<60 FPS)
**Diagnostic Steps**:
1. Check CPU usage during rendering
2. Monitor GPU utilization
3. Profile command buffer submission patterns
4. Verify efficient resource usage
5. **NEW**: Check for synchronization bottlenecks

**Common Causes**:
- Excessive GPU synchronization (PRIMARY SUSPECT)
- Inefficient buffer updates
- Too many draw calls
- Large vertex/index buffers
- **Thread safety violations causing stalls**

**Solutions**:
- **CRITICAL**: Remove `device_wait_idle()` from render loops
- Add proper memory barriers instead of heavy synchronization
- Implement dirty tracking for UBO updates
- Use staging buffers for dynamic data
- Batch similar draw operations
- Optimize vertex data layout

#### Issue: GPU Synchronization Bottlenecks
**Symptoms**: Good CPU utilization but poor GPU performance
**Root Cause**: Excessive CPU-GPU synchronization destroying parallelism

**Patterns to Fix**:
```rust
// ❌ TERRIBLE: Sync after every operation
for mesh in meshes {
    update_mesh_buffer(mesh);
    device.wait_idle(); // PERFORMANCE KILLER!
    render_mesh();
}

// ✅ GOOD: Batch operations, minimal sync
update_all_mesh_buffers(meshes);
render_all_meshes();
// Only sync at frame boundaries
```

**Performance Targets**:
- Maximum 1 fence wait per frame
- Memory barriers only at transition points
- No `device_wait_idle()` in render loop

#### Issue: Memory Leaks
**Symptoms**: Increasing memory usage over time
**Diagnostic Tools**:
- Enable Vulkan memory tracking
- Monitor system memory usage
- Check for proper Drop implementations

**Prevention**:
- All Vulkan resources wrapped in RAII structs
- Explicit cleanup in Drop implementations
- No raw handle exposure outside owning structs

## 🔧 Debugging Workflows

### Visual Issue Debugging

#### Step 1: Screenshot Baseline
```cmd
# Capture known-good baseline
.\tools\validate_rendering.bat baseline

# Expected output:
# Content Classification: RenderedScene
# Colored Pixels: 98.2%
# Build completed successfully
```

#### Step 2: Isolate the Problem
1. **Test Simple Scene**: Use basic material without complex shaders
2. **Disable Features**: Comment out advanced rendering features
3. **Check Primitives**: Verify basic triangle/quad rendering works
4. **Progressive Complexity**: Add features back one at a time

#### Step 3: Validate Changes
```cmd
# After each change
.\tools\validate_rendering.bat validation

# Compare with baseline
# Look for visual regressions
```

### Vulkan Debugging

#### Enable Validation Layers
```rust
// In VulkanContext creation
let validation_layers = vec![
    CString::new("VK_LAYER_KHRONOS_validation").unwrap(),
    CString::new("VK_LAYER_KHRONOS_synchronization2").unwrap(), // For threading issues
];
```

#### Common Validation Patterns
```rust
// Proper pipeline barrier pattern
fn safe_buffer_update(&mut self) -> VulkanResult<()> {
    // Wait for any previous vertex reads
    self.cmd_pipeline_barrier(
        vk::PipelineStageFlags::VERTEX_INPUT,
        vk::PipelineStageFlags::TRANSFER,
        &[vk::MemoryBarrier {
            src_access_mask: vk::AccessFlags::VERTEX_ATTRIBUTE_READ,
            dst_access_mask: vk::AccessFlags::TRANSFER_WRITE,
            ..Default::default()
        }]
    );
    
    // Safe to update buffer now
    self.update_vertex_buffer()?;
    Ok(())
}
```

#### Thread Safety Validation
```rust
// Enable threading validation
set VK_LAYER_PATH=%VULKAN_SDK%\Bin
set VK_INSTANCE_LAYERS=VK_LAYER_KHRONOS_validation:VK_LAYER_KHRONOS_synchronization2

// Look for specific threading violations
cargo run 2>&1 | findstr /C:"THREADING_ERROR" /C:"SYNC_ERROR"
```

#### Critical Validation Checks
1. **Memory Barriers**: HOST_WRITE → UNIFORM_READ transitions
2. **Resource Lifecycle**: No access after destruction
3. **Thread Safety**: Command pools and descriptor sets
4. **Synchronization**: Proper fence/semaphore usage

### Performance Debugging

#### Frame Time Analysis
```rust
// Add timing measurements
let start_time = std::time::Instant::now();
renderer.render_frame()?;
let frame_time = start_time.elapsed();

if frame_time.as_millis() > 16 { // >60 FPS
    println!("Slow frame: {}ms", frame_time.as_millis());
}
```

#### GPU Memory Tracking
```rust
// Monitor GPU memory usage
let memory_properties = unsafe {
    instance.get_physical_device_memory_properties(physical_device)
};

// Track allocations per memory type
for (i, heap) in memory_properties.memory_heaps.iter().enumerate() {
    println!("Heap {}: {} MB", i, heap.size / (1024 * 1024));
}
```

## 📋 Diagnostic Checklists

### Pre-Commit Validation Checklist
- [ ] Code compiles without warnings: `cargo build`
- [ ] Baseline screenshot captured before changes
- [ ] Changes implemented and tested
- [ ] Validation screenshot shows expected results
- [ ] No Vulkan validation errors in debug output
- [ ] Performance maintained (60+ FPS)
- [ ] Visual regression test passed

### New Feature Testing Checklist
- [ ] Feature works in isolation
- [ ] Integration with existing systems tested
- [ ] Backward compatibility maintained
- [ ] Performance impact measured
- [ ] Error handling implemented
- [ ] Documentation updated

### Release Preparation Checklist
- [ ] All tests pass: `cargo test`
- [ ] Clean Vulkan validation in release mode
- [ ] Performance benchmarks meet requirements
- [ ] Memory usage is stable
- [ ] Visual quality matches expectations
- [ ] Documentation is up to date

## 🚀 Quick Fix Commands

### Reset to Working State
```cmd
# Clean rebuild
cargo clean
cargo build

# Reset shaders
cd resources\shaders
# Verify all .vert and .frag files exist
dir *.vert *.frag

# Test basic functionality
cd ..\..\teapot_app
cargo run
```

### Emergency Screenshot Validation
```cmd
# Quick validation check
cd tools\screenshot_tool
cargo run -- --prefix "emergency" --wait 4000

# Analyze result immediately
cargo run -- --analyze "screenshots\emergency_*.png"
```

### Vulkan Validation Reset
```cmd
# Ensure clean Vulkan state
set VK_LAYER_PATH=%VULKAN_SDK%\Bin
set VK_INSTANCE_LAYERS=VK_LAYER_KHRONOS_validation

# Run with verbose validation
cargo run 2>&1 | findstr /C:"Validation"
```

## 📞 Escalation Procedures

### When to Seek Help
1. **Vulkan Validation Errors**: Persistent validation failures after following standard fixes
2. **Performance Regression**: >20% FPS drop with no clear cause
3. **Visual Corruption**: Rendering artifacts that don't match known issues
4. **Build System Issues**: Shader compilation failures across multiple systems

### Information to Provide
1. **System Information**: 
   - Vulkan SDK version
   - GPU driver version
   - Rust toolchain version

2. **Error Context**:
   - Complete error messages
   - Steps to reproduce
   - Expected vs actual behavior

3. **Diagnostic Output**:
   - Screenshot comparison (baseline vs current)
   - Vulkan validation layer output
   - Performance measurements

4. **Code Context**:
   - Recent changes made
   - Git commit hash
   - Relevant code snippets

## 📚 Reference Materials

### Vulkan Debugging Resources
- [Vulkan Validation Layers Guide](https://vulkan.lunarg.com/doc/view/latest/windows/validation_layers.html)
- [RenderDoc Integration](https://renderdoc.org/docs/behind_scenes/vulkan_support.html)
- [Hans-Kristian Arntzen's Synchronization Blog](https://themaister.net/blog/2019/08/14/yet-another-blog-explaining-vulkan-synchronization/)

### Performance Analysis Tools
- **Built-in**: Vulkan timeline semaphores for GPU timing
- **External**: RenderDoc for frame analysis
- **System**: Task Manager for CPU/memory monitoring

### Code Quality Tools
- **Rust Analyzer**: IDE integration for real-time error detection
- **Clippy**: Advanced linting for Rust code quality
- **Vulkan Validation Layers**: Runtime validation of Vulkan usage

This troubleshooting guide provides systematic approaches to diagnosing and fixing common issues in the Rusteroids rendering engine.
