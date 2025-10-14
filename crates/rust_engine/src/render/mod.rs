//! # Rendering System
//!
//! This module provides the core rendering abstraction layer for the Rusteroids engine.
//! It acts as a high-level, application-agnostic interface over the Vulkan rendering backend.
//!
//! ## Architecture
//!
//! The rendering system is designed with clear separation of concerns:
//! - **Renderer**: High-level rendering coordinator and API facade
//! - **Camera**: 3D perspective camera with mathematically correct projection matrices
//! - **Vulkan Backend**: Low-level graphics API implementation
//! - **Resource Types**: Meshes, materials, and lighting abstractions
//!
//! ## Design Goals
//!
//! - **API Agnostic**: High-level interface that could theoretically support other backends
//! - **Library-First**: Usable as a standalone rendering library, not tied to specific applications
//! - **Performance**: Efficient resource management with minimal overhead
//! - **Standards Compliance**: Follows academic Vulkan projection matrix guidelines
//!
//! ## Current Limitations (FIXME)
//!
//! - **Backend Coupling**: Some application-specific logic leaks through the abstraction
//! - **Push Constant Limitations**: Complex rendering features constrained by 160-byte limit

pub mod mesh;
pub mod material;
pub mod pipeline;
pub mod lighting;
pub mod coordinates;
pub mod dynamic;
pub mod config;
pub mod window;
pub mod vulkan;
pub mod backend;
pub mod render_queue;
pub mod batch_renderer;
pub mod game_object;
pub mod shared_resources;

pub use mesh::{Mesh, Vertex};
pub use material::{
    Material, MaterialType, MaterialId, PipelineType, AlphaMode,
    StandardMaterialParams, UnlitMaterialParams,
    MaterialManager, TextureManager, MaterialTextures, TextureHandle, TextureType
};
pub use pipeline::{PipelineManager, PipelineConfig, CullMode};
pub use lighting::{Light, LightType, LightingEnvironment};
pub use coordinates::{CoordinateSystem, CoordinateConverter};
pub use config::{VulkanRendererConfig, ShaderConfig};
pub use backend::{RenderBackend, WindowBackendAccess, BackendResult};
pub use window::WindowHandle;
pub use render_queue::{RenderQueue, RenderCommand, CommandType, RenderCommandId};
pub use game_object::{GameObject, ObjectUBO, cleanup_game_object};
pub use shared_resources::{SharedRenderingResources, SharedRenderingResourcesBuilder};
pub use batch_renderer::{BatchRenderer, RenderBatch, BatchError, BatchStats, BatchResult};

use thiserror::Error;
use crate::engine::{RendererConfig, WindowConfig}; // FIXME: Engine coupling - should be library-agnostic
use crate::foundation::math::{Vec3, Mat4, Mat4Ext, utils};
use crate::render::dynamic::{
    MeshPoolManager, MeshType, DynamicObjectHandle,
    SpawnerError, MaterialProperties
};

/// # Graphics Engine
///
/// High-level graphics coordinator that provides an application-friendly interface
/// over the Vulkan rendering backend. Manages the rendering state, camera, lighting,
/// and coordinates frame rendering operations.
///
/// ## Responsibilities
///
/// - **Graphics Coordination**: Orchestrates frame rendering with proper sequencing
/// - **State Management**: Tracks camera, lighting, and per-frame state
/// - **Resource Binding**: Connects high-level resources (meshes, materials) to GPU
/// - **Backend Abstraction**: Hides Vulkan complexity behind clean API
///
/// ## Design Notes
///
/// The GraphicsEngine acts as a facade pattern implementation, providing a simplified
/// interface over the complex Vulkan rendering pipeline. It maintains minimal
/// state and delegates actual rendering work to the Vulkan backend.
pub struct GraphicsEngine {
    /// Backend abstraction for graphics rendering
    /// FIXME: Should support dynamic backend selection at runtime
    backend: Box<dyn RenderBackend>,
    
    /// Currently active camera for 3D rendering
    /// FIXME: Should support multiple cameras for multi-viewport rendering
    current_camera: Option<Camera>,
    
    /// Current lighting environment
    /// FIXME: Should be integrated with scene management system
    current_lighting: Option<LightingEnvironment>,
    
    /// Frame counter for debugging and performance tracking
    /// FIXME: Should be optional debug feature, not core renderer responsibility
    frame_count: u64,
    
    /// Dynamic object pool management system for temporary objects
    /// Manages multiple single-mesh pools for optimal rendering performance
    pool_manager: Option<MeshPoolManager>,
    
}

impl GraphicsEngine {
    /// Create a new renderer from a window handle with configuration
    ///
    /// This is the preferred construction method as it maintains clear ownership
    /// of the window resource and allows applications to configure the renderer.
    ///
    /// # Arguments
    /// * `window_handle` - Mutable reference to window handle for Vulkan surface creation
    /// * `config` - Renderer configuration with application-specific settings
    ///
    /// # Design Notes
    /// This method represents good separation of concerns - it only requires
    /// what's necessary (a window and config) and doesn't depend on engine-specific configs.
    pub fn new_from_window(window_handle: &mut WindowHandle, config: &VulkanRendererConfig) -> RenderResult<Self> {
        log::info!("Initializing Vulkan renderer for '{}'", config.application_name);
        
        // Create Vulkan renderer using backend access
        // IMPLEMENTATION NOTE: This uses internal backend access with type-safe downcasting
        // rather than exposing Vulkan types through the public WindowHandle API
        
        // Safe downcast to Vulkan window using the RenderSurface trait
        let render_surface = window_handle.render_surface();
        let vulkan_window = render_surface.as_any_mut()
            .downcast_mut::<crate::render::vulkan::Window>()
            .expect("Expected Vulkan window backend");
        
        let vulkan_renderer = crate::render::vulkan::VulkanRenderer::new(vulkan_window, config)
            .map_err(|e| RenderError::InitializationFailed(format!("Failed to create Vulkan renderer: {:?}", e)))?;
        
        Ok(Self {
            backend: Box::new(vulkan_renderer),
            current_camera: None,
            current_lighting: None,
            frame_count: 0,
            pool_manager: None, // Will be initialized when first needed
        })
    }
    
    /// Create a new renderer (legacy method for engine integration)
    ///
    /// FIXME: This method breaks library abstraction by coupling to engine-specific
    /// RendererConfig and WindowConfig types. Should be removed once applications
    /// migrate to the cleaner `new_from_window` approach.
    ///
    /// # Architectural Problems
    /// - Depends on engine-specific configuration types
    /// - Mixes window creation with renderer creation responsibilities
    /// - Less flexible than window-handle approach
    pub fn new(_config: &RendererConfig, window_config: &WindowConfig) -> Result<Self, RenderError> {
        log::info!("Initializing Vulkan renderer: {}", window_config.title);
        
        // FIXME: Window creation should be application responsibility, not renderer
        let mut window_handle = WindowHandle::new(
            window_config.width,
            window_config.height,
            &window_config.title,
        );

        Ok(Self::new_from_window(&mut window_handle, &VulkanRendererConfig::default())?)
    }
    
    /// Begin a new frame
    ///
    /// Prepares the renderer for a new frame of rendering. Currently this is
    /// a minimal operation that primarily handles frame counting for debugging.
    ///
    /// FIXME: This method doesn't actually do meaningful setup work. The real
    /// frame setup happens inside draw_mesh_3d(). This should either be removed
    /// or expanded to handle proper per-frame setup like updating uniform buffers.
    pub fn begin_frame(&mut self) {
        // FIXME: Frame setup should be more comprehensive
        // Should handle: UBO updates, command buffer reset, sync object management
        log::trace!("Begin frame {}", self.frame_count);
    }
    
    /// Set the active camera for 3D rendering
    ///
    /// Configures the camera that will be used for subsequent 3D rendering operations.
    /// The camera defines the view and projection transformations for the scene.
    ///
    /// # Arguments
    /// * `camera` - Reference to camera configuration
    ///
    /// # Design Notes
    /// Currently stores a full clone of the camera data. This is inefficient for
    /// frequently changing cameras, but acceptable for current usage patterns.
    ///
    /// FIXME: Consider using camera handles/IDs for better performance and to
    /// support multiple active cameras for multi-viewport rendering.
    pub fn set_camera(&mut self, camera: &Camera) {
        self.current_camera = Some(camera.clone()); // FIXME: Cloning entire camera - inefficient
        log::trace!("Camera set: pos={:?}, target={:?}", camera.position, camera.target);
    }
    
    /// Set the lighting environment for the scene
    ///
    /// Configures the lighting that will be applied to rendered objects.
    /// Currently limited to a single directional light due to push constant constraints.
    ///
    /// # Arguments
    /// * `lighting` - The lighting environment containing lights and ambient settings
    ///
    /// # Current Limitations
    /// - Only supports one directional light (first in the array)
    /// - Other light types are ignored with warnings
    /// - Limited by 160-byte push constant size constraint
    ///
    /// # Future Architecture
    /// This method demonstrates a clear architectural limitation where the high-level
    /// renderer is constrained by low-level implementation details (push constants).
    /// The proper solution is uniform buffer objects (UBOs) for lighting data.
    pub fn set_lighting(&mut self, lighting: &LightingEnvironment) {
        self.current_lighting = Some(lighting.clone()); // FIXME: Another full clone - inefficient
        
        // FIXME: This logic couples high-level renderer to low-level push constant limitations
        // The renderer should abstract this complexity, not expose it to the application
        
        // Update the Vulkan renderer with lighting data
        // LIMITATION: Current push constants only support ONE directional light
        // Multiple lights require expanding push constants (limited to ~160 bytes) or uniform buffers
        if let Some(first_light) = lighting.lights.first() {
            match first_light.light_type {
                LightType::Directional => {
                    // Use helper methods for array conversion
                    self.backend.set_directional_light(
                        first_light.direction_array(),
                        first_light.intensity,
                        first_light.color_array(),
                        lighting.ambient_intensity,
                    );
                },
                _ => {
                    // FIXME: Should support all light types, not just directional
                    log::warn!("Only directional lights supported in push constants. Found: {:?}", first_light.light_type);
                    log::info!("To use multiple lights, need to implement uniform buffer lighting system");
                }
            }
        } else {
            // FIXME: Hardcoded default lighting - should be configurable
            self.backend.set_directional_light(
                [0.0, 1.0, 0.0],  // Default light coming from below (positive Y direction in Y-up world space)
                0.3,              // Mild default intensity
                [1.0, 1.0, 1.0],  // White light
                lighting.ambient_intensity,
            );
            log::trace!("No directional lights found, using default downward light");
        }
        
        // FIXME: These warnings show that the API contract is broken - the renderer
        // accepts lighting environments but can't actually use them fully
        if lighting.lights.len() > 1 {
            log::warn!("LightingEnvironment contains {} lights, but only the first directional light is used", lighting.lights.len());
            log::info!("Multiple light support requires expanding push constants or implementing uniform buffer system");
        }
        
        log::trace!("Lighting set with {} lights", lighting.lights.len());
    }
    
    /// Set multi-light environment using UBO-based lighting
    /// 
    /// This method uses uniform buffer objects instead of push constants for lighting data,
    /// enabling support for multiple lights while maintaining identical visual output.
    pub fn set_multi_light_environment(&mut self, multi_light_env: &lighting::MultiLightEnvironment) {
        self.backend.set_multi_light_environment(multi_light_env);
    }
    
    /// Render a 3D mesh with transformation and material
    ///
    /// Primary rendering method for 3D objects. Draws a mesh using the currently
    /// configured camera and lighting with the specified transformation and material.
    ///
    /// # Arguments
    /// * `mesh` - The mesh data (vertices and indices)
    /// * `model_matrix` - Transformation matrix from object space to world space
    /// * `material` - Material properties (color, textures, shading parameters)
    ///
    /// # Matrix Mathematics
    /// Implements Johannes Unterguggenberger's mathematically correct approach:
    /// Clip = P × X × V × M × Vertex where:
    /// - P = Projection matrix (perspective/orthographic transformation)
    /// - X = Vulkan coordinate transform (Y-up view space → Vulkan conventions)
    /// - V = View matrix (world space → camera/view space)
    /// - M = Model matrix (object space → world space)
    ///
    /// # Architecture Violations
    /// This method violates several library-agnostic design principles:
    /// 1. **Vulkan Coupling**: Direct calls to vulkan_renderer methods
    /// 2. **Matrix Format Coupling**: Manual nalgebra → array conversion
    /// 3. **Error Type Leakage**: Exposes Vulkan-specific error types
    /// 4. **Immediate Mode**: Updates GPU state on every call (inefficient)
    ///
    /// # Performance Issues
    /// - Mesh data updated on every draw call (should be cached)
    /// - Matrix calculations repeated for identical transformations
    /// - Lighting set per mesh instead of per frame
    /// - No batching for objects with similar materials
    ///
    /// FIXME: This method does too much and should be decomposed into:
    /// - update_mesh_data() - GPU resource management
    /// - set_transform_matrices() - Matrix calculations

    // ================================
    // NEW COMMAND RECORDING API (PROTOTYPE)
    // ================================
    
    /// Begin command recording for multiple objects (NEW API)
    /// This replaces the immediate-mode pattern with proper command recording
    pub fn begin_command_recording(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Begin proper render pass for multiple objects
        self.backend.begin_render_pass()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        
        // Set up per-frame uniforms (camera, lighting) once
        self.setup_frame_uniforms()?;
        
        Ok(())
    }
    
    /// Set up per-frame uniforms (camera, lighting) - called once per frame
    fn setup_frame_uniforms(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Set per-frame uniforms first
        if let Some(ref camera) = self.current_camera {
            let view_matrix = camera.get_view_matrix();
            let coord_transform = Mat4::vulkan_coordinate_transform();
            let projection_matrix = camera.get_projection_matrix();

            // Update camera UBO using existing backend method
            let view_projection = projection_matrix * coord_transform * view_matrix;
            self.backend.update_camera_ubo(view_matrix, projection_matrix, view_projection, camera.position);
        }

        // Set lighting environment using existing method
        if let Some(ref lighting) = self.current_lighting {
            let multi_light_env = self.convert_lighting_to_multi_light_environment(lighting);
            self.backend.set_multi_light_environment(&multi_light_env);
        }
        
        Ok(())
    }
    
    /// Bind shared resources for multiple objects (NEW API)
    /// This eliminates the need to bind these resources for each object
    pub fn bind_shared_geometry(&mut self, shared: &SharedRenderingResources) -> Result<(), Box<dyn std::error::Error>> {
        // Bind the shared vertex and index buffers
        // Set the shared mesh data once for all objects
        log::trace!("Binding shared geometry with {} vertices and {} indices", 
                   shared.vertex_count, shared.index_count);
        
        // Call the backend to bind shared resources
        self.backend.bind_shared_resources(shared)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        
        Ok(())
    }
    
    /// Draw a single object using command recording (NEW API)
    /// Records draw command without immediate submission
    pub fn record_object_draw(&mut self, object: &GameObject, shared_resources: &SharedRenderingResources) -> Result<(), Box<dyn std::error::Error>> {
        // Update per-object uniform data  
        let model_matrix = object.get_model_matrix();
        
        // Set per-object uniforms (model matrix, material)
        let model_array: [[f32; 4]; 4] = model_matrix.into();
        self.backend.set_model_matrix(model_array);
        
        let material_color = object.material.get_base_color_array();
        self.backend.set_material_color(material_color);
        
        // Get object's descriptor sets (these contain per-object uniform buffers)
        let object_descriptor_sets = &object.descriptor_sets;
        
        // Record draw using shared resources and object-specific descriptor sets
        self.backend.record_shared_object_draw(shared_resources, object_descriptor_sets)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        
        Ok(())
    }
    
    /// Submit all recorded commands and present (NEW API)
    /// This replaces individual draw_frame() calls with batched submission
    pub fn submit_commands_and_present(&mut self, _window: &mut WindowHandle) -> Result<(), Box<dyn std::error::Error>> {
        // End render pass and submit all recorded draw commands
        self.backend.end_render_pass_and_submit()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
            
        // Update frame counter
        self.frame_count += 1;
        
        Ok(())
    }
    
    /// Create a GameObject with proper GPU resource initialization
    /// This handles the complex VulkanContext access and resource creation
    pub fn create_game_object(&self, position: Vec3, rotation: Vec3, scale: Vec3, material: Material) -> Result<GameObject, Box<dyn std::error::Error>> {
        let mut game_object = GameObject::new(position, rotation, scale, material);
        
        // Access VulkanContext through backend downcasting
        // IMPLEMENTATION NOTE: This temporarily breaks abstraction to access Vulkan resources
        // TODO: Refactor to proper abstraction when GameObject system is fully integrated
        if let Some(vulkan_backend) = self.backend.as_any().downcast_ref::<crate::render::vulkan::renderer::VulkanRenderer>() {
            // Access VulkanContext and descriptor resources through the backend
            let context = vulkan_backend.get_context();
            let descriptor_pool = vulkan_backend.get_descriptor_pool();
            let descriptor_set_layout = vulkan_backend.get_object_descriptor_set_layout();
            let max_frames_in_flight = vulkan_backend.get_max_frames_in_flight();
            
            // Initialize GPU resources for the GameObject
            game_object.create_resources(
                context,
                &descriptor_pool,
                &descriptor_set_layout,
                max_frames_in_flight,
            ).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
            
            log::debug!("Created GameObject with GPU resources initialized");
        } else {
            return Err("Renderer is not using Vulkan backend".into());
        }
        
        Ok(game_object)
    }
    
    /// Create SharedRenderingResources for efficient multi-object rendering
    /// This handles the complex Vulkan resource creation for shared geometry
    pub fn create_shared_rendering_resources(&self, mesh: &Mesh, _materials: &[Material]) -> Result<shared_resources::SharedRenderingResources, Box<dyn std::error::Error>> {
        // Access VulkanContext through backend downcasting
        if let Some(vulkan_backend) = self.backend.as_any().downcast_ref::<crate::render::vulkan::VulkanRenderer>() {
            use ash::vk;
            
            log::info!("Creating SharedRenderingResources with {} vertices and {} indices", 
                      mesh.vertices.len(), mesh.indices.len());
            
            // Get required components from VulkanRenderer
            let context = vulkan_backend.get_context();
            let frame_descriptor_set_layout = vulkan_backend.get_frame_descriptor_set_layout();
            let material_descriptor_set_layout = vulkan_backend.get_object_descriptor_set_layout();
            
            // Create shared vertex buffer
            let vertex_buffer = crate::render::vulkan::buffer::Buffer::new(
                context.raw_device(),
                context.instance().clone(),
                context.physical_device().device,
                (mesh.vertices.len() * std::mem::size_of::<Vertex>()) as vk::DeviceSize,
                vk::BufferUsageFlags::VERTEX_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )?;
            
            // Upload vertex data
            vertex_buffer.write_data(&mesh.vertices)?;
            
            // Create shared index buffer
            let index_buffer = crate::render::vulkan::buffer::Buffer::new(
                context.raw_device(),
                context.instance().clone(),
                context.physical_device().device,
                (mesh.indices.len() * std::mem::size_of::<u32>()) as vk::DeviceSize,
                vk::BufferUsageFlags::INDEX_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )?;
            
            // Upload index data
            index_buffer.write_data(&mesh.indices)?;
            
            // Get pipeline and layouts (these should already exist in the VulkanRenderer)
            let pipeline = vulkan_backend.get_graphics_pipeline();
            let pipeline_layout = vulkan_backend.get_pipeline_layout();
            
            let shared_resources = shared_resources::SharedRenderingResources::new(
                vertex_buffer,
                index_buffer,
                pipeline,
                pipeline_layout,
                material_descriptor_set_layout,
                frame_descriptor_set_layout,
                mesh.indices.len() as u32,
                mesh.vertices.len() as u32,
            );
            
            log::info!("SharedRenderingResources created successfully");
            Ok(shared_resources)
        } else {
            Err("Renderer is not using Vulkan backend".into())
        }
    }
    
    /// Create SharedRenderingResources for instanced dynamic object rendering
    /// This creates resources with an instanced pipeline for efficient batch rendering
    pub fn create_instanced_rendering_resources(&self, mesh: &Mesh, _materials: &[Material]) -> Result<shared_resources::SharedRenderingResources, Box<dyn std::error::Error>> {
        // Access VulkanContext through backend downcasting
        if let Some(vulkan_backend) = self.backend.as_any().downcast_ref::<crate::render::vulkan::VulkanRenderer>() {
            use ash::vk;
            
            log::info!("Creating InstancedRenderingResources with {} vertices and {} indices", 
                mesh.vertices.len(), mesh.indices.len());            // Get required components from VulkanRenderer
            let context = vulkan_backend.get_context();
            let frame_descriptor_set_layout = vulkan_backend.get_frame_descriptor_set_layout();
            let material_descriptor_set_layout = vulkan_backend.get_object_descriptor_set_layout();
            
            // Create shared vertex buffer (same as regular)
            let vertex_buffer = crate::render::vulkan::buffer::Buffer::new(
                context.raw_device(),
                context.instance().clone(),
                context.physical_device().device,
                (mesh.vertices.len() * std::mem::size_of::<Vertex>()) as vk::DeviceSize,
                vk::BufferUsageFlags::VERTEX_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )?;
            
            // Upload vertex data
            vertex_buffer.write_data(&mesh.vertices)?;
            
            // Create shared index buffer (same as regular)
            let index_buffer = crate::render::vulkan::buffer::Buffer::new(
                context.raw_device(),
                context.instance().clone(),
                context.physical_device().device,
                (mesh.indices.len() * std::mem::size_of::<u32>()) as vk::DeviceSize,
                vk::BufferUsageFlags::INDEX_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )?;
            
            // Upload index data
            index_buffer.write_data(&mesh.indices)?;
            
            // Create INSTANCED pipeline (different from regular)
            let (graphics_pipeline, pipeline_layout) = self.create_instanced_pipeline(context, frame_descriptor_set_layout, material_descriptor_set_layout)?;
            
            // Debug log the pipeline handle
            log::info!("Created instanced pipeline: {:?}", graphics_pipeline.handle());
            
            // Build SharedRenderingResources with instanced pipeline
            let shared_resources = shared_resources::SharedRenderingResources::new_with_graphics_pipeline(
                vertex_buffer,
                index_buffer,
                graphics_pipeline,
                pipeline_layout,
                material_descriptor_set_layout,
                frame_descriptor_set_layout,
                mesh.indices.len() as u32,
                mesh.vertices.len() as u32,
            );
            
            log::info!("InstancedRenderingResources created successfully");
            Ok(shared_resources)
        } else {
            Err("Renderer is not using Vulkan backend".into())
        }
    }
    
    /// Create instanced pipeline for dynamic object rendering
    fn create_instanced_pipeline(&self, context: &crate::render::vulkan::VulkanContext, frame_layout: ash::vk::DescriptorSetLayout, material_layout: ash::vk::DescriptorSetLayout) -> Result<(crate::render::vulkan::shader::GraphicsPipeline, ash::vk::PipelineLayout), Box<dyn std::error::Error>> {
        use crate::render::vulkan::{VulkanVertexLayout, shader::ShaderModule};
        use ash::vk;
        
        // Load the same shaders (they'll adapt to instanced input via vertex attributes)
        let vertex_shader = ShaderModule::from_file(
            context.raw_device(),
            "target/shaders/standard_pbr_vert.spv",
        )?;
        
        let fragment_shader = ShaderModule::from_file(
            context.raw_device(),
            "target/shaders/standard_pbr_frag.spv",
        )?;
        
        // Create INSTANCED vertex input state
        let (binding_descriptions, attribute_descriptions) = VulkanVertexLayout::get_instanced_input_state();
        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(&binding_descriptions)
            .vertex_attribute_descriptions(&attribute_descriptions)
            .build();
        
        // Create descriptor set layout array for pipeline creation
        let descriptor_set_layouts = [frame_layout, material_layout];
        
        // Get render pass from VulkanRenderer (need to access it through backend)
        if let Some(vulkan_backend) = self.backend.as_any().downcast_ref::<crate::render::vulkan::VulkanRenderer>() {
            let render_pass = vulkan_backend.get_render_pass();
            
            // Create instanced graphics pipeline (it will create its own pipeline layout internally)
            let pipeline = crate::render::vulkan::shader::GraphicsPipeline::new_with_descriptor_layouts(
                context.raw_device(),
                render_pass,
                &vertex_shader,
                &fragment_shader,
                vertex_input_info,
                &descriptor_set_layouts,
            )?;
            
            let pipeline_handle = pipeline.handle();
            
            if pipeline_handle == vk::Pipeline::null() {
                return Err("Pipeline creation failed - returned null handle".into());
            }
            
            // Get the pipeline layout that was actually used to create the pipeline
            let pipeline_layout = pipeline.layout();
            
            Ok((pipeline, pipeline_layout))
        } else {
            Err("Backend is not Vulkan".into())
        }
    }
    
    /// Helper method to convert lighting environment (temporary until lighting refactor)
    fn convert_lighting_to_multi_light_environment(&self, _lighting: &LightingEnvironment) -> lighting::MultiLightEnvironment {
        // Convert from the old LightingEnvironment to the new MultiLightEnvironment
        // For now, create a simple environment with ambient lighting
        // TODO: Implement proper conversion when lighting system is refactored
        lighting::MultiLightEnvironment::new()
    }

    // ================================
    // DYNAMIC OBJECT RENDERING API
    // ================================

    /// Initialize the dynamic object pool system
    ///
    /// Creates the MeshPoolManager and sets up initial pools for common mesh types.
    /// This must be called before spawning any dynamic objects.
    ///
    /// # Arguments
    /// * `max_objects_per_pool` - Maximum objects per individual mesh pool
    ///
    /// # Returns
    /// Result indicating successful initialization
    ///
    /// # Examples
    /// ```rust
    /// // Initialize with 100 objects per mesh type
    /// graphics_engine.initialize_dynamic_system(100)?;
    ///
    /// // Now you can spawn objects
    /// let handle = graphics_engine.spawn_dynamic_object(
    ///     MeshType::Teapot,           // specify mesh type
    ///     Vec3::new(0.0, 0.0, 0.0),   // position
    ///     Vec3::zeros(),              // rotation
    ///     Vec3::new(1.0, 1.0, 1.0),   // scale
    ///     MaterialProperties::default(), // material
    ///     5.0                         // lifetime in seconds
    /// )?;
    /// ```
    pub fn initialize_dynamic_system(&mut self, max_objects_per_pool: usize) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Initializing mesh pool system with {} objects per pool", max_objects_per_pool);
        
        // Access VulkanContext through backend downcasting for pool system initialization
        if let Some(vulkan_backend) = self.backend.as_any().downcast_ref::<crate::render::vulkan::VulkanRenderer>() {
            let context = vulkan_backend.get_context();
            
            // Create MeshPoolManager
            let mut pool_manager = MeshPoolManager::new();
            
            // TODO: In Phase 3, create pools for each mesh type with proper SharedRenderingResources
            // For now, we create empty pools that will be populated when SharedRenderingResources integration is complete
            
            // Initialize the pool manager with Vulkan resources
            pool_manager.initialize_pools(context)?;
            self.pool_manager = Some(pool_manager);
            
            // Initialize backend's dynamic rendering system
            self.backend.initialize_dynamic_rendering(max_objects_per_pool)?;
            
            log::info!("Mesh pool system initialized successfully");
            Ok(())
        } else {
            Err("Renderer is not using Vulkan backend".into())
        }
    }

    /// Create a mesh pool for a specific mesh type (MeshPoolManager system)
    ///
    /// This method creates a pool for a specific mesh type using the MeshPoolManager.
    /// Must be called after initialize_dynamic_system and before spawning objects of this type.
    ///
    /// # Arguments
    /// * `mesh_type` - The type of mesh to create a pool for (Teapot, Sphere, Cube)
    /// * `mesh` - The mesh data to create SharedRenderingResources from
    /// * `materials` - Array of materials for this mesh type
    /// * `max_objects` - Maximum number of objects this pool can hold
    ///
    /// # Returns
    /// Result indicating successful pool creation or error
    pub fn create_mesh_pool(
        &mut self,
        mesh_type: crate::render::dynamic::MeshType,
        mesh: &Mesh,
        materials: &[Material],
        max_objects: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Creating mesh pool for {:?} with capacity {}", mesh_type, max_objects);
        
        // Create SharedRenderingResources for this mesh using instanced rendering
        let shared_resources = self.create_instanced_rendering_resources(mesh, materials)?;
        
        // Create the pool in the MeshPoolManager
        if let Some(ref mut pool_manager) = self.pool_manager {
            // Get VulkanContext from backend
            if let Some(vulkan_backend) = self.backend.as_any().downcast_ref::<crate::render::vulkan::VulkanRenderer>() {
                let context = vulkan_backend.get_context();
                pool_manager.create_pool(mesh_type, shared_resources, max_objects, context)?;
                log::info!("Successfully created pool for {:?}", mesh_type);
                Ok(())
            } else {
                Err("Backend is not a VulkanRenderer - cannot get VulkanContext".into())
            }
        } else {
            Err("Dynamic system not initialized. Call initialize_dynamic_system first.".into())
        }
    }
    
    /// Register a mesh type for multi-mesh rendering (Phase 0)
    ///
    /// This method registers a mesh type with its associated SharedRenderingResources
    /// in the MeshRegistry. This must be called for each mesh type that will be used
    /// with dynamic objects before spawning objects of that type.
    ///
    /// # Arguments
    /// * `mesh_type` - The type of mesh to register (Teapot, Sphere, Cube)
    /// * `mesh` - The mesh data to create SharedRenderingResources from
    /// * `materials` - Array of materials for this mesh type
    ///
    /// # Returns
    /// Result indicating successful registration or error
    ///
    /// # Example
    /// ```rust
    /// Spawn a dynamic object with specified parameters
    ///
    /// Creates a new dynamic object using the pooled resource system. Objects are
    /// automatically managed by the pool and will be despawned when their lifetime expires.
    /// This is the high-level API for creating temporary objects.
    ///
    /// # Arguments
    /// * `position` - World space position of the object
    /// * `rotation` - Rotation angles in radians (Euler angles: X, Y, Z)
    /// * `scale` - Scale factors for each axis
    /// * `material` - Material properties for rendering
    /// * `lifetime` - Object lifetime in seconds (0.0 for infinite)
    ///
    /// # Returns
    /// Handle to the spawned object for safe access, or SpawnerError on failure
    ///
    /// # Error Conditions
    /// - Pool exhaustion: No more objects available in the pool
    /// - Invalid parameters: Position/scale contains NaN or infinite values
    /// - System not initialized: initialize_dynamic_system() not called
    ///
    /// # Example Usage
    /// ```rust
    /// use crate::render::dynamic::MeshType;
    /// 
    /// // Spawn a temporary teapot that lasts 3 seconds
    /// let handle = graphics_engine.spawn_dynamic_object(
    ///     MeshType::Teapot,               // mesh type
    ///     Vec3::new(5.0, 2.0, -3.0),     // position
    ///     Vec3::new(0.0, 45.0_f32.to_radians(), 0.0), // Y rotation
    /// Spawn a dynamic object with specified parameters
    ///
    /// Creates a new dynamic object using the pooled resource system. Objects are
    /// automatically managed by the pool and will be despawned when their lifetime expires.
    /// This is the high-level API for creating temporary objects.
    ///
    /// # Arguments
    /// * `position` - World position for the object
    /// * `rotation` - Rotation in radians (Euler angles)
    /// * `scale` - Scale factors for each axis
    /// * `material` - Material properties for rendering
    /// * `lifetime` - How long the object should exist (seconds)
    ///
    /// # Returns
    /// Handle to the spawned object for tracking/despawning
    ///
    /// # Example
    /// ```rust,no_run
    /// # use crate::render::{GraphicsEngine, MaterialProperties};
    /// # use crate::foundation::math::Vec3;
    /// # let mut graphics_engine = GraphicsEngine::new(window_config)?;
    /// // Spawn a dynamic object (uses default mesh type)
    /// let handle = graphics_engine.spawn_dynamic_object(
    ///     Vec3::new(0.0, 5.0, -10.0),    // position
    ///     Vec3::new(0.0, 1.57, 0.0),     // rotation
    ///     Vec3::new(2.0, 1.0, 2.0),      // scale
    ///     MaterialProperties {
    ///         base_color: [1.0, 0.5, 0.0, 1.0], // Orange
    ///         metallic: 0.2,
    ///         roughness: 0.8,
    ///         emission: [0.0, 0.0, 0.0, 0.0],
    ///     },
    ///     3.0  // 3 second lifetime
    /// )?;
    /// ```
    pub fn spawn_dynamic_object(
        &mut self,
        mesh_type: MeshType,
        position: Vec3,
        rotation: Vec3,
        scale: Vec3,
        material: MaterialProperties,
    ) -> Result<DynamicObjectHandle, SpawnerError> {
        // Ensure pool system is initialized
        let pool_manager = self.pool_manager.as_mut()
            .ok_or(SpawnerError::InvalidParameters {
                reason: "Pool system not initialized - call initialize_dynamic_system() first".to_string()
            })?;
        
        // Convert MaterialProperties to Material
        let material_instance = Material::standard_pbr(crate::render::material::StandardMaterialParams {
            base_color: Vec3::new(
                material.base_color[0],
                material.base_color[1],
                material.base_color[2]
            ),
            alpha: material.base_color[3],
            metallic: material.metallic,
            roughness: material.roughness,
            emission: Vec3::new(
                material.emission[0],
                material.emission[1],
                material.emission[2]
            ),
            ..Default::default()
        });
        
        // Spawn the object in the appropriate mesh pool
        let handle = pool_manager.spawn_object(mesh_type, position, rotation, scale, material_instance)
            .map_err(|e| SpawnerError::InvalidParameters {
                reason: format!("Pool manager spawn failed: {}", e)
            })?;
        
        log::debug!("Spawned {} object at position {:?}", mesh_type, position);
        Ok(handle)
    }

    /// Despawn a specific dynamic object by handle
    ///
    /// Immediately removes a dynamic object from the rendering system and returns
    /// its resources to the pool. This allows manual cleanup before the object's
    /// lifetime expires.
    ///
    /// # Arguments
    /// * `handle` - Handle to the object to despawn
    ///
    /// # Returns
    /// Result indicating success or despawn failure
    ///
    /// # Error Conditions
    /// - Invalid handle: Handle is not valid or object already despawned
    /// - System not initialized: Dynamic system not properly set up
    ///
    /// # Example
    /// ```rust
    /// let handle = graphics_engine.spawn_dynamic_object(/* ... */)?;
    /// 
    /// // Later, manually despawn the object
    /// graphics_engine.despawn_dynamic_object(MeshType::Teapot, handle)?;
    /// ```
    pub fn despawn_dynamic_object(&mut self, mesh_type: MeshType, handle: DynamicObjectHandle) -> Result<(), SpawnerError> {
        let pool_manager = self.pool_manager.as_mut()
            .ok_or(SpawnerError::InvalidParameters {
                reason: "Pool system not initialized".to_string()
            })?;
        
        pool_manager.despawn_object(mesh_type, handle)
            .map_err(|e| SpawnerError::InvalidParameters {
                reason: format!("Pool manager despawn failed: {}", e)
            })?;
        log::debug!("Despawned {} object with handle generation {}", mesh_type, handle.generation);
        Ok(())
    }

    /// Update the transform of a dynamic object
    ///
    /// Updates the position, rotation, and scale of an existing dynamic object.
    /// This allows objects to move, rotate, and scale during their lifetime.
    ///
    /// # Arguments
    /// * `mesh_type` - The mesh type pool this object belongs to
    /// * `handle` - Handle to the dynamic object to update
    /// * `position` - New world position
    /// * `rotation` - New rotation in radians (Euler angles)
    /// * `scale` - New scale factors
    ///
    /// # Errors
    /// Returns `SpawnerError` if:
    /// - Pool system not initialized
    /// - Handle is invalid or object has been despawned
    /// - Transform update fails
    /// 
    /// # Note
    /// This is a temporary API - in Phase 3 this will be improved
    pub fn update_dynamic_object_transform(
        &mut self,
        mesh_type: MeshType,
        handle: DynamicObjectHandle,
        position: Vec3,
        rotation: Vec3,
        scale: Vec3,
    ) -> Result<(), SpawnerError> {
        if let Some(ref mut pool_manager) = self.pool_manager {
            pool_manager.update_object_transform(mesh_type, handle, position, rotation, scale)
                .map_err(|e| SpawnerError::InvalidParameters {
                    reason: format!("Failed to update object transform: {}", e),
                })
        } else {
            Err(SpawnerError::InvalidParameters {
                reason: "Pool manager not initialized".to_string(),
            })
        }
    }

    /// Update only the position of a dynamic object, preserving rotation and scale
    pub fn update_dynamic_object_position(
        &mut self,
        mesh_type: MeshType,
        handle: DynamicObjectHandle,
        position: Vec3,
    ) -> Result<(), SpawnerError> {
        if let Some(ref mut pool_manager) = self.pool_manager {
            pool_manager.update_object_position(mesh_type, handle, position)
                .map_err(|e| SpawnerError::InvalidParameters {
                    reason: format!("Failed to update object position: {}", e),
                })
        } else {
            Err(SpawnerError::InvalidParameters {
                reason: "Pool manager not initialized".to_string(),
            })
        }
    }

    /// Begin dynamic frame rendering setup
    ///
    /// Prepares the dynamic rendering system for a new frame. This should be called
    /// once per frame before any dynamic object operations. Handles automatic
    /// lifecycle updates and resource cleanup.
    ///
    /// # Arguments
    /// * `delta_time` - Time elapsed since last frame in seconds
    ///
    /// # Returns
    /// Result indicating successful frame setup
    ///  
    /// # Frame Workflow
    /// The proper dynamic rendering workflow is:
    /// 1. `begin_dynamic_frame(delta_time)` - Setup and lifecycle updates
    /// 2. Spawn/despawn dynamic objects as needed
    /// 3. `record_dynamic_draws()` - Record drawing commands
    /// 4. `end_dynamic_frame()` - Submit and present
    ///
    /// # Example
    /// ```rust
    /// // At start of render loop
    /// graphics_engine.begin_dynamic_frame(delta_time)?;
    /// 
    /// // Spawn objects as needed
    /// if should_spawn_asteroid {
    ///     graphics_engine.spawn_dynamic_object(/* ... */)?;
    /// }
    /// 
    /// // Record and submit rendering
    /// graphics_engine.record_dynamic_draws()?;
    /// graphics_engine.end_dynamic_frame(window)?;
    /// ```
    pub fn begin_dynamic_frame(&mut self, delta_time: f32) -> Result<(), Box<dyn std::error::Error>> {
        // Dynamic frame started - process any pending cleanup from manual despawns
        log::trace!("Dynamic frame started");
        
        // Update pool manager to process cleanup of manually despawned objects
        if let Some(pool_manager) = &mut self.pool_manager {
            pool_manager.update_all_pools(delta_time);
        }
        
        // Begin command recording for the frame (same as static objects)
        self.begin_command_recording()?;
        
        Ok(())
    }

    /// Record dynamic object draw commands
    ///
    /// Records all active dynamic objects into the command buffer using instanced
    /// rendering for optimal performance. This batches all dynamic objects by
    /// mesh type into minimal draw calls.
    ///
    /// # Returns
    /// Result indicating successful command recording
    ///
    /// # Performance Notes
    /// This method uses instanced rendering to draw all active dynamic objects
    /// in as few draw calls as possible (one per mesh type). It's much more efficient 
    /// than individual draw calls.
    ///
    /// # Prerequisites
    /// - Pool system must be initialized
    /// - begin_dynamic_frame() must be called first
    /// - Camera and lighting should be set up
    pub fn record_dynamic_draws(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref mut pool_manager) = self.pool_manager {
            let stats = pool_manager.get_stats();
            
            if stats.total_active_objects > 0 {
                log::trace!("Recording draws for {} active dynamic objects across {} pools", 
                           stats.total_active_objects, stats.render_batches_per_frame);
                
                // Use MeshPoolManager's proper render method to avoid state corruption between mesh types
                pool_manager.render_all_pools_via_backend(&mut *self.backend)
                    .map_err(|e| format!("Failed to render dynamic object pools: {}", e))?;
                
                log::debug!("Dynamic object rendering completed: {} total objects across {} mesh types", 
                           stats.total_active_objects, stats.render_batches_per_frame);
            } else {
                log::trace!("No active dynamic objects to render");
            }
        }
        
        Ok(())
    }

    /// End dynamic frame and submit commands
    ///
    /// Finalizes the dynamic rendering frame by submitting all recorded commands
    /// and presenting the frame. This completes the dynamic rendering workflow.
    ///
    /// # Arguments
    /// * `window` - Window handle for presentation
    ///
    /// # Returns
    /// Result indicating successful frame completion
    ///
    /// # Frame Completion
    /// This method handles:
    /// - Command buffer submission
    /// - Frame presentation  
    /// - Frame counter increment
    /// - Resource state updates
    pub fn end_dynamic_frame(&mut self, window: &mut WindowHandle) -> Result<(), Box<dyn std::error::Error>> {
        // Submit commands and present (same as static objects)
        self.submit_commands_and_present(window)?;
        
        log::trace!("Dynamic frame {} completed", self.frame_count);
        Ok(())
    }

    /// Get dynamic system statistics
    ///
    /// Returns statistics about the dynamic object system including pool utilization,
    /// active object count, and performance metrics. Useful for debugging and
    /// performance monitoring.
    ///
    /// # Returns
    /// Option containing spawner statistics, or None if system not initialized
    ///
    /// # Statistics Include
    /// - Active object count
    /// - Pool utilization percentage
    /// - Peak usage statistics
    /// - Automatic vs manual despawn counts
    ///
    /// # Example
    /// ```rust
    /// if let Some(stats) = graphics_engine.get_dynamic_stats() {
    ///     println!("Dynamic objects: {}/{} ({}% utilization)", 
    ///              stats.active_objects, 
    ///              stats.pool_capacity,
    ///              (stats.utilization_percentage * 100.0) as u32);
    /// }
    /// ```
    pub fn get_dynamic_stats(&self) -> Option<crate::render::dynamic::PoolManagerStats> {
        self.pool_manager.as_ref().map(|manager| manager.get_stats())
    }

    /// Initialize instanced rendering system for efficient batch rendering
    ///
    /// Sets up the instance renderer for efficient batch rendering of multiple objects
    /// with the same mesh using Vulkan instanced rendering (vkCmdDrawInstanced).
    /// This provides significant performance improvements over individual draw calls.
    ///
    /// # Arguments
    /// * `max_instances` - Maximum number of instances that can be rendered in a single batch
    ///
    /// # Returns
    /// Result indicating successful initialization or error
    ///
    /// # Example
    /// ```rust
    /// // Initialize for up to 100 instances per batch
    /// graphics_engine.initialize_instance_renderer(100)?;
    /// ```
    pub fn initialize_instance_renderer(&mut self, max_instances: usize) -> Result<(), Box<dyn std::error::Error>> {
        // Use the backend trait method for initialization
        self.backend.initialize_instance_renderer(max_instances)?;
        log::info!("Initialized instance renderer with max {} instances", max_instances);
        Ok(())
    }

    // ================================
    // LEGACY RENDERING API (TO BE DEPRECATED)
    // ================================
    /// - apply_material() - Material state setup  
    /// - execute_draw_call() - Actual rendering
    pub fn draw_mesh_3d(&mut self, mesh: &Mesh, model_matrix: &Mat4, material: &Material) -> Result<(), Box<dyn std::error::Error>> {
        
        // ARCHITECTURE ISSUE: Direct Vulkan API calls violate library-agnostic principle
        // A proper abstraction would hide the backend implementation entirely
        
        // Update mesh data - PERFORMANCE ISSUE: This happens on every draw call
        // Meshes should be uploaded once and referenced by handle for efficiency
        // FIXME: Implement mesh caching/handle system to avoid redundant uploads
        self.backend.update_mesh(&mesh.vertices, &mesh.indices)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        
        // Set material properties
        // FIXME: Material should be an object that can be applied as a unit,
        // not individual property calls that can get out of sync
        let material_color = material.get_base_color_array();
        self.backend.set_material_color(material_color);
        
        // ARCHITECTURAL DEPENDENCY: Requires camera to be set beforehand
        // This creates a fragile runtime dependency that could be compile-time checked
        if let Some(ref camera) = self.current_camera {
            
            // MATRIX MATHEMATICS: Following Johannes Unterguggenberger's academic approach
            // This is the one aspect that's architecturally sound and mathematically correct
            
            // Camera transformation matrices (now handled by UBOs)
            // 1. View matrix: World space → Camera/view space (Y-up, right-handed)
            let view_matrix = camera.get_view_matrix();
            
            // 2. Vulkan coordinate transform: Standard view space → Vulkan NDC conventions
            let coord_transform = Mat4::vulkan_coordinate_transform();
            
            // 3. Projection matrix: Camera frustum → normalized device coordinates
            let projection_matrix = camera.get_projection_matrix();
            
            // 4. Complete view-projection transformation for UBO
            let view_projection = projection_matrix * coord_transform * view_matrix;
            
            // Update camera UBO with current frame data
            self.backend.update_camera_ubo(view_matrix, projection_matrix, view_projection, camera.position);
            
            // FIXME: Duplicate conversion logic - this pattern is repeated for every matrix
            // Should have a generic to_vulkan_array() extension method for Mat4<f32>
            let model_array = [
                [model_matrix[(0, 0)], model_matrix[(1, 0)], model_matrix[(2, 0)], model_matrix[(3, 0)]],
                [model_matrix[(0, 1)], model_matrix[(1, 1)], model_matrix[(2, 1)], model_matrix[(3, 1)]],
                [model_matrix[(0, 2)], model_matrix[(1, 2)], model_matrix[(2, 2)], model_matrix[(3, 2)]],
                [model_matrix[(0, 3)], model_matrix[(1, 3)], model_matrix[(2, 3)], model_matrix[(3, 3)]],
            ];

            // UBO-based rendering: Only set model matrix in push constants
            self.backend.set_model_matrix(model_array);
            
            // ARCHITECTURAL ANTI-PATTERN: Setting lighting data per mesh draw
            // Lighting should be set once per frame, not once per object
            // This creates unnecessary GPU state changes and CPU overhead
            if let Some(ref lighting) = self.current_lighting {
                if let Some(first_light) = lighting.lights.first() {
                    match first_light.light_type {
                        LightType::Directional => {
                            // Keep light direction in world space (don't transform it)
                            // The shader will handle lighting calculations in world space
                            
                            // PERFORMANCE ISSUE: This call happens for every mesh, not once per frame
                            // FIXME: Move lighting setup to begin_frame() or a dedicated set_scene_lighting()
                            self.backend.set_directional_light(
                                first_light.direction_array(),
                                first_light.intensity,
                                first_light.color_array(),
                                lighting.ambient_intensity,
                            );
                            log::trace!("Set world-space directional light: dir={:?}, intensity={}", 
                                       first_light.direction_array(), first_light.intensity);
                        },
                        _ => {
                            // API CONTRACT VIOLATION: We accept all light types but only support one
                            log::warn!("Only directional lights are currently supported in push constants");
                        }
                    }
                }
            }
        } else {
            // RUNTIME ERROR: Camera dependency creates fragile rendering state
            // FIXME: This should be a compile-time requirement, not runtime failure
            log::error!("Cannot render mesh without camera - call set_camera() first");
            return Err("No camera set for rendering".into());
        }
        
        // Execute the actual draw call
        // IMPROVED ERROR HANDLING: Backend abstraction now properly handles errors
        match self.backend.draw_frame() {
            Ok(()) => {
                // Success - frame has been submitted and will be presented
                log::trace!("Successfully drew mesh with {} vertices", mesh.vertices.len());
            },
            Err(RenderError::BackendError(ref err_msg)) if err_msg.contains("ERROR_OUT_OF_DATE_KHR") => {
                // Backend reports swapchain out of date - handle gracefully
                log::warn!("Swapchain out of date during render - requesting recreation");
                // FIXME: Automatic swapchain recreation should happen transparently
                return Ok(()); // Skip this frame to avoid crashes
            }
            // Backend abstraction now provides clean error handling
            Err(e) => {
                log::error!("Backend rendering error: {:?}", e);
                return Err(Box::new(e) as Box<dyn std::error::Error>);
            }
        }
        
        Ok(())
    }
    
    /// End the current frame and present to screen
    ///
    /// Finalizes the rendering of the current frame. Currently this is mostly
    /// a formality since the actual frame presentation happens within draw_mesh_3d().
    ///
    /// # Arguments
    /// * `_window_handle` - Window handle (currently unused)
    ///
    /// # Architecture Issues
    /// This method reveals a fundamental design problem with the frame lifecycle.
    /// The actual presentation happens in draw_mesh_3d() → draw_frame(), making
    /// this method mostly ceremonial and the frame boundaries poorly defined.
    ///
    /// # Proper Frame Lifecycle Should Be:
    /// 1. `begin_frame()` - Reset command buffers, start render pass
    /// 2. `draw_*()` calls - Record drawing commands (no presentation)
    /// 3. `end_frame()` - End render pass, submit commands, present
    ///
    /// FIXME: Currently the lifecycle is broken because draw_mesh_3d() does everything.
    /// This makes it impossible to batch multiple draw calls into a single frame.
    pub fn end_frame(&mut self, _window_handle: &mut WindowHandle) -> Result<(), Box<dyn std::error::Error>> {
        log::trace!("End frame {}", self.frame_count);
        self.frame_count += 1;
        
        // FIXME: Frame is already presented by draw_mesh_3d() calls
        // This reveals the broken frame lifecycle architecture - we just increment a counter
        Ok(())
    }
    
    /// Recreate the Vulkan swapchain for window resizing
    ///
    /// Handles window resize events by recreating the Vulkan swapchain with new dimensions.
    /// This is necessary when the window size changes to maintain proper rendering.
    ///
    /// # Arguments
    /// * `window_handle` - Mutable reference to window handle for accessing Vulkan surface
    ///
    /// # Vulkan Implementation Details
    /// This method directly accesses Vulkan-specific window data, violating library-agnostic
    /// design principles. A proper abstraction would hide swapchain management entirely.
    ///
    /// FIXME: Swapchain recreation should be automatic and transparent to applications.
    /// The renderer should detect out-of-date swapchains and recreate them without
    /// requiring explicit application calls.
    pub fn recreate_swapchain(&mut self, window_handle: &mut WindowHandle) {
        log::info!("Recreating swapchain for window resize");
        
        // INTERNAL BACKEND ACCESS: Use safe downcasting via RenderSurface trait
        // Backend now handles swapchain recreation internally
        
        // FIXME: Error handling should be more robust
        // Failed swapchain recreation should either retry or gracefully degrade
        if let Err(e) = self.backend.recreate_swapchain(window_handle) {
            log::error!("Failed to recreate swapchain: {:?}", e);
            // FIXME: What should we do if swapchain recreation fails?
            // Current behavior is to continue with broken state
        } else {
            log::info!("Successfully recreated swapchain");
        }
    }
    
    /// Get the current swapchain dimensions
    ///
    /// Returns the current swapchain extent in pixels. This is primarily used
    /// for aspect ratio calculations and viewport configuration.
    ///
    /// # Returns
    /// Tuple of (width, height) in pixels
    ///
    /// # Architecture Notes
    /// This method exposes Vulkan swapchain implementation details to the application.
    /// A better design would provide generic "get_render_area()" or "get_aspect_ratio()"
    /// methods that don't reveal the underlying graphics API.
    ///
    /// FIXME: This should return a generic RenderArea struct, not raw dimensions
    pub fn get_swapchain_extent(&self) -> (u32, u32) {
        // ABSTRACTION LEAK: Exposing Vulkan swapchain details
        self.backend.get_swapchain_extent()
    }
    
    /// Legacy ECS rendering method
    ///
    /// Provides compatibility with the old ECS-based rendering system.
    /// Currently does minimal work and is primarily used for engine integration.
    ///
    /// # Returns
    /// Result indicating success or rendering error
    ///
    /// # Architecture Notes
    /// This method represents the old architecture where the renderer was tightly
    /// coupled to the ECS system. The new architecture separates rendering from
    /// ECS to improve modularity and testability.
    ///
    /// FIXME: This legacy method should eventually be removed once all applications
    /// migrate to the new explicit draw_mesh_3d() API.
    pub fn render(&mut self) -> Result<(), RenderError> {
        // Legacy rendering path - minimal implementation for backward compatibility
        log::trace!("Legacy render call - frame {}", self.frame_count);
        self.frame_count += 1;
        Ok(())
    }
    
    /// Resize the renderer to new dimensions
    ///
    /// Handles renderer resize events. Currently not fully implemented.
    ///
    /// # Arguments  
    /// * `_width` - New width in pixels (currently unused)
    /// * `_height` - New height in pixels (currently unused)
    ///
    /// # Returns
    /// Result indicating success or rendering error
    ///
    /// # Implementation Status
    /// This method is a placeholder and doesn't perform actual resize operations.
    /// Proper implementation should update projection matrices and viewport settings.
    ///
    /// FIXME: Implement actual resize logic:
    /// 1. Update projection matrix aspect ratios
    /// 2. Recreate swapchain if necessary  
    /// 3. Update viewport and scissor rectangles
    /// 4. Notify cameras of aspect ratio changes
    pub fn resize(&mut self, _width: u32, _height: u32) -> Result<(), RenderError> {
        // TODO: Implement resize logic
        log::debug!("Renderer resize requested: {}x{} (not yet implemented)", _width, _height);
        Ok(())
    }
}

/// 3D Camera for perspective and orthographic projections
///
/// Represents a camera in 3D space with position, orientation, and projection parameters.
/// Supports both perspective and orthographic projections with proper matrix generation
/// following Johannes Unterguggenberger's mathematical approach.
///
/// # Coordinate System
/// Uses standard right-handed Y-up coordinate system in view space:
/// - X+ = Right
/// - Y+ = Up  
/// - Z+ = Forward (towards viewer, opposite of OpenGL convention)
///
/// The Vulkan coordinate transformation is applied separately to convert to Vulkan's
/// Y-down NDC convention while preserving standard view space mathematics.
///
/// # Design Principles
/// - Library-agnostic: No direct Vulkan dependencies in camera math
/// - Immutable operation: Methods don't modify camera state unexpectedly
/// - Mathematical correctness: Follows established computer graphics conventions
/// - Flexible API: Supports various camera creation and manipulation patterns
///
/// # Performance Notes
/// Matrix calculations are performed on-demand rather than cached. For performance-
/// critical applications with static cameras, consider caching the computed matrices.
#[derive(Debug, Clone)]
pub struct Camera {
    /// Camera position in world space
    pub position: Vec3,
    
    /// Point the camera is looking at in world space
    pub target: Vec3,
    
    /// Up vector for camera orientation (typically [0, 1, 0])
    pub up: Vec3,
    
    /// Field of view angle in radians (for perspective projection)
    pub fov: f32,
    
    /// Aspect ratio (width / height) for projection calculations
    pub aspect: f32,
    
    /// Distance to near clipping plane
    pub near: f32,
    
    /// Distance to far clipping plane  
    pub far: f32,
}

impl Camera {
    /// Create a new perspective camera with standard Y-up orientation
    ///
    /// Creates a perspective camera suitable for 3D rendering with the specified
    /// field of view, aspect ratio, and clipping plane distances.
    ///
    /// # Arguments
    /// * `position` - Camera position in world space
    /// * `fov_degrees` - Field of view angle in degrees (converted to radians internally)
    /// * `aspect` - Aspect ratio (width / height) of the viewport
    /// * `near` - Distance to near clipping plane (must be > 0)
    /// * `far` - Distance to far clipping plane (must be > near)
    ///
    /// # Returns
    /// New Camera instance configured for perspective projection
    ///
    /// # Example
    /// ```rust
    /// let camera = Camera::perspective(
    ///     Vec3::new(0.0, 2.0, 5.0),  // Position 5 units back, 2 up
    ///     75.0,                       // 75-degree field of view
    ///     16.0 / 9.0,                // Widescreen aspect ratio
    ///     0.1,                        // Near plane at 10cm
    ///     100.0                       // Far plane at 100 meters
    /// );
    /// ```
    ///
    /// # Design Notes
    /// The default target is origin [0,0,0] and up vector is +Y [0,1,0], providing
    /// a sensible starting configuration that can be customized after creation.
    pub fn perspective(position: Vec3, fov_degrees: f32, aspect: f32, near: f32, far: f32) -> Self {
        Self {
            position,
            target: Vec3::zeros(), // Default to looking at origin
            up: Vec3::new(0.0, 1.0, 0.0), // Standard Y-up orientation
            fov: utils::deg_to_rad(fov_degrees), // Convert degrees to radians for internal use
            aspect,
            near,
            far,
        }
    }
    
    /// Update camera position in world space
    ///
    /// Sets the camera position to a new location while preserving the current
    /// target and orientation.
    ///
    /// # Arguments
    /// * `position` - New camera position in world coordinates
    ///
    /// # Performance Notes
    /// This method is lightweight and doesn't trigger any matrix recalculations
    /// until the next call to get_view_matrix() or get_projection_matrix().
    pub fn set_position(&mut self, position: Vec3) {
        self.position = position;
        log::trace!("Camera position updated to: {:?}", position);
    }
    
    /// Update camera target (look-at point)
    ///
    /// Changes where the camera is pointing without moving the camera position.
    ///
    /// # Arguments
    /// * `target` - Point in world space to look at
    pub fn set_target(&mut self, target: Vec3) {
        self.target = target;
        log::trace!("Camera target updated to: {:?}", target);
    }
    
    /// Configure camera to look at a specific point with custom up vector
    ///
    /// Simultaneously sets the target point and up vector for camera orientation.
    /// This is useful for creating specific camera orientations or for orbit-style
    /// camera controls.
    ///
    /// # Arguments
    /// * `target` - Point in world space to look at
    /// * `up` - Up vector for camera orientation (should be normalized)
    ///
    /// # Mathematical Notes
    /// The up vector doesn't need to be perpendicular to the view direction.
    /// The view matrix calculation will automatically orthonormalize the vectors
    /// using the Gram-Schmidt process.
    pub fn look_at(&mut self, target: Vec3, up: Vec3) {
        self.target = target;
        self.up = up;
        log::trace!("Camera look_at updated - target: {:?}, up: {:?}", target, up);
    }
    
    /// Update camera aspect ratio for viewport changes
    ///
    /// Updates the aspect ratio used for projection matrix calculations.
    /// This is typically called when the window/viewport is resized.
    ///
    /// # Arguments
    /// * `aspect` - New aspect ratio (width / height)
    ///
    /// # Automatic Change Detection
    /// Only logs aspect ratio changes when the difference is significant
    /// (> 0.01) to reduce log noise during window resize events.
    pub fn set_aspect_ratio(&mut self, aspect: f32) {
        // Use a larger threshold to prevent spam during window resize
        if (self.aspect - aspect).abs() > 0.01 {
            log::info!("Camera aspect ratio changed: {:.3} → {:.3}", self.aspect, aspect);
            self.aspect = aspect;
        } else {
            // Still update the aspect ratio, just don't log it
            self.aspect = aspect;
        }
    }
    
    /// Generate view matrix for world-to-camera space transformation
    ///
    /// Creates the view matrix that transforms vertices from world space to
    /// camera space using the camera's position, target, and up vector.
    ///
    /// # Returns
    /// 4x4 view transformation matrix
    ///
    /// # Mathematical Implementation
    /// Uses the standard look-at matrix calculation following right-handed
    /// coordinate system conventions. The transformation:
    /// 1. Translates world by -camera_position
    /// 2. Rotates to align world axes with camera axes
    ///
    /// # Coordinate System Notes
    /// Generates matrices for standard Y-up view space. The separate Vulkan
    /// coordinate transformation matrix handles conversion to Vulkan conventions.
    pub fn get_view_matrix(&self) -> Mat4 {
        // Use standard look-at matrix generation (Y-up, right-handed)
        // Vulkan coordinate transformation is applied separately
        Mat4::look_at(self.position, self.target, self.up)
    }
    
    /// Generate perspective projection matrix  
    ///
    /// Creates the projection matrix for perspective rendering using the camera's
    /// field of view, aspect ratio, and clipping planes.
    ///
    /// # Returns
    /// 4x4 perspective projection matrix
    ///
    /// # Mathematical Implementation
    /// Follows Johannes Unterguggenberger's mathematically correct approach for
    /// perspective projection. Uses standard right-handed coordinate conventions
    /// in view space before Vulkan coordinate transformation.
    ///
    /// # Aspect Ratio Handling
    /// Automatically uses the current aspect ratio. For dynamic viewports,
    /// ensure set_aspect_ratio() is called when window dimensions change.
    pub fn get_projection_matrix(&self) -> Mat4 {
        // Use standard perspective projection (Y-up view space)
        // Vulkan coordinate transformation applied separately in rendering pipeline
        Mat4::perspective(self.fov, self.aspect, self.near, self.far)
    }
    
    /// Generate combined view-projection matrix
    ///
    /// Creates the complete view-projection transformation following Johannes
    /// Unterguggenberger's mathematically correct approach: P × X × V
    ///
    /// # Matrix Chain Components
    /// - P = Perspective projection matrix
    /// - X = Vulkan coordinate transformation matrix  
    /// - V = View matrix (world to camera space)
    ///
    /// # Returns
    /// 4x4 combined view-projection matrix ready for rendering
    ///
    /// # Usage Notes
    /// This method provides the complete camera transformation. For rendering
    /// individual objects, multiply this result by the model matrix:
    /// `Final = ViewProjection × Model × Vertex`
    ///
    /// # Performance Consideration
    /// This method recalculates matrices on every call. For static cameras,
    /// consider caching the result until camera parameters change.
    pub fn get_view_projection_matrix(&self) -> Mat4 {
        // Calculate component matrices
        let view_matrix = self.get_view_matrix();
        let coord_transform = Mat4::vulkan_coordinate_transform();
        let projection_matrix = self.get_projection_matrix();
        
        // Combine in mathematically correct order: P × X × V
        // This follows Johannes Unterguggenberger's academic approach
        projection_matrix * coord_transform * view_matrix
    }
}

impl Default for Camera {
    /// Create a default perspective camera with sensible settings
    ///
    /// Provides a reasonable starting camera configuration suitable for most
    /// 3D applications. Positioned above and behind the origin, looking down
    /// at the scene center.
    ///
    /// # Default Configuration
    /// - Position: (0, 3, 3) - Above and behind origin
    /// - Target: (0, 0, 0) - Looking at origin
    /// - Up: (0, 1, 0) - Standard Y-up orientation
    /// - FOV: 45 degrees - Natural perspective view
    /// - Aspect: 16:9 - Common widescreen ratio
    /// - Near: 0.1 - Close near plane for detailed objects
    /// - Far: 1000.0 - Distant far plane for large scenes
    ///
    /// # Coordinate System
    /// Uses standard Y-up right-handed coordinates, compatible with common
    /// 3D modeling conventions and the engine's mathematical foundation.
    fn default() -> Self {
        Self {
            position: Vec3::new(0.0, 3.0, 3.0), // Above and behind origin - good default viewpoint
            target: Vec3::zeros(),               // Look at scene center
            up: Vec3::new(0.0, 1.0, 0.0),      // Standard Y-up orientation
            fov: std::f32::consts::FRAC_PI_4,   // 45 degrees in radians - natural FOV
            aspect: 16.0 / 9.0,                 // Widescreen aspect ratio
            near: 0.1,                          // Close near plane - good for detail
            far: 1000.0,                        // Distant far plane - good for landscapes
        }
    }
}

/// High-level rendering error types
///
/// Represents errors that can occur during rendering operations, abstracted
/// from specific graphics API error types to maintain library-agnostic design.
///
/// # Design Philosophy
/// These error types provide meaningful information for application developers
/// without exposing Vulkan-specific details. The actual backend errors are
/// logged for debugging purposes but abstracted from the public API.
#[derive(Error, Debug)]
pub enum RenderError {
    /// Renderer initialization failed during setup
    ///
    /// Occurs when the rendering system cannot be properly initialized, typically
    /// due to missing graphics drivers, incompatible hardware, or configuration issues.
    #[error("Renderer initialization failed: {0}")]
    InitializationFailed(String),
    
    /// A rendering operation failed during execution
    ///
    /// Indicates failure during active rendering operations such as drawing meshes,
    /// updating textures, or presenting frames. Often recoverable with retry logic.
    #[error("Rendering failed: {0}")]
    RenderingFailed(String),
    
    /// Resource creation or management failed
    ///
    /// Occurs when GPU resources (buffers, textures, shaders) cannot be created
    /// or managed properly, typically due to memory constraints or invalid data.
    #[error("Resource creation failed: {0}")]
    ResourceCreationFailed(String),
    
    /// Backend-specific error occurred
    ///
    /// Wraps backend-specific errors (Vulkan, OpenGL, etc.) in a generic form
    /// for consistent error handling across different graphics backends.
    #[error("Backend error: {0}")]
    BackendError(String),
}

/// Result type for rendering operations
pub type RenderResult<T> = Result<T, RenderError>;
