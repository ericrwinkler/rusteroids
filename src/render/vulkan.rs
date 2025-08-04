// src/render/vulkan.rs
use crate::render::window::WindowHandle;
use crate::render::buffer::{Buffer, BufferBuilder};
use crate::render::mesh::Vertex;
use ash::extensions::khr::{Surface, Swapchain};
use ash::{Entry, vk};

pub struct VulkanContext {
    #[allow(dead_code)] // Fixme - remove this when not needed
    pub entry: Entry,
    pub instance: ash::Instance,
    pub surface_loader: Surface,
    pub surface: vk::SurfaceKHR,
    pub physical_device: vk::PhysicalDevice,
    pub device: ash::Device,
    pub graphics_queue: vk::Queue,
    #[allow(dead_code)] // Fixme - remove this when not needed
    pub graphics_queue_family_index: u32,
    pub swapchain_loader: Swapchain,
    pub swapchain: vk::SwapchainKHR,
    pub image_views: Vec<vk::ImageView>,
    pub render_pass: vk::RenderPass,
    pub framebuffers: Vec<vk::Framebuffer>,
    pub vert_shader_module: vk::ShaderModule,
    pub frag_shader_module: vk::ShaderModule,
    pub pipeline_layout: vk::PipelineLayout,
    pub graphics_pipeline: vk::Pipeline,
    pub command_pool: vk::CommandPool,
    pub command_buffers: Vec<vk::CommandBuffer>,
    pub image_available_semaphores: Vec<vk::Semaphore>,
    pub render_finished_semaphores: Vec<vk::Semaphore>,
    pub in_flight_fences: Vec<vk::Fence>,
    pub current_frame: usize,
    pub mesh_vertex_count: usize, // Track how many vertices we should render
    pub vertex_buffer: Option<Buffer>,
    pub index_buffer: Option<Buffer>,
    pub mvp_matrix: [f32; 16], // Current Model-View-Projection matrix
    // Depth buffer resources
    pub depth_image: vk::Image,
    pub depth_image_memory: vk::DeviceMemory,
    pub depth_image_view: vk::ImageView,
}

impl VulkanContext {
    // Helper function to find memory type index
    fn find_memory_type(
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
        type_filter: u32,
        properties: vk::MemoryPropertyFlags,
    ) -> u32 {
        let mem_properties = unsafe {
            instance.get_physical_device_memory_properties(physical_device)
        };
        
        for i in 0..mem_properties.memory_type_count {
            if (type_filter & (1 << i)) != 0
                && mem_properties.memory_types[i as usize]
                    .property_flags
                    .contains(properties)
            {
                return i;
            }
        }
        
        panic!("Failed to find suitable memory type");
    }
    
    // Helper function to create depth buffer
    fn create_depth_resources(
        device: &ash::Device,
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
        extent: vk::Extent2D,
    ) -> (vk::Image, vk::DeviceMemory, vk::ImageView) {
        let depth_format = vk::Format::D32_SFLOAT;
        
        // Create depth image
        let image_info = vk::ImageCreateInfo::builder()
            .image_type(vk::ImageType::TYPE_2D)
            .extent(vk::Extent3D {
                width: extent.width,
                height: extent.height,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .format(depth_format)
            .tiling(vk::ImageTiling::OPTIMAL)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .samples(vk::SampleCountFlags::TYPE_1);
            
        let depth_image = unsafe {
            device.create_image(&image_info, None)
                .expect("Failed to create depth image")
        };
        
        // Allocate memory for depth image
        let mem_requirements = unsafe {
            device.get_image_memory_requirements(depth_image)
        };
        
        let mem_type_index = Self::find_memory_type(
            instance,
            physical_device,
            mem_requirements.memory_type_bits,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        );
        
        let alloc_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(mem_requirements.size)
            .memory_type_index(mem_type_index);
            
        let depth_image_memory = unsafe {
            device.allocate_memory(&alloc_info, None)
                .expect("Failed to allocate depth image memory")
        };
        
        unsafe {
            device.bind_image_memory(depth_image, depth_image_memory, 0)
                .expect("Failed to bind depth image memory");
        }
        
        // Create depth image view
        let view_info = vk::ImageViewCreateInfo::builder()
            .image(depth_image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(depth_format)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::DEPTH,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });
            
        let depth_image_view = unsafe {
            device.create_image_view(&view_info, None)
                .expect("Failed to create depth image view")
        };
        
        (depth_image, depth_image_memory, depth_image_view)
    }

    pub fn new(window: &mut WindowHandle) -> Self {
        // Vulkan: Entry and Instance
        let entry = unsafe { ash::Entry::load().unwrap() };
        let app_name = std::ffi::CString::new("VulkanTriangle").unwrap();
        let app_info = ash::vk::ApplicationInfo::builder()
            .application_name(&app_name)
            .application_version(0)
            .engine_name(&app_name)
            .engine_version(0)
            .api_version(ash::vk::API_VERSION_1_0);

        // Get required extensions from GLFW
        let required_extensions = window
            .glfw
            .get_required_instance_extensions()
            .expect("Unable to get required instance extensions");
        let cstr_extensions: Vec<std::ffi::CString> = required_extensions
            .iter()
            .map(|ext| std::ffi::CString::new(ext.as_str()).unwrap())
            .collect();
        let raw_extensions: Vec<*const i8> =
            cstr_extensions.iter().map(|ext| ext.as_ptr()).collect();

        let create_info = ash::vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_extension_names(&raw_extensions);

        let instance = unsafe {
            entry
                .create_instance(&create_info, None)
                .expect("Failed to create Vulkan instance")
        };

        // Create Vulkan surface for the GLFW window
        let mut surface = ash::vk::SurfaceKHR::null();
        let result = window.create_window_surface(instance.handle(), &mut surface);
        if result != ash::vk::Result::SUCCESS {
            panic!("Failed to create Vulkan surface: error code {:?}", result);
        }
        let surface_loader = ash::extensions::khr::Surface::new(&entry, &instance);

        // Step 1: Select a physical device and queue family
        let physical_devices = unsafe { instance.enumerate_physical_devices() }
            .expect("Failed to enumerate physical devices");
        let physical_device = *physical_devices
            .get(0)
            .expect("No Vulkan physical device found");

        // Find a queue family that supports graphics and presentation
        let queue_family_properties =
            unsafe { instance.get_physical_device_queue_family_properties(physical_device) };
        let mut graphics_queue_family_index = None;
        for (index, info) in queue_family_properties.iter().enumerate() {
            let supports_graphics = info.queue_flags.contains(ash::vk::QueueFlags::GRAPHICS);
            let supports_present = unsafe {
                surface_loader
                    .get_physical_device_surface_support(physical_device, index as u32, surface)
                    .unwrap_or(false)
            };
            if supports_graphics && supports_present {
                graphics_queue_family_index = Some(index as u32);
                break;
            }
        }
        let graphics_queue_family_index =
            graphics_queue_family_index.expect("No suitable queue family found");

        // Step 2: Create logical device and graphics queue (with swapchain extension)
        let queue_priorities = [1.0f32];
        let queue_info = ash::vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(graphics_queue_family_index)
            .queue_priorities(&queue_priorities);
        let device_extensions = [ash::extensions::khr::Swapchain::name().as_ptr()];
        let device_create_info = ash::vk::DeviceCreateInfo::builder()
            .queue_create_infos(std::slice::from_ref(&queue_info))
            .enabled_extension_names(&device_extensions);
        let device = unsafe {
            instance
                .create_device(physical_device, &device_create_info, None)
                .expect("Failed to create logical device")
        };
        let graphics_queue = unsafe { device.get_device_queue(graphics_queue_family_index, 0) };

        // Step 3: Create swapchain
        let swapchain_loader = ash::extensions::khr::Swapchain::new(&instance, &device);
        let surface_caps = unsafe {
            surface_loader
                .get_physical_device_surface_capabilities(physical_device, surface)
                .expect("Failed to get surface capabilities")
        };
        let surface_formats = unsafe {
            surface_loader
                .get_physical_device_surface_formats(physical_device, surface)
                .expect("Failed to get surface formats")
        };
        let surface_format = surface_formats
            .iter()
            .find(|sf| sf.format == ash::vk::Format::B8G8R8A8_SRGB)
            .cloned()
            .unwrap_or(surface_formats[0]);
        let present_modes = unsafe {
            surface_loader
                .get_physical_device_surface_present_modes(physical_device, surface)
                .expect("Failed to get present modes")
        };
        let present_mode = present_modes
            .iter()
            .cloned()
            .find(|&mode| mode == ash::vk::PresentModeKHR::MAILBOX)
            .unwrap_or(ash::vk::PresentModeKHR::FIFO);
        let image_count = surface_caps.min_image_count + 1;
        let image_count =
            if surface_caps.max_image_count > 0 && image_count > surface_caps.max_image_count {
                surface_caps.max_image_count
            } else {
                image_count
            };
        let swapchain_create_info = ash::vk::SwapchainCreateInfoKHR::builder()
            .surface(surface)
            .min_image_count(image_count)
            .image_format(surface_format.format)
            .image_color_space(surface_format.color_space)
            .image_extent(surface_caps.current_extent)
            .image_array_layers(1)
            .image_usage(ash::vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(ash::vk::SharingMode::EXCLUSIVE)
            .pre_transform(surface_caps.current_transform)
            .composite_alpha(ash::vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true)
            .old_swapchain(ash::vk::SwapchainKHR::null());
        let swapchain = unsafe {
            swapchain_loader
                .create_swapchain(&swapchain_create_info, None)
                .expect("Failed to create swapchain")
        };
        let swapchain_images = unsafe {
            swapchain_loader
                .get_swapchain_images(swapchain)
                .expect("Failed to get swapchain images")
        };

        // Step 4: Create image views for swapchain images
        let image_views: Vec<ash::vk::ImageView> = swapchain_images
            .iter()
            .map(|&image| {
                let create_info = ash::vk::ImageViewCreateInfo::builder()
                    .image(image)
                    .view_type(ash::vk::ImageViewType::TYPE_2D)
                    .format(surface_format.format)
                    .components(ash::vk::ComponentMapping {
                        r: ash::vk::ComponentSwizzle::IDENTITY,
                        g: ash::vk::ComponentSwizzle::IDENTITY,
                        b: ash::vk::ComponentSwizzle::IDENTITY,
                        a: ash::vk::ComponentSwizzle::IDENTITY,
                    })
                    .subresource_range(ash::vk::ImageSubresourceRange {
                        aspect_mask: ash::vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    });
                unsafe {
                    device
                        .create_image_view(&create_info, None)
                        .expect("Failed to create image view")
                }
            })
            .collect();

        // Step 5: Create render pass with depth attachment
        let color_attachment = ash::vk::AttachmentDescription::builder()
            .format(surface_format.format)
            .samples(ash::vk::SampleCountFlags::TYPE_1)
            .load_op(ash::vk::AttachmentLoadOp::CLEAR)
            .store_op(ash::vk::AttachmentStoreOp::STORE)
            .stencil_load_op(ash::vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(ash::vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(ash::vk::ImageLayout::UNDEFINED)
            .final_layout(ash::vk::ImageLayout::PRESENT_SRC_KHR)
            .build();
            
        let depth_attachment = ash::vk::AttachmentDescription::builder()
            .format(ash::vk::Format::D32_SFLOAT)
            .samples(ash::vk::SampleCountFlags::TYPE_1)
            .load_op(ash::vk::AttachmentLoadOp::CLEAR)
            .store_op(ash::vk::AttachmentStoreOp::DONT_CARE)
            .stencil_load_op(ash::vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(ash::vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(ash::vk::ImageLayout::UNDEFINED)
            .final_layout(ash::vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
            .build();
            
        let color_attachment_ref = ash::vk::AttachmentReference::builder()
            .attachment(0)
            .layout(ash::vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .build();
            
        let depth_attachment_ref = ash::vk::AttachmentReference::builder()
            .attachment(1)
            .layout(ash::vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
            .build();
            
        let subpass = ash::vk::SubpassDescription::builder()
            .pipeline_bind_point(ash::vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(std::slice::from_ref(&color_attachment_ref))
            .depth_stencil_attachment(&depth_attachment_ref)
            .build();
            
        let attachments = [color_attachment, depth_attachment];
        let render_pass_info = ash::vk::RenderPassCreateInfo::builder()
            .attachments(&attachments)
            .subpasses(std::slice::from_ref(&subpass));
        let render_pass = unsafe {
            device
                .create_render_pass(&render_pass_info, None)
                .expect("Failed to create render pass")
        };

        // Create depth buffer resources
        let (depth_image, depth_image_memory, depth_image_view) = Self::create_depth_resources(
            &device,
            &instance,
            physical_device,
            surface_caps.current_extent,
        );

        // Step 6: Create framebuffers for each swapchain image view with depth attachment
        let framebuffers: Vec<ash::vk::Framebuffer> = image_views
            .iter()
            .map(|&view| {
                let attachments = [view, depth_image_view];
                let framebuffer_info = ash::vk::FramebufferCreateInfo::builder()
                    .render_pass(render_pass)
                    .attachments(&attachments)
                    .width(surface_caps.current_extent.width)
                    .height(surface_caps.current_extent.height)
                    .layers(1);
                unsafe {
                    device
                        .create_framebuffer(&framebuffer_info, None)
                        .expect("Failed to create framebuffer")
                }
            })
            .collect();

        // Step 8: Load SPIR-V shaders and create shader modules
        use std::fs::File;
        use std::io::Read;
        fn read_spv(path: &str) -> Vec<u32> {
            let mut file = File::open(path).expect("Failed to open shader file");
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes)
                .expect("Failed to read shader file");
            ash::util::read_spv(&mut std::io::Cursor::new(bytes)).expect("Failed to parse SPIR-V")
        }
        let vert_shader_code = read_spv("target/shaders/vert.spv");
        let frag_shader_code = read_spv("target/shaders/frag.spv");
        let vert_shader_module = unsafe {
            device
                .create_shader_module(
                    &ash::vk::ShaderModuleCreateInfo::builder().code(&vert_shader_code),
                    None,
                )
                .expect("Failed to create vertex shader module")
        };
        let frag_shader_module = unsafe {
            device
                .create_shader_module(
                    &ash::vk::ShaderModuleCreateInfo::builder().code(&frag_shader_code),
                    None,
                )
                .expect("Failed to create fragment shader module")
        };

        // Step 9: Create pipeline layout with push constants
        let push_constant_range = ash::vk::PushConstantRange::builder()
            .stage_flags(ash::vk::ShaderStageFlags::VERTEX)
            .offset(0)
            .size(64) // 4x4 matrix = 16 floats = 64 bytes
            .build();
        
        let push_constant_ranges = [push_constant_range];
        let pipeline_layout_info = ash::vk::PipelineLayoutCreateInfo::builder()
            .push_constant_ranges(&push_constant_ranges);
        let pipeline_layout = unsafe {
            device
                .create_pipeline_layout(&pipeline_layout_info, None)
                .expect("Failed to create pipeline layout")
        };

        // Step 10: Create graphics pipeline (minimal, no vertex input)
        let entry_point = std::ffi::CString::new("main").unwrap();
        let shader_stages = [
            ash::vk::PipelineShaderStageCreateInfo::builder()
                .stage(ash::vk::ShaderStageFlags::VERTEX)
                .module(vert_shader_module)
                .name(&entry_point)
                .build(),
            ash::vk::PipelineShaderStageCreateInfo::builder()
                .stage(ash::vk::ShaderStageFlags::FRAGMENT)
                .module(frag_shader_module)
                .name(&entry_point)
                .build(),
        ];
        
        // Define vertex input description
        let binding_descriptions = [ash::vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(std::mem::size_of::<Vertex>() as u32)
            .input_rate(ash::vk::VertexInputRate::VERTEX)
            .build()];
        
        let attribute_descriptions = [
            // Position attribute
            ash::vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(0)
                .format(ash::vk::Format::R32G32B32_SFLOAT)
                .offset(0)
                .build(),
            // Normal attribute
            ash::vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(1)
                .format(ash::vk::Format::R32G32B32_SFLOAT)
                .offset(12) // 3 * sizeof(f32)
                .build(),
            // UV attribute
            ash::vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(2)
                .format(ash::vk::Format::R32G32_SFLOAT)
                .offset(24) // 6 * sizeof(f32)
                .build(),
        ];
        
        let vertex_input_info = ash::vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(&binding_descriptions)
            .vertex_attribute_descriptions(&attribute_descriptions);
        let input_assembly = ash::vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(ash::vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);
        let viewport = ash::vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: surface_caps.current_extent.width as f32,
            height: surface_caps.current_extent.height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        };
        let scissor = ash::vk::Rect2D {
            offset: ash::vk::Offset2D { x: 0, y: 0 },
            extent: surface_caps.current_extent,
        };
        let viewport_state = ash::vk::PipelineViewportStateCreateInfo::builder()
            .viewports(std::slice::from_ref(&viewport))
            .scissors(std::slice::from_ref(&scissor));
        let rasterizer = ash::vk::PipelineRasterizationStateCreateInfo::builder()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(ash::vk::PolygonMode::FILL) // Back to filled mode
            .line_width(1.0)
            .cull_mode(ash::vk::CullModeFlags::NONE) // Keep face culling disabled
            .front_face(ash::vk::FrontFace::COUNTER_CLOCKWISE)
            .depth_bias_enable(false);
        let multisampling = ash::vk::PipelineMultisampleStateCreateInfo::builder()
            .sample_shading_enable(false)
            .rasterization_samples(ash::vk::SampleCountFlags::TYPE_1);
        let color_blend_attachment = ash::vk::PipelineColorBlendAttachmentState {
            color_write_mask: ash::vk::ColorComponentFlags::R
                | ash::vk::ColorComponentFlags::G
                | ash::vk::ColorComponentFlags::B
                | ash::vk::ColorComponentFlags::A,
            blend_enable: ash::vk::FALSE,
            src_color_blend_factor: ash::vk::BlendFactor::ONE,
            dst_color_blend_factor: ash::vk::BlendFactor::ZERO,
            color_blend_op: ash::vk::BlendOp::ADD,
            src_alpha_blend_factor: ash::vk::BlendFactor::ONE,
            dst_alpha_blend_factor: ash::vk::BlendFactor::ZERO,
            alpha_blend_op: ash::vk::BlendOp::ADD,
            ..Default::default()
        };
        let color_blending = ash::vk::PipelineColorBlendStateCreateInfo::builder()
            .attachments(std::slice::from_ref(&color_blend_attachment));
            
        // Enable depth testing with proper depth buffer
        let depth_stencil = ash::vk::PipelineDepthStencilStateCreateInfo::builder()
            .depth_test_enable(true)
            .depth_write_enable(true)
            .depth_compare_op(ash::vk::CompareOp::LESS_OR_EQUAL) // Try less restrictive comparison
            .depth_bounds_test_enable(false)
            .stencil_test_enable(false);
            
        let pipeline_info = ash::vk::GraphicsPipelineCreateInfo::builder()
            .stages(&shader_stages)
            .vertex_input_state(&vertex_input_info)
            .input_assembly_state(&input_assembly)
            .viewport_state(&viewport_state)
            .rasterization_state(&rasterizer)
            .multisample_state(&multisampling)
            .depth_stencil_state(&depth_stencil)  // Now enabled with depth buffer
            .color_blend_state(&color_blending)
            .layout(pipeline_layout)
            .render_pass(render_pass)
            .subpass(0);
        let graphics_pipeline = unsafe {
            device
                .create_graphics_pipelines(
                    ash::vk::PipelineCache::null(),
                    std::slice::from_ref(&pipeline_info),
                    None,
                )
                .expect("Failed to create graphics pipeline")[0]
        };

        // Step 11: Create command pool and allocate command buffers
        let command_pool_info = ash::vk::CommandPoolCreateInfo::builder()
            .queue_family_index(graphics_queue_family_index)
            .flags(ash::vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
        let command_pool = unsafe {
            device
                .create_command_pool(&command_pool_info, None)
                .expect("Failed to create command pool")
        };
        let command_buffer_allocate_info = ash::vk::CommandBufferAllocateInfo::builder()
            .command_pool(command_pool)
            .level(ash::vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(framebuffers.len() as u32);
        let command_buffers = unsafe {
            device
                .allocate_command_buffers(&command_buffer_allocate_info)
                .expect("Failed to allocate command buffers")
        };

        // Step 12: Record command buffers
        for (i, &command_buffer) in command_buffers.iter().enumerate() {
            let begin_info = ash::vk::CommandBufferBeginInfo::builder();
            unsafe {
                device
                    .begin_command_buffer(command_buffer, &begin_info)
                    .expect("Failed to begin command buffer");
                let clear_values = [ash::vk::ClearValue {
                    color: ash::vk::ClearColorValue {
                        float32: [0.1, 0.1, 0.1, 1.0], // Very dark grey background
                    },
                }];
                let render_pass_info = ash::vk::RenderPassBeginInfo::builder()
                    .render_pass(render_pass)
                    .framebuffer(framebuffers[i])
                    .render_area(ash::vk::Rect2D {
                        offset: ash::vk::Offset2D { x: 0, y: 0 },
                        extent: surface_caps.current_extent,
                    })
                    .clear_values(&clear_values);
                device.cmd_begin_render_pass(
                    command_buffer,
                    &render_pass_info,
                    ash::vk::SubpassContents::INLINE,
                );
                device.cmd_bind_pipeline(
                    command_buffer,
                    ash::vk::PipelineBindPoint::GRAPHICS,
                    graphics_pipeline,
                );
                device.cmd_draw(command_buffer, 3, 1, 0, 0);
                device.cmd_end_render_pass(command_buffer);
                device
                    .end_command_buffer(command_buffer)
                    .expect("Failed to end command buffer");
            }
        }

        // Step 13: Create synchronization objects
        let image_available_semaphores: Vec<ash::vk::Semaphore> = (0..framebuffers.len())
            .map(|_| {
                let info = ash::vk::SemaphoreCreateInfo::default();
                unsafe {
                    device
                        .create_semaphore(&info, None)
                        .expect("Failed to create semaphore")
                }
            })
            .collect();
        let render_finished_semaphores: Vec<ash::vk::Semaphore> = (0..framebuffers.len())
            .map(|_| {
                let info = ash::vk::SemaphoreCreateInfo::default();
                unsafe {
                    device
                        .create_semaphore(&info, None)
                        .expect("Failed to create semaphore")
                }
            })
            .collect();
        let in_flight_fences: Vec<ash::vk::Fence> = (0..framebuffers.len())
            .map(|_| {
                let info =
                    ash::vk::FenceCreateInfo::builder().flags(ash::vk::FenceCreateFlags::SIGNALED);
                unsafe {
                    device
                        .create_fence(&info, None)
                        .expect("Failed to create fence")
                }
            })
            .collect();

        VulkanContext {
            entry,
            instance,
            surface_loader,
            surface,
            physical_device,
            device,
            graphics_queue,
            graphics_queue_family_index,
            swapchain_loader,
            swapchain,
            image_views,
            render_pass,
            framebuffers,
            vert_shader_module,
            frag_shader_module,
            pipeline_layout,
            graphics_pipeline,
            command_pool,
            command_buffers,
            image_available_semaphores,
            render_finished_semaphores,
            in_flight_fences,
            current_frame: 0,
            mesh_vertex_count: 3, // Default to triangle
            vertex_buffer: None,
            index_buffer: None,
            mvp_matrix: [
                1.0, 0.0, 0.0, 0.0,
                0.0, 1.0, 0.0, 0.0,
                0.0, 0.0, 1.0, 0.0,
                0.0, 0.0, 0.0, 1.0,
            ], // Identity matrix
            depth_image,
            depth_image_memory,
            depth_image_view,
        }
    }

    pub fn draw_frame(
        &mut self,
        _window: &mut crate::render::window::WindowHandle,
    ) -> Result<(), ash::vk::Result> {
        let framebuffers_len = self.framebuffers.len();
        let device = &self.device;
        let swapchain_loader = &self.swapchain_loader;
        let swapchain = self.swapchain;
        let graphics_queue = self.graphics_queue;
        let command_buffers = &self.command_buffers;
        let image_available_semaphores = &self.image_available_semaphores;
        let render_finished_semaphores = &self.render_finished_semaphores;
        let in_flight_fences = &self.in_flight_fences;
        let current_frame = self.current_frame;

        unsafe {
            device.wait_for_fences(&[in_flight_fences[current_frame]], true, std::u64::MAX)?;
            device.reset_fences(&[in_flight_fences[current_frame]])?;
        }
        let image_index = match unsafe {
            swapchain_loader.acquire_next_image(
                swapchain,
                std::u64::MAX,
                image_available_semaphores[current_frame],
                ash::vk::Fence::null(),
            )
        } {
            Ok((idx, _)) => idx as usize,
            Err(e) => return Err(e),
        };

        // Re-record the command buffer for this frame with current mesh data
        let command_buffer = command_buffers[image_index];
        unsafe {
            device.reset_command_buffer(command_buffer, ash::vk::CommandBufferResetFlags::empty())?;
            
            let begin_info = ash::vk::CommandBufferBeginInfo::builder();
            device.begin_command_buffer(command_buffer, &begin_info)?;
            
            // Get current surface capabilities
            let surface_caps = self.surface_loader
                .get_physical_device_surface_capabilities(self.physical_device, self.surface)
                .expect("Failed to get surface capabilities");
            
            let clear_values = [
                ash::vk::ClearValue {
                    color: ash::vk::ClearColorValue {
                        float32: [0.1, 0.1, 0.1, 1.0], // Very dark grey background
                    },
                },
                ash::vk::ClearValue {
                    depth_stencil: ash::vk::ClearDepthStencilValue {
                        depth: 1.0,
                        stencil: 0,
                    },
                },
            ];
            let render_pass_info = ash::vk::RenderPassBeginInfo::builder()
                .render_pass(self.render_pass)
                .framebuffer(self.framebuffers[image_index])
                .render_area(ash::vk::Rect2D {
                    offset: ash::vk::Offset2D { x: 0, y: 0 },
                    extent: surface_caps.current_extent,
                })
                .clear_values(&clear_values);
            device.cmd_begin_render_pass(
                command_buffer,
                &render_pass_info,
                ash::vk::SubpassContents::INLINE,
            );
            device.cmd_bind_pipeline(
                command_buffer,
                ash::vk::PipelineBindPoint::GRAPHICS,
                self.graphics_pipeline,
            );
            
            // Set viewport dynamically based on current surface capabilities
            let viewport = ash::vk::Viewport {
                x: 0.0,
                y: 0.0,
                width: surface_caps.current_extent.width as f32,
                height: surface_caps.current_extent.height as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            };
            device.cmd_set_viewport(command_buffer, 0, &[viewport]);
            
            // Set scissor rectangle dynamically
            let scissor = ash::vk::Rect2D {
                offset: ash::vk::Offset2D { x: 0, y: 0 },
                extent: surface_caps.current_extent,
            };
            device.cmd_set_scissor(command_buffer, 0, &[scissor]);
            
            // Bind vertex buffer if available
            if let Some(ref vertex_buffer) = self.vertex_buffer {
                let vertex_buffers = [vertex_buffer.buffer];
                let offsets = [0];
                device.cmd_bind_vertex_buffers(command_buffer, 0, &vertex_buffers, &offsets);
            }
            
            // Bind index buffer if available
            if let Some(ref index_buffer) = self.index_buffer {
                device.cmd_bind_index_buffer(command_buffer, index_buffer.buffer, 0, vk::IndexType::UINT32);
            }
            
            // Push constants for MVP matrix
            device.cmd_push_constants(
                command_buffer,
                self.pipeline_layout,
                ash::vk::ShaderStageFlags::VERTEX,
                0,
                bytemuck::cast_slice(&self.mvp_matrix)
            );
            
            // Use indexed rendering if we have an index buffer, otherwise regular draw
            if self.index_buffer.is_some() {
                device.cmd_draw_indexed(command_buffer, self.mesh_vertex_count as u32, 1, 0, 0, 0);
            } else {
                device.cmd_draw(command_buffer, self.mesh_vertex_count as u32, 1, 0, 0);
            }
            device.cmd_end_render_pass(command_buffer);
            device.end_command_buffer(command_buffer)?;
        }

        let wait_semaphores = [image_available_semaphores[current_frame]];
        let signal_semaphores = [render_finished_semaphores[current_frame]];
        let command_buffers_to_submit = [command_buffers[image_index]];
        let submit_info = ash::vk::SubmitInfo::builder()
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&[ash::vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
            .command_buffers(&command_buffers_to_submit)
            .signal_semaphores(&signal_semaphores);

        unsafe {
            device.queue_submit(
                graphics_queue,
                &[submit_info.build()],
                in_flight_fences[current_frame],
            )?;
        }

        let swapchains = [swapchain];
        let image_indices = [image_index as u32];
        let present_info = ash::vk::PresentInfoKHR::builder()
            .wait_semaphores(&signal_semaphores)
            .swapchains(&swapchains)
            .image_indices(&image_indices);
        let present_result =
            unsafe { swapchain_loader.queue_present(graphics_queue, &present_info) };
        match present_result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        self.current_frame = (self.current_frame + 1) % framebuffers_len;
        Ok(())
    }
    
    pub fn set_mesh_data(&mut self, vertices: &[crate::render::mesh::Vertex], indices: &[u32]) -> Result<(), ash::vk::Result> {
        // Clean up any existing buffers
        if let Some(mut buffer) = self.vertex_buffer.take() {
            buffer.cleanup(&self.device);
        }
        if let Some(mut buffer) = self.index_buffer.take() {
            buffer.cleanup(&self.device);
        }
        
        // Create buffer builder
        let buffer_builder = BufferBuilder::new(&self.device, &self.instance, self.physical_device);
        
        // Create vertex buffer
        let vertex_buffer = buffer_builder.create_vertex_buffer(vertices)?;
        self.vertex_buffer = Some(vertex_buffer);
        
        // Create index buffer
        let index_buffer = buffer_builder.create_index_buffer(indices)?;
        self.index_buffer = Some(index_buffer);
        
        // Update vertex count to index count for indexed rendering
        self.mesh_vertex_count = indices.len();
        
        Ok(())
    }

    pub fn set_mvp_matrix(&mut self, mvp_matrix: &crate::util::math::Mat4) {
        // Convert Mat4 to array format for push constants
        self.mvp_matrix = [
            mvp_matrix.m[0][0], mvp_matrix.m[1][0], mvp_matrix.m[2][0], mvp_matrix.m[3][0],
            mvp_matrix.m[0][1], mvp_matrix.m[1][1], mvp_matrix.m[2][1], mvp_matrix.m[3][1],
            mvp_matrix.m[0][2], mvp_matrix.m[1][2], mvp_matrix.m[2][2], mvp_matrix.m[3][2],
            mvp_matrix.m[0][3], mvp_matrix.m[1][3], mvp_matrix.m[2][3], mvp_matrix.m[3][3],
        ];
    }

    pub fn recreate_swapchain(&mut self, _window: &mut crate::render::window::WindowHandle) {
        let device = &self.device;
        let surface_loader = &self.surface_loader;
        let swapchain_loader = &self.swapchain_loader;
        let physical_device = self.physical_device;
        let surface = self.surface;
        let render_pass = self.render_pass;
        let _pipeline_layout = self.pipeline_layout;
        let graphics_pipeline = self.graphics_pipeline;
        let command_pool = self.command_pool;

        unsafe {
            device.device_wait_idle().unwrap();
        }

        // 1. Save the old swapchain handle
        let old_swapchain = self.swapchain;

        // 2. Query surface and swapchain info
        let surface_caps = unsafe {
            surface_loader
                .get_physical_device_surface_capabilities(physical_device, surface)
                .expect("Failed to get surface capabilities")
        };
        let surface_formats = unsafe {
            surface_loader
                .get_physical_device_surface_formats(physical_device, surface)
                .expect("Failed to get surface formats")
        };
        let surface_format = surface_formats
            .iter()
            .find(|sf| sf.format == ash::vk::Format::B8G8R8A8_SRGB)
            .cloned()
            .unwrap_or(surface_formats[0]);
        let present_modes = unsafe {
            surface_loader
                .get_physical_device_surface_present_modes(physical_device, surface)
                .expect("Failed to get present modes")
        };
        let present_mode = present_modes
            .iter()
            .cloned()
            .find(|&mode| mode == ash::vk::PresentModeKHR::MAILBOX)
            .unwrap_or(ash::vk::PresentModeKHR::FIFO);
        let image_count = surface_caps.min_image_count + 1;
        let image_count =
            if surface_caps.max_image_count > 0 && image_count > surface_caps.max_image_count {
                surface_caps.max_image_count
            } else {
                image_count
            };

        // 3. Create the new swapchain, passing the old one
        let swapchain_create_info = ash::vk::SwapchainCreateInfoKHR::builder()
            .surface(surface)
            .min_image_count(image_count)
            .image_format(surface_format.format)
            .image_color_space(surface_format.color_space)
            .image_extent(surface_caps.current_extent)
            .image_array_layers(1)
            .image_usage(ash::vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(ash::vk::SharingMode::EXCLUSIVE)
            .pre_transform(surface_caps.current_transform)
            .composite_alpha(ash::vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true)
            .old_swapchain(old_swapchain);
        let swapchain = unsafe {
            swapchain_loader
                .create_swapchain(&swapchain_create_info, None)
                .expect("Failed to create swapchain")
        };

        // 4. Now destroy old resources
        for &fb in &self.framebuffers {
            unsafe {
                device.destroy_framebuffer(fb, None);
            }
        }
        for &view in &self.image_views {
            unsafe {
                device.destroy_image_view(view, None);
            }
        }
        // Clean up old depth buffer resources
        unsafe {
            device.destroy_image_view(self.depth_image_view, None);
            device.destroy_image(self.depth_image, None);
            device.free_memory(self.depth_image_memory, None);
            swapchain_loader.destroy_swapchain(old_swapchain, None);
        }

        // 5. Continue as before
        let swapchain_images = unsafe {
            swapchain_loader
                .get_swapchain_images(swapchain)
                .expect("Failed to get swapchain images")
        };
        let image_views: Vec<ash::vk::ImageView> = swapchain_images
            .iter()
            .map(|&image| {
                let create_info = ash::vk::ImageViewCreateInfo::builder()
                    .image(image)
                    .view_type(ash::vk::ImageViewType::TYPE_2D)
                    .format(surface_format.format)
                    .components(ash::vk::ComponentMapping {
                        r: ash::vk::ComponentSwizzle::IDENTITY,
                        g: ash::vk::ComponentSwizzle::IDENTITY,
                        b: ash::vk::ComponentSwizzle::IDENTITY,
                        a: ash::vk::ComponentSwizzle::IDENTITY,
                    })
                    .subresource_range(ash::vk::ImageSubresourceRange {
                        aspect_mask: ash::vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    });
                unsafe {
                    device
                        .create_image_view(&create_info, None)
                        .expect("Failed to create image view")
                }
            })
            .collect();
            
        // Recreate depth buffer with new surface capabilities
        let (depth_image, depth_image_memory, depth_image_view) = Self::create_depth_resources(
            &device,
            &self.instance,
            physical_device,
            surface_caps.current_extent,
        );
        
        let framebuffers: Vec<ash::vk::Framebuffer> = image_views
            .iter()
            .map(|&view| {
                let attachments = [view, depth_image_view];
                let framebuffer_info = ash::vk::FramebufferCreateInfo::builder()
                    .render_pass(render_pass)
                    .attachments(&attachments)
                    .width(surface_caps.current_extent.width)
                    .height(surface_caps.current_extent.height)
                    .layers(1);
                unsafe {
                    device
                        .create_framebuffer(&framebuffer_info, None)
                        .expect("Failed to create framebuffer")
                }
            })
            .collect();
        unsafe {
            device
                .reset_command_pool(command_pool, ash::vk::CommandPoolResetFlags::empty())
                .unwrap();
        }
        let command_buffer_allocate_info = ash::vk::CommandBufferAllocateInfo::builder()
            .command_pool(command_pool)
            .level(ash::vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(framebuffers.len() as u32);
        let command_buffers = unsafe {
            device
                .allocate_command_buffers(&command_buffer_allocate_info)
                .expect("Failed to allocate command buffers")
        };
        for (i, &command_buffer) in command_buffers.iter().enumerate() {
            let begin_info = ash::vk::CommandBufferBeginInfo::builder();
            unsafe {
                device
                    .begin_command_buffer(command_buffer, &begin_info)
                    .expect("Failed to begin command buffer");
                let clear_values = [ash::vk::ClearValue {
                    color: ash::vk::ClearColorValue {
                        float32: [0.1, 0.1, 0.1, 1.0], // Very dark grey background
                    },
                }];
                let render_pass_info = ash::vk::RenderPassBeginInfo::builder()
                    .render_pass(render_pass)
                    .framebuffer(framebuffers[i])
                    .render_area(ash::vk::Rect2D {
                        offset: ash::vk::Offset2D { x: 0, y: 0 },
                        extent: surface_caps.current_extent,
                    })
                    .clear_values(&clear_values);
                device.cmd_begin_render_pass(
                    command_buffer,
                    &render_pass_info,
                    ash::vk::SubpassContents::INLINE,
                );
                device.cmd_bind_pipeline(
                    command_buffer,
                    ash::vk::PipelineBindPoint::GRAPHICS,
                    graphics_pipeline,
                );
                device.cmd_draw(command_buffer, 3, 1, 0, 0);
                device.cmd_end_render_pass(command_buffer);
                device
                    .end_command_buffer(command_buffer)
                    .expect("Failed to end command buffer");
            }
        }
        let image_available_semaphores: Vec<ash::vk::Semaphore> = (0..framebuffers.len())
            .map(|_| {
                let info = ash::vk::SemaphoreCreateInfo::default();
                unsafe {
                    device
                        .create_semaphore(&info, None)
                        .expect("Failed to create semaphore")
                }
            })
            .collect();
        let render_finished_semaphores: Vec<ash::vk::Semaphore> = (0..framebuffers.len())
            .map(|_| {
                let info = ash::vk::SemaphoreCreateInfo::default();
                unsafe {
                    device
                        .create_semaphore(&info, None)
                        .expect("Failed to create semaphore")
                }
            })
            .collect();
        let in_flight_fences: Vec<ash::vk::Fence> = (0..framebuffers.len())
            .map(|_| {
                let info =
                    ash::vk::FenceCreateInfo::builder().flags(ash::vk::FenceCreateFlags::SIGNALED);
                unsafe {
                    device
                        .create_fence(&info, None)
                        .expect("Failed to create fence")
                }
            })
            .collect();

        self.swapchain = swapchain;
        self.image_views = image_views;
        self.framebuffers = framebuffers;
        self.command_buffers = command_buffers;
        self.image_available_semaphores = image_available_semaphores;
        self.render_finished_semaphores = render_finished_semaphores;
        self.in_flight_fences = in_flight_fences;
        self.depth_image = depth_image;
        self.depth_image_memory = depth_image_memory;
        self.depth_image_view = depth_image_view;
        self.current_frame = 0;
    }

    pub fn cleanup(&mut self) {
        let device = &self.device;
        unsafe {
            for &fence in &self.in_flight_fences {
                device.destroy_fence(fence, None);
            }
            for &sem in &self.image_available_semaphores {
                device.destroy_semaphore(sem, None);
            }
            for &sem in &self.render_finished_semaphores {
                device.destroy_semaphore(sem, None);
            }
            device.destroy_command_pool(self.command_pool, None);
            for &fb in &self.framebuffers {
                device.destroy_framebuffer(fb, None);
            }
            device.destroy_pipeline(self.graphics_pipeline, None);
            device.destroy_pipeline_layout(self.pipeline_layout, None);
            device.destroy_shader_module(self.vert_shader_module, None);
            device.destroy_shader_module(self.frag_shader_module, None);
            device.destroy_render_pass(self.render_pass, None);
            // Clean up depth buffer resources
            device.destroy_image_view(self.depth_image_view, None);
            device.destroy_image(self.depth_image, None);
            device.free_memory(self.depth_image_memory, None);
            for &view in &self.image_views {
                device.destroy_image_view(view, None);
            }
            device.destroy_device(None);
            self.surface_loader.destroy_surface(self.surface, None);
            self.instance.destroy_instance(None);
        }
    }
}
