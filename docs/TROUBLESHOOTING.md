# Rusteroids Troubleshooting Guide

## 🚨 **CRITICAL ISSUES**

### Performance Disaster: Multiple Objects Rendering

**Symptoms**:
- Frame rate drops to <10 FPS with multiple objects
- GPU memory spikes every frame  
- High CPU usage during rendering
- Visual lag and stuttering

**Root Cause**: Immediate-mode rendering pattern
```rust
// BROKEN PATTERN (current)
for object in objects {
    let transformed_mesh = transform_mesh_on_cpu(object);  // ❌ CPU work
    upload_mesh_to_gpu(transformed_mesh);                  // ❌ GPU upload  
    draw_frame();                                          // ❌ Immediate submit
}
```

**Solution**: Command recording pattern
```rust  
// CORRECT PATTERN (target)
renderer.begin_frame();
renderer.bind_shared_geometry();                          // ✅ Once per frame
for object in objects {
    object.update_uniform_buffer();                       // ✅ Small UBO update
    renderer.record_draw_command(object);                 // ✅ Record only
}
renderer.submit_and_present();                           // ✅ Single submit
```

**Fix Status**: See TODO_CONSOLIDATED.md → Emergency Priority → Multiple Objects Architecture

---

### Vulkan Validation Errors

**Common Issues & Solutions**:

#### 1. **Descriptor Set Validation**
```
VUID-vkCmdDrawIndexed-None-02699: Descriptor set must be bound
```

**Cause**: Drawing without binding per-object descriptor sets
**Fix**: 
```rust
// Ensure descriptor set is bound before drawing
renderer.bind_descriptor_sets(command_buffer, descriptor_sets);
renderer.draw_indexed(command_buffer, index_count);
```

#### 2. **Buffer Memory Mapping**
```
VUID-vkMapMemory-memory-00678: Memory must be host-visible
```

**Cause**: Trying to map device-local memory
**Fix**:
```rust
// Use host-visible memory for uniform buffers
let memory_properties = vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT;
```

#### 3. **Synchronization Issues**
```
VUID-vkQueueSubmit-pCommandBuffers-00071: Command buffer in invalid state
```

**Cause**: Submitting command buffer before recording is complete
**Fix**:
```rust
// Ensure proper command buffer lifecycle
command_recorder.begin_recording(cmd_buf)?;
// ... record commands ...
command_recorder.end_recording(cmd_buf)?;
queue.submit(cmd_buf)?;
```

---

## ⚠️ **BUILD ISSUES**

### Cargo Build Failures

#### **Shader Compilation Errors**
```
error: failed to run custom build script for `rust_engine`
```

**Diagnosis**:
```powershell
# Check if glslc is available
glslc --version

# Manually compile shaders to see errors
glslc resources/shaders/standard_pbr_vert.vert -o target/shaders/standard_pbr_vert.spv
```

**Solutions**:
1. **Missing Vulkan SDK**:
   ```powershell
   # Download and install Vulkan SDK
   # Add to PATH: C:\VulkanSDK\<version>\Bin
   ```

2. **Shader Syntax Errors**:
   ```glsl
   // Check shader version compatibility
   #version 450 core  // Ensure version matches
   ```

3. **Missing Shader Files**:
   ```bash
   # Verify all shaders exist
   ls resources/shaders/
   ```

#### **Dependency Resolution**
```
error: could not compile `glfw-sys` due to 2 previous errors
```

**Solutions**:
1. **CMake Missing**:
   ```powershell
   choco install cmake
   # Or download from cmake.org
   ```

2. **MSVC Build Tools**:
   ```powershell
   # Install Visual Studio Build Tools
   # Or full Visual Studio with C++ development tools
   ```

---

## 🔧 **RUNTIME ISSUES**

### Application Crashes

#### **Segmentation Fault on Startup**
```
thread 'main' panicked at 'index out of bounds'
```

**Common Causes**:
1. **Uninitialized Vulkan Objects**
   ```rust
   // Check all Vulkan objects are properly created
   assert!(!device.is_null());
   assert!(!swapchain.is_null());
   ```

2. **Invalid Buffer Access**
   ```rust
   // Ensure buffer indices are valid
   assert!(vertex_index < vertex_buffer.len());
   ```

#### **Window Creation Failure**
```
Failed to create GLFW window
```

**Diagnosis**:
```rust
// Check GLFW initialization
if !glfw.init().is_ok() {
    panic!("Failed to initialize GLFW");
}

// Verify Vulkan support
window.hints.client_api(glfw::ClientApiHint::NoApi);
window.hints.resizable(false);
```

**Solutions**:
1. **Graphics Driver Update**: Update GPU drivers
2. **Vulkan Support**: Verify GPU supports Vulkan 1.0+
3. **Window Hints**: Ensure proper GLFW window hints

---

### Memory Issues

#### **Memory Leaks**
```
Vulkan objects not properly destroyed
```

**Detection**:
```rust
// Add debug tracking
impl Drop for VulkanRenderer {
    fn drop(&mut self) {
        println!("Destroying Vulkan objects...");
        // Ensure all cleanup is called
    }
}
```

**Prevention**:
```rust
// Use RAII patterns
struct BufferGuard {
    buffer: vk::Buffer,
    memory: vk::DeviceMemory,
    device: Device,
}

impl Drop for BufferGuard {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_buffer(self.buffer, None);
            self.device.free_memory(self.memory, None);
        }
    }
}
```

#### **GPU Memory Exhaustion**
```
VK_ERROR_OUT_OF_DEVICE_MEMORY
```

**Diagnosis**:
```rust
// Monitor memory usage
let memory_props = instance.get_physical_device_memory_properties(physical_device);
for i in 0..memory_props.memory_heap_count {
    println!("Heap {}: {} MB", i, memory_props.memory_heaps[i as usize].size / 1024 / 1024);
}
```

**Solutions**:
1. **Buffer Reuse**: Share vertex buffers between objects
2. **Memory Pooling**: Implement custom memory allocator
3. **Resource Cleanup**: Properly destroy unused resources

---

## 🎯 **VISUAL ISSUES**

### Rendering Problems

#### **Black Screen**
**Checklist**:
- [ ] Shaders compiled successfully
- [ ] Vertex data uploaded correctly  
- [ ] Camera positioned to see objects
- [ ] Lighting configured (objects may be too dark)
- [ ] Depth testing enabled

**Debug Steps**:
```rust
// 1. Verify vertex data
println!("Vertex count: {}", vertices.len());
println!("First vertex: {:?}", vertices[0]);

// 2. Check camera position
println!("Camera pos: {:?}", camera.position);
println!("Camera target: {:?}", camera.target);

// 3. Verify shader uniforms
println!("Model matrix: {:?}", model_matrix);
println!("View matrix: {:?}", view_matrix);
```

#### **Objects Not Appearing**
**Common Issues**:
1. **Camera Position**: Objects behind camera or outside view frustum
2. **Scale Issues**: Objects too small/large to see
3. **Culling Problems**: Wrong winding order causing backface culling
4. **Depth Issues**: Objects behind other geometry

**Solutions**:
```rust
// Reset camera to known good position
camera.position = Vec3::new(0.0, 2.0, 8.0);
camera.target = Vec3::new(0.0, 0.0, 0.0);

// Check object bounds
let bounds = calculate_mesh_bounds(&vertices);
println!("Mesh bounds: {:?}", bounds);

// Disable culling temporarily
rasterization_state.cull_mode = vk::CullModeFlags::NONE;
```

#### **Wrong Colors/Materials**
**Diagnosis**:
```rust
// Verify material data reaches shader
println!("Material base color: {:?}", material.base_color);
println!("Material metallic: {}", material.metallic);

// Check lighting setup
println!("Light count: {}", lights.len());
println!("Light direction: {:?}", lights[0].direction);
```

---

## 🛠️ **DEVELOPMENT TOOLS**

### Debug Utilities

#### **Screenshot Validation**
```powershell
# Automated visual testing
.\tools\screenshot_tool\validate_rendering.bat

# Manual screenshot capture
cargo run --bin screenshot_tool -- --output validation_screenshot.png
```

**Interpreting Results**:
- **Green**: Successful render with expected content
- **Yellow**: Warning - content detected but may be incorrect
- **Red**: Failure - no content or error state

#### **Performance Profiling**
```rust
// Add timing measurements
let start = std::time::Instant::now();
renderer.render_frame()?;
let duration = start.elapsed();
println!("Frame time: {:?}", duration);

// GPU memory tracking
let memory_info = device.get_memory_properties();
println!("GPU memory usage: {} MB", get_memory_usage() / 1024 / 1024);
```

#### **Vulkan Debug Messages**
```rust
// Enable validation layers
let validation_layers = vec!["VK_LAYER_KHRONOS_validation"];

// Set up debug messenger for detailed error info
let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
    .message_severity(vk::DebugUtilsMessageSeverityFlagsEXT::all())
    .message_type(vk::DebugUtilsMessageTypeFlagsEXT::all())
    .pfn_user_callback(Some(debug_callback));
```

---

## 📊 **PERFORMANCE DEBUGGING**

### Frame Rate Analysis

#### **Low FPS Diagnosis**
```rust
// Measure different phases
struct FrameTimer {
    cpu_time: Duration,
    gpu_time: Duration,
    present_time: Duration,
}

fn profile_frame(&mut self) -> FrameTimer {
    let start = Instant::now();
    
    // CPU work
    let cpu_start = Instant::now();
    self.update_game_objects();
    let cpu_time = cpu_start.elapsed();
    
    // GPU work  
    let gpu_start = Instant::now();
    self.renderer.render_frame();
    let gpu_time = gpu_start.elapsed();
    
    // Present
    let present_start = Instant::now();
    self.renderer.present();
    let present_time = present_start.elapsed();
    
    FrameTimer { cpu_time, gpu_time, present_time }
}
```

#### **Memory Usage Tracking**
```rust
// Track allocations
struct MemoryTracker {
    vertex_buffer_size: usize,
    uniform_buffer_size: usize,
    texture_memory: usize,
}

impl MemoryTracker {
    fn log_usage(&self) {
        println!("Vertex buffers: {} KB", self.vertex_buffer_size / 1024);
        println!("Uniform buffers: {} KB", self.uniform_buffer_size / 1024);
        println!("Textures: {} KB", self.texture_memory / 1024);
        println!("Total GPU: {} KB", self.total() / 1024);
    }
}
```

---

## 🚀 **OPTIMIZATION STRATEGIES**

### Performance Improvements

#### **Vulkan Best Practices**
1. **Minimize State Changes**
   ```rust
   // Group draws by pipeline, then by vertex buffer, then by texture
   for pipeline in pipelines {
       bind_pipeline(pipeline);
       for vertex_buffer in vertex_buffers {
           bind_vertex_buffer(vertex_buffer);
           for material in materials {
               bind_material(material);
               // Draw all objects with this state
           }
       }
   }
   ```

2. **Efficient Uniform Updates**
   ```rust
   // Use push constants for small, frequently changing data
   let push_constants = ObjectPushConstants {
       model_matrix: object.get_model_matrix(),
   };
   cmd_buffer.push_constants(pipeline_layout, push_constants);
   
   // Use dynamic uniform buffers for larger data
   let offset = object_index * dynamic_alignment;
   cmd_buffer.bind_descriptor_sets_with_offsets(descriptor_set, &[offset]);
   ```

3. **Memory Management**
   ```rust
   // Allocate large memory chunks and sub-allocate
   struct MemoryPool {
       memory: DeviceMemory,
       free_ranges: Vec<Range<u64>>,
   }
   
   impl MemoryPool {
       fn allocate(&mut self, size: u64) -> Option<u64> {
           // Find suitable free range
           // Return offset into pool
       }
   }
   ```

---

## 📝 **DEBUGGING CHECKLIST**

### Before Reporting Issues

**Environment**:
- [ ] Vulkan SDK version: ________________
- [ ] GPU model: ________________________
- [ ] Driver version: ___________________
- [ ] Windows version: __________________

**Build Status**:
- [ ] `cargo build` succeeds without warnings
- [ ] All shaders compile successfully
- [ ] No Vulkan validation errors
- [ ] Screenshot tool passes validation

**Runtime Behavior**:
- [ ] Application starts without crashing
- [ ] Window appears with correct size
- [ ] Frame rate is acceptable (>30 FPS)
- [ ] Visual output matches expectations

**Performance Metrics**:
- [ ] Frame time: _____________ ms
- [ ] GPU memory usage: _______ MB  
- [ ] Object count: ____________
- [ ] Vertex count per frame: ___________

### Issue Reporting Template

```markdown
## Issue Description
Brief description of the problem

## Environment
- OS: Windows [version]
- GPU: [model and driver version]
- Vulkan SDK: [version]

## Steps to Reproduce
1. Step one
2. Step two
3. Step three

## Expected Behavior
What should happen

## Actual Behavior  
What actually happens

## Performance Data
- Frame rate: X FPS
- Memory usage: X MB
- Validation errors: Yes/No

## Additional Context
Any other relevant information
```
