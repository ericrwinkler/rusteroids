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

// Public modules for application use
pub mod window;
pub mod api;

// Core primitives
pub mod primitives;

// Resources
pub mod resources;

// Systems
pub mod systems;

/// Graphics backend implementations
/// 
/// Contains platform-specific rendering backend implementations.
/// Currently supports Vulkan with potential for additional backends in the future.
pub mod backends;

// High-level APIs that applications should use
pub use api::{VulkanRendererConfig, RenderBackend, WindowBackendAccess, BackendResult, MeshHandle, MaterialHandle, ObjectResourceHandle};
pub use resources::materials::ShaderConfig;
pub use window::WindowHandle;

// Core rendering types that applications need
pub use primitives::{Mesh, Vertex, CoordinateSystem, CoordinateConverter, Camera};
pub use resources::materials::{Material, MaterialType, MaterialId, PipelineType, AlphaMode};
pub use systems::lighting::{Light, LightType, LightingEnvironment};
pub use systems::text::{
    FontAtlas, GlyphInfo, TextLayout, TextVertex, TextBounds, text_vertices_to_mesh,
    create_lit_text_material, create_unlit_text_material,
    WorldTextManager, WorldTextHandle, WorldTextConfig,
};

// DEPRECATED: These should be removed in favor of cleaner APIs
// TODO: Remove these exports once applications migrate to new APIs
// Current overlapping patterns that should be consolidated:
// - MaterialManager + TextureManager: Should be unified into ResourceManager
// - SharedRenderingResources: Should be internal to backend, not exposed
// - BatchRenderer: Should be integrated into core renderer, not separate
pub use resources::materials::{
    StandardMaterialParams, UnlitMaterialParams,
    MaterialManager, TextureManager, MaterialTextures, TextureHandle, TextureType
};
pub use resources::pipelines::{
    PipelineManager, PipelineConfig, CullMode,
    // Future rendering attributes (currently stubbed with FIXME)
    BlendMode, RenderPriority, PolygonMode, DepthBiasConfig,
};
pub use resources::shared::{RenderQueue, RenderCommand, CommandType, RenderCommandId};
pub use resources::shared::{GameObject, ObjectUBO, cleanup_game_object};
#[deprecated(since = "0.1.0", note = "SharedRenderingResources is now internal - use MeshHandle/MaterialHandle instead")]
pub use resources::shared::{SharedRenderingResources, SharedRenderingResourcesBuilder};
pub use systems::batching::{BatchRenderer, RenderBatch, BatchError, BatchStats, BatchResult};

use thiserror::Error;
// REMOVED: Engine coupling for cleaner library abstraction
// use crate::engine::{RendererConfig, WindowConfig}; 
use crate::foundation::math::{Vec3, Mat4, Mat4Ext};
use crate::render::systems::dynamic::{
    MeshPoolManager, MeshType, DynamicObjectHandle,
    SpawnerError
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
            .downcast_mut::<crate::render::backends::vulkan::Window>()
            .expect("Expected Vulkan window backend");
        
        let vulkan_renderer = crate::render::backends::vulkan::VulkanRenderer::new(vulkan_window, config)
            .map_err(|e| RenderError::InitializationFailed(format!("Failed to create Vulkan renderer: {:?}", e)))?;
        
        Ok(Self {
            backend: Box::new(vulkan_renderer),
            current_camera: None,
            current_lighting: None,
            frame_count: 0,
            pool_manager: None, // Will be initialized when first needed
        })
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
        // NOTE: Current architecture uses UBOs for lighting (proper approach)
        // Push constants are reserved for frequently changing per-draw data (transforms)
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
                    log::warn!("Only directional lights supported in current UBO implementation. Found: {:?}", first_light.light_type);
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
            log::info!("Multiple light support requires expanding UBO system and shader updates");
        }
        
        log::trace!("Lighting set with {} lights", lighting.lights.len());
    }
    
    /// Set multi-light environment using UBO-based lighting
    /// 
    /// This method uses uniform buffer objects for lighting data (industry best practice),
    /// while push constants handle per-draw transform data for optimal performance.
    /// enabling support for multiple lights while maintaining identical visual output.
    pub fn set_multi_light_environment(&mut self, multi_light_env: &systems::lighting::MultiLightEnvironment) {
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
    
    // === NEW CLEAN API - Use these methods instead ===
    
    /// Load a mesh and return an opaque handle for rendering
    /// This replaces create_shared_rendering_resources with a cleaner abstraction
    pub fn load_mesh(&mut self, mesh: &Mesh, materials: &[Material]) -> Result<api::MeshHandle, Box<dyn std::error::Error>> {
        let mesh_handle = self.backend.create_mesh_resource(mesh, materials)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        
        log::info!("Loaded mesh with handle {:?}", mesh_handle);
        Ok(mesh_handle)
    }
    
    /// Render multiple objects with the same mesh using transforms
    /// This replaces the complex bind_shared_geometry + record_object_draw pattern
    pub fn render_objects(&mut self, mesh_handle: api::MeshHandle, transforms: &[Mat4]) -> Result<(), Box<dyn std::error::Error>> {
        self.backend.render_objects_with_mesh(mesh_handle, transforms)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        
        log::trace!("Rendered {} objects with mesh handle {:?}", transforms.len(), mesh_handle);
        Ok(())
    }
    
    // === DEPRECATED API - Scheduled for removal ===
    
    /// Bind shared resources for multiple objects (NEW API)
    /// DEPRECATED: Use load_mesh instead for cleaner abstraction
    #[deprecated(note = "Use load_mesh for cleaner abstraction")]
    pub fn bind_shared_geometry(&mut self, shared: &SharedRenderingResources) -> Result<(), Box<dyn std::error::Error>> {
        // Bind the shared vertex and index buffers
        // Set the shared mesh data once for all objects
        log::trace!("Binding shared geometry with {} vertices and {} indices", 
                   shared.vertex_count, shared.index_count);
        
        // Call the backend to bind shared resources
        #[allow(deprecated)]
        self.backend.bind_shared_resources(shared)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        
        Ok(())
    }
    
    /// Draw a single object using command recording (DEPRECATED - use render_objects instead)
    /// Records draw command without immediate submission
    #[deprecated(since = "0.1.0", note = "Use render_objects with MeshHandle for cleaner API")]
    pub fn record_object_draw(&mut self, object: &GameObject, shared_resources: &SharedRenderingResources) -> Result<(), Box<dyn std::error::Error>> {
        // Update per-object uniform data  
        let model_matrix = object.get_model_matrix();
        
        // Set per-object uniforms (model matrix, material)
        let model_array: [[f32; 4]; 4] = model_matrix.into();
        self.backend.set_model_matrix(model_array);
        
        let material_color = object.material.get_base_color_array();
        self.backend.set_material_color(material_color);
        
        // TODO: Replace with proper object ID system 
        // For now, use the object's memory address as a unique identifier
        let object_id = object as *const _ as usize as u32;
        
        // Record draw using shared resources and object ID
        #[allow(deprecated)]
        self.backend.record_shared_object_draw(shared_resources, object_id)
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
    
    /// Create a GameObject with proper GPU resource initialization (UPDATED CLEAN API)
    /// This handles the backend resource creation using opaque handles
    pub fn create_game_object(&mut self, position: Vec3, rotation: Vec3, scale: Vec3, material: Material) -> Result<GameObject, Box<dyn std::error::Error>> {
        let mut game_object = GameObject::new(position, rotation, scale, material.clone());
        
        // Use the new clean API to create object resources
        let resource_handle = self.backend.create_object_resources(&material)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        
        // Set the resource handle in the GameObject
        game_object.set_resource_handle(resource_handle);
        
        log::debug!("Created GameObject with clean backend resource handle: {:?}", resource_handle);
        
        Ok(game_object)
    }
    
    /// Update GameObject uniforms using clean backend API
    /// This updates the per-object uniform data (model matrix, material properties)
    pub fn update_object_uniforms(&mut self, game_object: &GameObject) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(resource_handle) = game_object.get_resource_handle() {
            let model_matrix = game_object.get_model_matrix();
            let material_color = game_object.material.get_base_color_array();
            
            self.backend.update_object_uniforms(resource_handle, model_matrix, &material_color)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        } else {
            log::warn!("GameObject has no resource handle - cannot update uniforms");
        }
        
        Ok(())
    }
    
    /// Create SharedRenderingResources for efficient multi-object rendering (DEPRECATED)
    /// This handles the complex Vulkan resource creation for shared geometry
    #[deprecated(since = "0.1.0", note = "Use load_mesh to get MeshHandle for cleaner API")]
    pub fn create_shared_rendering_resources(&self, mesh: &Mesh, materials: &[Material]) -> Result<resources::shared::SharedRenderingResources, Box<dyn std::error::Error>> {
        // Access VulkanContext through backend downcasting
        if let Some(vulkan_backend) = self.backend.as_any().downcast_ref::<crate::render::backends::vulkan::VulkanRenderer>() {
            #[allow(deprecated)]
            resources::shared::create_shared_rendering_resources(vulkan_backend, mesh, materials)
        } else {
            Err("Renderer is not using Vulkan backend".into())
        }
    }
    
    /// Create SharedRenderingResources for instanced dynamic object rendering (DEPRECATED)
    /// This creates resources with an instanced pipeline for efficient batch rendering
    #[deprecated(since = "0.1.0", note = "Use load_mesh to get MeshHandle for cleaner API")]
    pub fn create_instanced_rendering_resources(&self, mesh: &Mesh, materials: &[Material]) -> Result<resources::shared::SharedRenderingResources, Box<dyn std::error::Error>> {
        // Access VulkanContext through backend downcasting
        if let Some(vulkan_backend) = self.backend.as_any().downcast_ref::<crate::render::backends::vulkan::VulkanRenderer>() {
            #[allow(deprecated)]
            resources::shared::create_instanced_rendering_resources(
                vulkan_backend,
                mesh,
                materials,
                |context, frame_layout, material_layout| {
                    self.create_instanced_pipeline(context, frame_layout, material_layout)
                },
            )
        } else {
            Err("Renderer is not using Vulkan backend".into())
        }
    }
    
    /// Create instanced pipeline for dynamic object rendering
    fn create_instanced_pipeline(&self, context: &crate::render::backends::vulkan::VulkanContext, frame_layout: ash::vk::DescriptorSetLayout, material_layout: ash::vk::DescriptorSetLayout) -> Result<(crate::render::backends::vulkan::GraphicsPipeline, ash::vk::PipelineLayout), Box<dyn std::error::Error>> {
        // Get render pass from VulkanRenderer
        if let Some(vulkan_backend) = self.backend.as_any().downcast_ref::<crate::render::backends::vulkan::VulkanRenderer>() {
            let render_pass = vulkan_backend.get_render_pass();
            
            // Delegate to pipeline builder module
            systems::dynamic::instancing::create_instanced_pipeline(
                context,
                frame_layout,
                material_layout,
                render_pass,
            )
        } else {
            Err("Backend is not Vulkan".into())
        }
    }
    
    /// Helper method to convert lighting environment (temporary until lighting refactor)
    fn convert_lighting_to_multi_light_environment(&self, _lighting: &LightingEnvironment) -> systems::lighting::MultiLightEnvironment {
        // Convert from the old LightingEnvironment to the new MultiLightEnvironment
        // For now, create a simple environment with ambient lighting
        // TODO: Implement proper conversion when lighting system is refactored
        systems::lighting::MultiLightEnvironment::new()
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
        systems::dynamic::graphics_api::initialize_dynamic_system(&mut self.pool_manager, &*self.backend, max_objects_per_pool)?;
        
        // Initialize backend's dynamic rendering system
        self.backend.initialize_dynamic_rendering(max_objects_per_pool)?;
        
        Ok(())
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
        mesh_type: crate::render::systems::dynamic::MeshType,
        mesh: &Mesh,
        materials: &[Material],
        max_objects: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Create SharedRenderingResources first (before borrowing pool_manager mutably)
        #[allow(deprecated)]
        let shared_resources = self.create_instanced_rendering_resources(mesh, materials)?;
        
        // Extract texture from first material
        let texture_handle = materials.get(0)
            .and_then(|mat| mat.textures.base_color);
        
        log::info!("Creating mesh pool for {:?} with capacity {} and texture {:?}", mesh_type, max_objects, texture_handle);
        
        // Now delegate to graphics_api with pre-created resources
        if let Some(ref mut mgr) = self.pool_manager {
            if let Some(vulkan_backend) = self.backend.as_any_mut().downcast_mut::<crate::render::backends::vulkan::VulkanRenderer>() {
                let material_descriptor_set = if let Some(tex_handle) = texture_handle {
                    vulkan_backend.create_material_descriptor_set_with_texture(tex_handle)?
                } else {
                    vulkan_backend.get_default_material_descriptor_set()
                };
                
                let context = vulkan_backend.get_context();
                
                mgr.create_pool(
                    mesh_type,
                    shared_resources,
                    max_objects,
                    context,
                    material_descriptor_set,
                    texture_handle,
                )?;
                log::info!("Successfully created pool for {:?}", mesh_type);
                Ok(())
            } else {
                Err("Backend is not a VulkanRenderer".into())
            }
        } else {
            Err("Dynamic system not initialized".into())
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
    /// use crate::render::systems::dynamic::MeshType;
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
        material: Material,
    ) -> Result<DynamicObjectHandle, SpawnerError> {
        systems::dynamic::graphics_api::spawn_dynamic_object(
            &mut self.pool_manager,
            mesh_type,
            position,
            rotation,
            scale,
            material,
        )
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
        systems::dynamic::graphics_api::despawn_dynamic_object(&mut self.pool_manager, mesh_type, handle)
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
        systems::dynamic::graphics_api::update_dynamic_object_transform(
            &mut self.pool_manager,
            mesh_type,
            handle,
            position,
            rotation,
            scale,
        )
    }

    /// Update only the position of a dynamic object, preserving rotation and scale
    pub fn update_dynamic_object_position(
        &mut self,
        mesh_type: MeshType,
        handle: DynamicObjectHandle,
        position: Vec3,
    ) -> Result<(), SpawnerError> {
        systems::dynamic::graphics_api::update_dynamic_object_position(
            &mut self.pool_manager,
            mesh_type,
            handle,
            position,
        )
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
        systems::dynamic::graphics_api::begin_dynamic_frame(&mut self.pool_manager, delta_time);
        
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
        // Get camera position and target for depth sorting
        let (camera_position, camera_target) = self.current_camera.as_ref()
            .map(|cam| (cam.position, cam.target))
            .unwrap_or((Vec3::zeros(), Vec3::new(0.0, 0.0, -1.0)));
        
        systems::dynamic::graphics_api::record_dynamic_draws(
            &mut self.pool_manager,
            &mut *self.backend,
            camera_position,
            camera_target,
        )
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
    pub fn get_dynamic_stats(&self) -> Option<crate::render::systems::dynamic::PoolManagerStats> {
        systems::dynamic::graphics_api::get_dynamic_stats(&self.pool_manager)
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
    // TEXTURE MANAGEMENT API
    // ================================

    /// Upload texture from image data to GPU
    ///
    /// Uploads a texture from raw image data (useful for procedurally generated textures
    /// like font atlases). The texture is uploaded to GPU memory and a handle is returned
    /// for use with materials.
    ///
    /// # Arguments
    /// * `image_data` - Image data (RGBA format)
    /// * `texture_type` - Type of texture (BaseColor, Normal, etc.)
    ///
    /// # Returns
    /// TextureHandle for use with materials, or error on failure
    ///
    /// # Example
    /// ```rust
    /// // Upload font atlas texture
    /// let image_data = ImageData {
    ///     data: atlas_pixels,
    ///     width: 1024,
    ///     height: 1024,
    ///     channels: 4,
    /// };
    /// let texture_handle = graphics_engine.upload_texture_from_image_data(
    ///     image_data,
    ///     TextureType::BaseColor,
    /// )?;
    /// ```
    pub fn upload_texture_from_image_data(
        &mut self,
        image_data: crate::assets::ImageData,
        texture_type: resources::materials::TextureType,
    ) -> Result<resources::materials::TextureHandle, Box<dyn std::error::Error>> {
        // Delegate to backend's texture manager
        if let Some(vulkan_backend) = self.backend.as_any_mut().downcast_mut::<crate::render::backends::vulkan::VulkanRenderer>() {
            let texture_handle = vulkan_backend.upload_texture_from_image_data(image_data, texture_type)?;
            log::debug!("Uploaded texture to GPU: {:?} (type: {:?})", texture_handle, texture_type);
            Ok(texture_handle)
        } else {
            Err("Backend doesn't support texture upload".into())
        }
    }
    
    /// Recreate the Vulkan swapchain for window resizing
    pub fn recreate_swapchain(&mut self, window_handle: &mut WindowHandle) {
        log::info!("Recreating swapchain for window resize");
        
        if let Err(e) = self.backend.recreate_swapchain(window_handle) {
            log::error!("Failed to recreate swapchain: {:?}", e);
        } else {
            log::info!("Successfully recreated swapchain");
        }
    }
    
    /// Get the current swapchain dimensions
    pub fn get_swapchain_extent(&self) -> (u32, u32) {
        self.backend.get_swapchain_extent()
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
