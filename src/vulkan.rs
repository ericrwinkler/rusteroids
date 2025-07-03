extern crate ash;
extern crate glfw;


pub struct Vulkan {
    instance: ash::Instance,
    application_information: ash::vk::ApplicationInfo,
    instance_create_info: ash::vk::InstanceCreateInfo,
}

impl Vulkan {
    pub fn new() -> Self {
        
        let mut application_information = ash::vk::ApplicationInfo::default();
        application_information.p_application_name = "Rusteroids".as_ptr() as *const _;
        application_information.application_version = ash::vk::make_api_version(0, 0, 1, 0);
        application_information.p_engine_name = "No Engine".as_ptr() as *const _;
        application_information.engine_version = ash::vk::make_api_version(0, 0, 1, 0);
        application_information.api_version = ash::vk::make_api_version(0, 0, 1, 0);


        let mut instance_create_info = ash::vk::InstanceCreateInfo::default();
        instance_create_info.s_type = ash::vk::StructureType::INSTANCE_CREATE_INFO;
        instance_create_info.p_application_info = &application_information;

        // ...existing code...

        // Get required GLFW extensions for Vulkan
        let glfw_extensions = glfw::get_required_instance_extensions()
            .expect("Failed to get required GLFW Vulkan extensions");

        // Convert Vec<&str> to Vec<*const i8> for Vulkan
        let raw_extensions: Vec<*const i8> = glfw_extensions
            .iter()
            .map(|ext| ext.as_ptr() as *const i8)
            .collect();

        // Set extension count and names in create_info
        instance_create_info.enabled_extension_count = raw_extensions.len() as u32;
        instance_create_info.pp_enabled_extension_names = raw_extensions.as_ptr();
        
        Vulkan {
            
        }
    }
}
