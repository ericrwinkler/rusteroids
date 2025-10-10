# Rusteroids Development TODO

## 🚨 **EMERGENCY PRIORITY**

### 1. **Fix Multiple Objects Rendering Architecture**

**Problem**: Current implementation has catastrophic performance due to immediate-mode rendering pattern.

**Current Issues**:
- Creating 189,600 vertices per frame (10 teapots × 18,960 vertices each)
- CPU-side vertex transformation every frame  
- Uploading 758KB mesh data per frame to GPU
- `draw_mesh_3d()` calls `draw_frame()` internally, preventing batching
- Single averaged material instead of per-object materials

**Target Performance**: 10+ objects at 60+ FPS with different materials

#### **Implementation Plan**

##### **Step 1: Design Per-Object Architecture**

**GameObject Structure** (based on Vulkan tutorial):
```rust
// crates/rust_engine/src/render/game_object.rs
pub struct GameObject {
    // Transform properties
    pub position: Vec3,
    pub rotation: Vec3,
    pub scale: Vec3,
    
    // Per-object GPU resources (one per frame-in-flight)
    pub uniform_buffers: Vec<Buffer>,
    pub uniform_memory: Vec<DeviceMemory>, 
    pub uniform_mapped: Vec<*mut u8>,
    
    // Per-object descriptor sets (one per frame-in-flight)
    pub descriptor_sets: Vec<DescriptorSet>,
    
    // Material for this object
    pub material: Material,
}

impl GameObject {
    pub fn get_model_matrix(&self) -> Mat4 {
        let translation = Mat4::new_translation(&self.position);
        let rotation_x = Mat4::rotation_x(self.rotation.x);
        let rotation_y = Mat4::rotation_y(self.rotation.y);
        let rotation_z = Mat4::rotation_z(self.rotation.z);
        let scale = Mat4::new_scaling(self.scale.x);
        translation * rotation_z * rotation_y * rotation_x * scale
    }
}
```

**Shared Resources Structure**:
```rust
// crates/rust_engine/src/render/shared_resources.rs
pub struct SharedRenderingResources {
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub pipeline: Pipeline,
    pub descriptor_set_layout: DescriptorSetLayout,
}
```

##### **Step 2: Modify Renderer Architecture**

**New Renderer API**:
```rust
// crates/rust_engine/src/render/mod.rs
impl Renderer {
    /// Begin frame and start command buffer recording
    pub fn begin_frame(&mut self) -> Result<(), RenderError> {
        // 1. Acquire swapchain image
        // 2. Begin command buffer recording
        // 3. Begin render pass
        // 4. Set viewport and scissor
    }
    
    /// Bind shared resources (call once per frame)
    pub fn bind_shared_resources(&mut self, shared: &SharedRenderingResources) -> Result<(), RenderError> {
        // 1. Bind vertex buffer
        // 2. Bind index buffer  
        // 3. Bind graphics pipeline
    }
    
    /// Draw single object with its own uniform buffer
    pub fn draw_object(&mut self, object: &GameObject, frame_index: usize) -> Result<(), RenderError> {
        // 1. Update object's uniform buffer with current model matrix
        // 2. Bind object's descriptor set for this frame
        // 3. Record draw indexed command (no submission)
    }
    
    /// End frame and submit all recorded commands
    pub fn end_frame(&mut self, window: &mut WindowHandle) -> Result<(), RenderError> {
        // 1. End render pass
        // 2. End command buffer recording
        // 3. Submit command buffer
        // 4. Present swapchain image
    }
}
```

**Usage Pattern**:
```rust
// teapot_app/src/main_dynamic.rs
fn render_frame(&mut self) -> Result<(), Box<dyn std::error::Error>> {
    // Begin frame
    self.renderer.begin_frame()?;
    
    // Set camera and lighting (once per frame)
    self.renderer.set_camera(&self.camera);
    self.renderer.set_multi_light_environment(&multi_light_env);
    
    // Bind shared geometry (once per frame)
    self.renderer.bind_shared_resources(&self.shared_resources)?;
    
    // Draw each object with its own transform and material
    for object in &self.game_objects {
        self.renderer.draw_object(object, self.current_frame)?;
    }
    
    // Submit all commands and present
    self.renderer.end_frame(&mut self.window)?;
    
    Ok(())
}
```

##### **Step 3: Implement Per-Object Uniform Buffers**

**Uniform Buffer Object Structure**:
```rust
// crates/rust_engine/src/render/uniforms.rs
#[repr(C, align(16))]
pub struct ObjectUBO {
    pub model_matrix: [[f32; 4]; 4],
    pub normal_matrix: [[f32; 3]; 3],  // For lighting calculations
}
```

**GameObject Resource Creation**:
```rust
impl GameObject {
    pub fn create_resources(
        &mut self, 
        device: &Device, 
        descriptor_pool: &DescriptorPool,
        descriptor_set_layout: &DescriptorSetLayout,
        max_frames_in_flight: usize
    ) -> Result<(), RenderError> {
        // Create uniform buffers for each frame in flight
        for i in 0..max_frames_in_flight {
            let buffer_size = std::mem::size_of::<ObjectUBO>() as u64;
            
            // Create uniform buffer
            let (buffer, memory) = create_buffer(
                device,
                buffer_size,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT
            )?;
            
            // Map memory for updates
            let mapped = unsafe { 
                device.map_memory(memory, 0, buffer_size, vk::MemoryMapFlags::empty())? 
            };
            
            self.uniform_buffers.push(buffer);
            self.uniform_memory.push(memory);
            self.uniform_mapped.push(mapped as *mut u8);
        }
        
        // Create descriptor sets
        self.create_descriptor_sets(device, descriptor_pool, descriptor_set_layout, max_frames_in_flight)?;
        
        Ok(())
    }
    
    pub fn update_uniform_buffer(&self, frame_index: usize) -> Result<(), RenderError> {
        let ubo = ObjectUBO {
            model_matrix: self.get_model_matrix().into(),
            normal_matrix: self.calculate_normal_matrix().into(),
        };
        
        unsafe {
            let mapped = self.uniform_mapped[frame_index];
            std::ptr::copy_nonoverlapping(
                &ubo as *const ObjectUBO as *const u8,
                mapped,
                std::mem::size_of::<ObjectUBO>()
            );
        }
        
        Ok(())
    }
}
```

##### **Step 4: Update Descriptor Pool Size**

**Modified Descriptor Pool Creation**:
```rust
// crates/rust_engine/src/render/vulkan/ubo_manager.rs
pub fn create_descriptor_pool(device: &Device, max_objects: u32, max_frames_in_flight: u32) -> Result<DescriptorPool, RenderError> {
    let pool_sizes = [
        // Per-frame descriptor sets (camera + lighting)
        vk::DescriptorPoolSize {
            ty: vk::DescriptorType::UNIFORM_BUFFER,
            descriptor_count: max_frames_in_flight * 2, // Camera UBO + Lighting UBO
        },
        // Per-object descriptor sets (model matrices)
        vk::DescriptorPoolSize {
            ty: vk::DescriptorType::UNIFORM_BUFFER,  
            descriptor_count: max_objects * max_frames_in_flight,
        },
        // Texture samplers
        vk::DescriptorPoolSize {
            ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
            descriptor_count: max_objects * max_frames_in_flight * 3, // Base, normal, metallic-roughness
        },
    ];
    
    let pool_info = vk::DescriptorPoolCreateInfo::builder()
        .flags(vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET)
        .max_sets(max_frames_in_flight + (max_objects * max_frames_in_flight))
        .pool_sizes(&pool_sizes);
    
    unsafe { device.create_descriptor_pool(&pool_info, None) }
        .map_err(RenderError::VulkanError)
}
```

##### **Step 5: Implement Command Buffer Recording**

**Modified VulkanRenderer**:
```rust
// crates/rust_engine/src/render/vulkan/renderer/mod.rs
impl VulkanRenderer {
    pub fn begin_command_recording(&mut self) -> VulkanResult<()> {
        // Wait for previous frame
        self.sync_manager.wait_for_frame(self.current_frame)?;
        
        // Acquire swapchain image
        let (image_index, _) = self.swapchain_manager.acquire_next_image(
            &self.context,
            self.sync_manager.get_image_available_semaphore(self.current_frame)
        )?;
        
        self.current_image_index = image_index;
        
        // Begin command buffer
        let command_buffer = self.command_recorder.get_command_buffer(self.current_frame);
        self.command_recorder.begin_recording(command_buffer)?;
        
        // Begin render pass
        self.command_recorder.begin_render_pass(
            command_buffer,
            &self.swapchain_manager.get_framebuffer(image_index as usize),
            self.swapchain_manager.get_extent()
        )?;
        
        Ok(())
    }
    
    pub fn record_draw_object(&mut self, descriptor_set: vk::DescriptorSet, index_count: u32) -> VulkanResult<()> {
        let command_buffer = self.command_recorder.get_command_buffer(self.current_frame);
        
        // Bind per-object descriptor set
        self.command_recorder.bind_descriptor_sets(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            self.pipeline_manager.get_current_pipeline_layout(),
            1, // Set index for per-object data
            &[descriptor_set]
        )?;
        
        // Record draw command
        self.command_recorder.draw_indexed(command_buffer, index_count);
        
        Ok(())
    }
    
    pub fn end_command_recording_and_submit(&mut self) -> VulkanResult<()> {
        let command_buffer = self.command_recorder.get_command_buffer(self.current_frame);
        
        // End render pass and command buffer
        self.command_recorder.end_render_pass(command_buffer);
        self.command_recorder.end_recording(command_buffer)?;
        
        // Submit and present
        self.sync_manager.submit_and_present(
            &self.context,
            command_buffer,
            self.sync_manager.get_image_available_semaphore(self.current_frame),
            self.current_image_index,
            self.current_frame
        )?;
        
        // Advance frame
        self.current_frame = (self.current_frame + 1) % self.max_frames_in_flight;
        
        Ok(())
    }
}
```

##### **Step 6: Update Dynamic Teapot Application**

**Modified Application Structure**:
```rust
// teapot_app/src/main_dynamic.rs
pub struct DynamicTeapotApp {
    window: WindowHandle,
    renderer: Renderer,
    camera: Camera,
    
    // Shared resources
    shared_resources: SharedRenderingResources,
    teapot_mesh_data: (Vec<Vertex>, Vec<u32>), // Original mesh data
    
    // Per-object data
    game_objects: Vec<GameObject>,
    
    // Lighting system
    world: World,
    lighting_system: EcsLightingSystem,
    
    // Frame management
    current_frame: usize,
    max_frames_in_flight: usize,
    start_time: Instant,
}

impl DynamicTeapotApp {
    pub fn new() -> Self {
        // ... initialization ...
        
        // Create game objects with different positions, rotations, materials
        let mut game_objects = Vec::new();
        let grid_size = 4; // 4x4 grid of teapots
        let spacing = 3.0;
        
        for i in 0..10 {
            let row = i / grid_size;
            let col = i % grid_size;
            let x = (col as f32 - grid_size as f32 * 0.5) * spacing;
            let z = (row as f32 - grid_size as f32 * 0.5) * spacing;
            
            let mut object = GameObject {
                position: Vec3::new(x, 0.0, z),
                rotation: Vec3::new(0.0, i as f32 * 0.3, 0.0),
                scale: Vec3::new(0.8, 0.8, 0.8),
                material: self.create_material_for_object(i),
                uniform_buffers: Vec::new(),
                uniform_memory: Vec::new(),
                uniform_mapped: Vec::new(),
                descriptor_sets: Vec::new(),
            };
            
            // Create GPU resources for this object
            object.create_resources(
                &device,
                &descriptor_pool,
                &descriptor_set_layout,
                max_frames_in_flight
            )?;
            
            game_objects.push(object);
        }
        
        Self {
            game_objects,
            // ... other fields ...
        }
    }
    
    fn create_material_for_object(&self, index: usize) -> Material {
        let colors = [
            Vec3::new(0.9, 0.85, 0.7),   // Cream ceramic
            Vec3::new(0.7, 0.4, 0.3),    // Terracotta
            Vec3::new(0.95, 0.93, 0.88), // Silver
            Vec3::new(1.0, 0.8, 0.2),    // Gold
            Vec3::new(0.2, 0.6, 0.9),    // Blue
            Vec3::new(0.8, 0.2, 0.3),    // Red
            Vec3::new(0.3, 0.8, 0.3),    // Green
            Vec3::new(0.8, 0.3, 0.8),    // Magenta
            Vec3::new(0.9, 0.6, 0.1),    // Orange
            Vec3::new(0.6, 0.4, 0.8),    // Purple
        ];
        
        Material::standard_pbr(StandardMaterialParams {
            base_color: colors[index % colors.len()],
            alpha: 1.0,
            metallic: if index % 3 == 0 { 0.9 } else { 0.1 },
            roughness: if index % 3 == 0 { 0.1 } else { 0.4 },
            ..Default::default()
        })
    }
}

impl Application for DynamicTeapotApp {
    fn render_frame(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Begin frame
        self.renderer.begin_frame()?;
        
        // Set per-frame uniforms
        self.renderer.set_camera(&self.camera);
        let multi_light_env = self.build_multi_light_environment_from_entities();
        self.renderer.set_multi_light_environment(&multi_light_env);
        
        // Bind shared resources
        self.renderer.bind_shared_resources(&self.shared_resources)?;
        
        // Update and draw each object
        let elapsed = self.start_time.elapsed().as_secs_f32();
        for (i, object) in self.game_objects.iter_mut().enumerate() {
            // Update object animation
            object.rotation.y = elapsed * 0.5 + i as f32 * 0.3;
            object.position.y = (elapsed * 1.2 + i as f32 * 0.5).sin() * 1.0;
            
            // Update uniform buffer
            object.update_uniform_buffer(self.current_frame)?;
            
            // Record draw command
            self.renderer.draw_object(object, self.current_frame)?;
        }
        
        // Submit all commands
        self.renderer.end_frame(&mut self.window)?;
        
        Ok(())
    }
}
```

##### **Step 7: Success Criteria & Validation**

**Performance Targets**:
- [ ] 10+ objects rendering at 60+ FPS
- [ ] Each object has different material (colors visible)
- [ ] GPU memory usage <5MB (vs 758KB/frame currently)
- [ ] Clean Vulkan validation (zero errors)

**Testing Protocol**:
```bash
# Build and run
cargo build --release
cargo run --bin teapot_dynamic

# Validate performance
# Should see: "Rendering 10 objects at 60+ FPS with individual materials"

# Screenshot validation
.\tools\validate_rendering.bat validation
# Should see: RenderedScene with >60% colored pixels, multiple colors visible
```

**Rollback Plan**: If implementation fails, revert to single object rendering until issues resolved.

---

## 🟡 **HIGH PRIORITY** 

### 2. **Multi-Light System Enhancement** 
*Status: Core implementation complete, needs optimization*

**Completed**:
- ✅ MultiLightUBO supporting 4 directional + 8 point + 4 spot lights
- ✅ All shaders process light arrays
- ✅ ECS LightingSystem converts entities to GPU format

**Remaining Work**:
- [ ] Shadow mapping for directional lights
- [ ] Light culling for performance with many lights
- [ ] Dynamic light addition/removal during runtime

---

## 🟢 **MEDIUM PRIORITY**

### 3. **Asset Pipeline System**

**Goal**: Comprehensive asset loading with validation and optimization.

**Components Needed**:
- [ ] Model loader supporting .obj, .gltf, .fbx
- [ ] Texture loader with format conversion
- [ ] Shader hot-reloading for development
- [ ] Asset validation and error reporting

### 4. **Scene Management System**

**Goal**: High-level scene graph with hierarchical transforms.

**Features**:
- [ ] Parent-child transform relationships  
- [ ] Scene serialization/deserialization
- [ ] Frustum culling optimization
- [ ] Level-of-detail (LOD) system

---

## 🔵 **LOW PRIORITY**

### 5. **Thread-Safe Engine Foundation**

**Goal**: Prepare for multi-threaded system execution.

**Implementation**:
- [ ] Thread-safe component storage
- [ ] System dependency graph
- [ ] Parallel system execution
- [ ] Lock-free data structures where possible

### 6. **Advanced Rendering Features**

**Post-Processing**:
- [ ] Screen-space ambient occlusion (SSAO)
- [ ] Temporal anti-aliasing (TAA)
- [ ] Bloom and tone mapping
- [ ] Debug visualization modes

**Lighting**:
- [ ] Shadow mapping (cascade, omnidirectional)
- [ ] Image-based lighting (IBL)
- [ ] Area lights
- [ ] Volumetric lighting

---

## 📋 **DEVELOPMENT PROCESS**

### Feature Implementation Workflow

1. **Design Phase**
   - [ ] Update TODO.md with detailed implementation plan
   - [ ] Review architecture for compatibility
   - [ ] Identify performance requirements

2. **Implementation Phase**  
   - [ ] Create feature branch
   - [ ] Implement with unit tests
   - [ ] Validate with screenshot tool
   - [ ] Performance testing

3. **Integration Phase**
   - [ ] Code review
   - [ ] Integration testing
   - [ ] Documentation update
   - [ ] Merge to main

### Quality Gates

**For Each Feature**:
- [ ] Compiles without warnings
- [ ] All tests pass
- [ ] Screenshot validation passes
- [ ] Performance requirements met
- [ ] Documentation updated

**For Each Release**:
- [ ] Full integration test suite
- [ ] Performance benchmarking
- [ ] Memory leak testing
- [ ] Vulkan validation clean
