//! Vulkan context management
//! 
//! Provides low-level Vulkan context initialization and management
//! following the ownership and resource management rules defined in DESIGN.md

use ash::{Device, Entry, Instance};
#[cfg(debug_assertions)]
use ash::extensions::ext::DebugUtils;
use ash::extensions::khr::{Surface, Swapchain as SwapchainLoader};
use ash::vk;
use std::ffi::{CStr, CString};
use thiserror::Error;
use crate::render::backends::vulkan::state::swapchain::Swapchain;

/// Vulkan context errors
/// Vulkan-specific error types
#[derive(Error, Debug)]
pub enum VulkanError {
    /// General Vulkan API error with result code
    #[error("Vulkan API error: {0:?}")]
    Api(vk::Result),
    
    /// Resource with specified ID could not be found
    #[error("Resource not found: {id}")]
    ResourceNotFound { 
        /// The unique identifier of the resource
        id: u64 
    },
    
    /// Invalid operation attempted
    #[error("Invalid operation: {reason}")]
    InvalidOperation { 
        /// Description of why the operation is invalid
        reason: String 
    },
    
    /// Memory allocation failed
    #[error("Out of memory: {requested} bytes")]
    OutOfMemory { 
        /// Number of bytes that were requested
        requested: usize 
    },
    
    /// Vulkan context initialization failed
    #[error("Initialization failed: {0}")]
    InitializationFailed(String),
    
    /// No suitable memory type found for allocation
    #[error("No suitable memory type found")]
    NoSuitableMemoryType,
}

/// Result type for Vulkan operations
pub type VulkanResult<T> = Result<T, VulkanError>;

/// Vulkan instance wrapper with RAII cleanup
pub struct VulkanInstance {
    /// Vulkan entry point
    pub entry: Entry,
    /// Vulkan instance handle
    pub instance: Instance,
    /// Debug utilities extension (debug builds)
    #[cfg(debug_assertions)]
    pub debug_utils: Option<DebugUtils>,
    /// Debug messenger handle (debug builds)
    #[cfg(debug_assertions)]
    pub debug_messenger: Option<vk::DebugUtilsMessengerEXT>,
}

impl VulkanInstance {
    /// Create a new Vulkan instance with validation layers
        pub fn new(window: &mut crate::render::backends::vulkan::Window, app_name: &str, enable_validation: bool) -> VulkanResult<Self> {
        let entry = unsafe { Entry::load() }
            .map_err(|e| VulkanError::InitializationFailed(format!("Failed to load Vulkan: {:?}", e)))?;
        
        // Create application info
        let app_name_cstr = CString::new(app_name).unwrap();
        let engine_name_cstr = CString::new("RustEngine").unwrap();
        let app_info = vk::ApplicationInfo::builder()
            .application_name(&app_name_cstr)
            .application_version(vk::make_api_version(0, 1, 0, 0))
            .engine_name(&engine_name_cstr)
            .engine_version(vk::make_api_version(0, 1, 0, 0))
            .api_version(vk::API_VERSION_1_0);

        // Get required extensions from GLFW
        let required_extensions = window.get_required_instance_extensions()
            .map_err(|e| VulkanError::InitializationFailed(format!("Failed to get required extensions: {}", e)))?;
        
        let cstr_extensions: Vec<CString> = required_extensions
            .iter()
            .map(|ext| CString::new(ext.as_str()).unwrap())
            .collect();
        
        #[allow(unused_mut)] // Mutable in debug builds for adding debug extensions
        let mut extensions: Vec<*const i8> = cstr_extensions
            .iter()
            .map(|ext| ext.as_ptr())
            .collect();

        // Add debug extensions for debug builds
        #[cfg(debug_assertions)]
        if enable_validation {
            // Note: VK_EXT_debug_utils warning is expected and safe for debug builds.
            // This extension is intended for development/debugging use and should be
            // disabled in release builds (which this #[cfg(debug_assertions)] ensures).
            extensions.push(DebugUtils::name().as_ptr());
        }

        // Get validation layers - including synchronization validation for development
        let layer_names = if cfg!(debug_assertions) && enable_validation {
            vec![
                CString::new("VK_LAYER_KHRONOS_validation").unwrap(),
                // Note: VK_LAYER_KHRONOS_synchronization2 is typically enabled via 
                // VK_LAYER_KHRONOS_validation with synchronization validation features
            ]
        } else {
            vec![]
        };
        
        let layer_names_ptrs: Vec<*const i8> = layer_names.iter()
            .map(|name| name.as_ptr())
            .collect();

        // Enable synchronization validation features for development builds
        #[cfg(debug_assertions)]
        let mut validation_features = if enable_validation {
            Some(vk::ValidationFeaturesEXT::builder()
                .enabled_validation_features(&[
                    vk::ValidationFeatureEnableEXT::SYNCHRONIZATION_VALIDATION,
                    vk::ValidationFeatureEnableEXT::BEST_PRACTICES,
                ]))
        } else {
            None
        };

        // Create instance with synchronization validation
        #[cfg(debug_assertions)]
        let mut create_info_builder = vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_extension_names(&extensions)
            .enabled_layer_names(&layer_names_ptrs);

        #[cfg(debug_assertions)]
        if enable_validation {
            if let Some(ref mut features) = validation_features {
                create_info_builder = create_info_builder.push_next(features);
            }
        }

        #[cfg(debug_assertions)]
        let create_info = create_info_builder;

        #[cfg(not(debug_assertions))]
        let create_info = vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_extension_names(&extensions)
            .enabled_layer_names(&layer_names_ptrs);

        let instance = unsafe {
            entry.create_instance(&create_info, None)
                .map_err(VulkanError::Api)?
        };

        // Set up debug messenger for debug builds
        #[cfg(debug_assertions)]
        let (debug_utils, debug_messenger) = if enable_validation {
            let debug_utils = DebugUtils::new(&entry, &instance);
            let debug_messenger = Self::setup_debug_messenger(&debug_utils)?;
            (Some(debug_utils), Some(debug_messenger))
        } else {
            (None, None)
        };

        Ok(Self {
            entry,
            instance,
            #[cfg(debug_assertions)]
            debug_utils,
            #[cfg(debug_assertions)]
            debug_messenger,
        })
    }

    #[cfg(debug_assertions)]
    fn setup_debug_messenger(debug_utils: &DebugUtils) -> VulkanResult<vk::DebugUtilsMessengerEXT> {
        let create_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
            )
            .message_type(
                vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
            )
            .pfn_user_callback(Some(debug_callback));

        unsafe {
            debug_utils.create_debug_utils_messenger(&create_info, None)
                .map_err(VulkanError::Api)
        }
    }
}

impl Drop for VulkanInstance {
    fn drop(&mut self) {
        unsafe {
            #[cfg(debug_assertions)]
            if let (Some(debug_utils), Some(debug_messenger)) = 
                (&self.debug_utils, &self.debug_messenger) {
                debug_utils.destroy_debug_utils_messenger(*debug_messenger, None);
            }
            
            self.instance.destroy_instance(None);
        }
    }
}

/// Debug callback for validation layers
#[cfg(debug_assertions)]
unsafe extern "system" fn debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::ffi::c_void,
) -> vk::Bool32 {
    let callback_data = *callback_data;
    let message = CStr::from_ptr(callback_data.p_message).to_string_lossy();

    if message_severity >= vk::DebugUtilsMessageSeverityFlagsEXT::ERROR {
        log::error!("[Vulkan] {:?} - {}", message_type, message);
    } else if message_severity >= vk::DebugUtilsMessageSeverityFlagsEXT::WARNING {
        log::warn!("[Vulkan] {:?} - {}", message_type, message);
    } else {
        log::debug!("[Vulkan] {:?} - {}", message_type, message);
    }

    vk::FALSE
}

/// Physical device selection and capabilities
pub struct PhysicalDeviceInfo {
    /// Vulkan physical device handle
    pub device: vk::PhysicalDevice,
    /// Device properties and limits
    pub properties: vk::PhysicalDeviceProperties,
    /// Supported device features
    pub features: vk::PhysicalDeviceFeatures,
    /// Available queue families
    pub queue_families: Vec<vk::QueueFamilyProperties>,
    /// Index of the graphics queue family
    pub graphics_family: u32,
    /// Index of the presentation queue family
    pub present_family: u32,
}

impl PhysicalDeviceInfo {
    /// Select a suitable physical device for rendering
    pub fn select_suitable_device(
        instance: &Instance,
        surface: vk::SurfaceKHR,
        surface_loader: &Surface,
    ) -> VulkanResult<Self> {
        let devices = unsafe {
            instance.enumerate_physical_devices()
                .map_err(VulkanError::Api)?
        };

        for device in devices {
            if let Ok(device_info) = Self::evaluate_device(instance, device, surface, surface_loader) {
                log::info!("Selected GPU: {}", unsafe {
                    CStr::from_ptr(device_info.properties.device_name.as_ptr()).to_string_lossy()
                });
                return Ok(device_info);
            }
        }

        Err(VulkanError::InitializationFailed(
            "No suitable GPU found".to_string()
        ))
    }

    fn evaluate_device(
        instance: &Instance,
        device: vk::PhysicalDevice,
        surface: vk::SurfaceKHR,
        surface_loader: &Surface,
    ) -> VulkanResult<Self> {
        let properties = unsafe { instance.get_physical_device_properties(device) };
        let features = unsafe { instance.get_physical_device_features(device) };
        let queue_families = unsafe {
            instance.get_physical_device_queue_family_properties(device)
        };

        // Find graphics and present queue families
        let mut graphics_family = None;
        let mut present_family = None;

        for (index, family) in queue_families.iter().enumerate() {
            let index = index as u32;
            
            if family.queue_flags.contains(vk::QueueFlags::GRAPHICS) && graphics_family.is_none() {
                graphics_family = Some(index);
            }

            let present_support = unsafe {
                surface_loader.get_physical_device_surface_support(device, index, surface)
                    .map_err(VulkanError::Api)?
            };

            if present_support && present_family.is_none() {
                present_family = Some(index);
            }

            if graphics_family.is_some() && present_family.is_some() {
                break;
            }
        }

        let graphics_family = graphics_family.ok_or_else(|| {
            VulkanError::InitializationFailed("No graphics queue family found".to_string())
        })?;

        let present_family = present_family.ok_or_else(|| {
            VulkanError::InitializationFailed("No present queue family found".to_string())
        })?;

        // Check device extensions support
        let extensions = unsafe {
            instance.enumerate_device_extension_properties(device)
                .map_err(VulkanError::Api)?
        };

        let required_extensions = [SwapchainLoader::name()];
        let has_required_extensions = required_extensions.iter().all(|required| {
            extensions.iter().any(|available| {
                let extension_name = unsafe { 
                    CStr::from_ptr(available.extension_name.as_ptr()) 
                };
                extension_name == *required
            })
        });

        if !has_required_extensions {
            return Err(VulkanError::InitializationFailed(
                "Required device extensions not supported".to_string()
            ));
        }

        Ok(Self {
            device,
            properties,
            features,
            queue_families,
            graphics_family,
            present_family,
        })
    }
}

/// Logical device wrapper with RAII cleanup
pub struct LogicalDevice {
    /// Vulkan logical device handle
    pub device: Device,
    /// Graphics operations queue
    pub graphics_queue: vk::Queue,
    /// Surface presentation queue
    pub present_queue: vk::Queue,
    /// Index of the graphics queue family
    pub graphics_family: u32,
    /// Index of the presentation queue family
    pub present_family: u32,
    /// Swapchain extension loader
    pub swapchain_loader: SwapchainLoader,
}

impl LogicalDevice {
    /// Create a new logical device with required queues
    pub fn new(
        instance: &Instance,
        physical_device_info: &PhysicalDeviceInfo,
    ) -> VulkanResult<Self> {
        let unique_families: std::collections::HashSet<u32> = [
            physical_device_info.graphics_family,
            physical_device_info.present_family,
        ].iter().cloned().collect();

        let queue_infos: Vec<vk::DeviceQueueCreateInfo> = unique_families
            .iter()
            .map(|&family| {
                vk::DeviceQueueCreateInfo::builder()
                    .queue_family_index(family)
                    .queue_priorities(&[1.0])
                    .build()
            })
            .collect();

        let required_extensions = [SwapchainLoader::name().as_ptr()];

        let device_features = vk::PhysicalDeviceFeatures::builder()
            .sampler_anisotropy(true)  // Enable anisotropic filtering for better texture quality
            .build();

        let create_info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(&queue_infos)
            .enabled_extension_names(&required_extensions)
            .enabled_features(&device_features);

        let device = unsafe {
            instance.create_device(physical_device_info.device, &create_info, None)
                .map_err(VulkanError::Api)?
        };

        let graphics_queue = unsafe {
            device.get_device_queue(physical_device_info.graphics_family, 0)
        };

        let present_queue = unsafe {
            device.get_device_queue(physical_device_info.present_family, 0)
        };

        let swapchain_loader = SwapchainLoader::new(instance, &device);
        
        // Note: You may see validation warnings about vkGetDeviceProcAddr() trying to grab
        // instance-level functions like vkGetPhysicalDevicePresentRectanglesKHR. This is a
        // known issue with ash's swapchain loader initialization and can be safely ignored.

        Ok(Self {
            device,
            graphics_queue,
            present_queue,
            graphics_family: physical_device_info.graphics_family,
            present_family: physical_device_info.present_family,
            swapchain_loader,
        })
    }
}

impl Drop for LogicalDevice {
    fn drop(&mut self) {
        unsafe {
            // Ensure device is idle before destruction
            let _ = self.device.device_wait_idle();
            self.device.destroy_device(None);
        }
    }
}

/// Main Vulkan context that owns all core Vulkan resources
pub struct VulkanContext {
    /// Vulkan surface for rendering
    pub surface: vk::SurfaceKHR,
    /// Surface extension loader
    pub surface_loader: Surface,
    /// Selected physical device information
    pub physical_device: PhysicalDeviceInfo,
    /// Swapchain for presenting frames
    pub swapchain: Option<crate::render::backends::vulkan::Swapchain>,
    /// Logical device for operations
    pub device: LogicalDevice,
    /// Vulkan instance and debug utilities
    pub instance: VulkanInstance,
}

impl VulkanContext {
    /// Create a new Vulkan context for the window
    pub fn new(window: &mut crate::render::backends::vulkan::Window, app_name: &str) -> VulkanResult<Self> {
        // Create Vulkan instance
        let instance = VulkanInstance::new(window, app_name, cfg!(debug_assertions))?;

        // Create surface using GLFW's built-in method
        let surface_loader = Surface::new(&instance.entry, &instance.instance);
        let surface = window.create_vulkan_surface(instance.instance.handle())
            .map_err(|e| VulkanError::InitializationFailed(format!("Surface creation: {}", e)))?;

        // Select physical device
        let physical_device = PhysicalDeviceInfo::select_suitable_device(
            &instance.instance, surface, &surface_loader
        )?;

        // Create logical device
        let device = LogicalDevice::new(&instance.instance, &physical_device)?;

        // Create swapchain
        let window_size = window.get_framebuffer_size();
        let window_extent = vk::Extent2D {
            width: window_size.0,
            height: window_size.1,
        };
        
        let swapchain = crate::render::backends::vulkan::Swapchain::new(
            &instance.instance,
            device.device.clone(),
            surface,
            &surface_loader,
            &physical_device,
            window_extent,
        )?;

        Ok(Self {
            instance,
            surface,
            surface_loader,
            physical_device,
            device,
            swapchain: Some(swapchain),
        })
    }

    /// Get a reference to the Vulkan entry
    pub fn entry(&self) -> &Entry {
        &self.instance.entry
    }

    /// Get a reference to the Vulkan instance
    pub fn instance(&self) -> &Instance {
        &self.instance.instance
    }

    /// Get the surface handle
    pub fn surface(&self) -> vk::SurfaceKHR {
        self.surface
    }

    /// Get the surface loader
    pub fn surface_loader(&self) -> &Surface {
        &self.surface_loader
    }

    /// Get the physical device info
    pub fn physical_device(&self) -> &PhysicalDeviceInfo {
        &self.physical_device
    }

    /// Get the logical device
    pub fn device(&self) -> &LogicalDevice {
        &self.device
    }

    /// Get the raw Device handle
    pub fn raw_device(&self) -> Device {
        self.device.device.clone()
    }

    /// Get the VulkanInstance
    pub fn vulkan_instance(&self) -> &VulkanInstance {
        &self.instance
    }

    /// Get the swapchain
    pub fn swapchain(&self) -> &Swapchain {
        self.swapchain.as_ref().unwrap()
    }

    /// Get the swapchain loader
    pub fn swapchain_loader(&self) -> &SwapchainLoader {
        &self.device.swapchain_loader
    }

    /// Get the graphics queue
    pub fn graphics_queue(&self) -> vk::Queue {
        self.device.graphics_queue
    }

    /// Get the present queue  
    pub fn present_queue(&self) -> vk::Queue {
        self.device.present_queue
    }

    /// Get the graphics queue family index
    pub fn graphics_queue_family(&self) -> u32 {
        self.physical_device.graphics_family
    }
    
    /// Recreate the swapchain (for window resizing)
    pub fn recreate_swapchain(&mut self, window: &crate::render::backends::vulkan::Window) -> VulkanResult<()> {
        // Wait for device to be idle before recreating swapchain
        unsafe {
            self.device.device.device_wait_idle().map_err(VulkanError::Api)?;
        }
        
        // Get new window size
        let window_size = window.get_framebuffer_size();
        let window_extent = vk::Extent2D {
            width: window_size.0,
            height: window_size.1,
        };
        
        // Get the old swapchain handle before replacement
        let old_swapchain = self.swapchain.as_ref().map(|s| s.handle()).unwrap_or(vk::SwapchainKHR::null());
        
        // Create new swapchain using the old one for proper recreation
        let new_swapchain = crate::render::backends::vulkan::Swapchain::recreate(
            &self.instance.instance,
            self.device.device.clone(),
            self.surface,
            &self.surface_loader,
            &self.physical_device,
            window_extent,
            old_swapchain,
        )?;
        
        // Replace the old swapchain with the new one
        // The old swapchain will be automatically destroyed when dropped by RAII
        // No manual destruction needed - this prevents validation errors
        self.swapchain = Some(new_swapchain);
        
        Ok(())
    }
}

impl Drop for VulkanContext {
    fn drop(&mut self) {
        unsafe {
            // Wait for device to be idle before cleanup
            let _ = self.device.device.device_wait_idle();
            
            // Explicitly drop swapchain first if it exists
            if let Some(_) = self.swapchain.take() {
                // Swapchain drop will be called automatically
            }
            
            // Now destroy the surface before device cleanup
            self.surface_loader.destroy_surface(self.surface, None);
        }
        // Fields will drop in reverse declaration order:
        // 1. instance (VulkanInstance - last, contains the instance)
        // 2. device (LogicalDevice - second-to-last, destroys the device)
        // 3. swapchain, physical_device, surface_loader, surface (already handled above)
        // This ensures device is destroyed before instance
    }
}
