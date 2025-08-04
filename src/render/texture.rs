// src/render/texture.rs
use ash::vk;
use std::path::Path;

pub struct Texture {
    pub image: vk::Image,
    pub memory: vk::DeviceMemory,
    pub view: vk::ImageView,
    pub sampler: vk::Sampler,
    pub width: u32,
    pub height: u32,
    pub format: vk::Format,
}

pub struct TextureBuilder<'a> {
    device: &'a ash::Device,
    instance: &'a ash::Instance,
    physical_device: vk::PhysicalDevice,
}

impl<'a> TextureBuilder<'a> {
    pub fn new(
        device: &'a ash::Device,
        instance: &'a ash::Instance,
        physical_device: vk::PhysicalDevice,
    ) -> Self {
        Self {
            device,
            instance,
            physical_device,
        }
    }

    pub fn load_from_file<P: AsRef<Path>>(&self, path: P) -> Result<Texture, Box<dyn std::error::Error>> {
        // For now, create a simple placeholder texture
        // TODO: Implement actual image loading (using image crate or similar)
        self.create_solid_color_texture([255, 255, 255, 255]) // White texture
    }

    pub fn create_solid_color_texture(&self, color: [u8; 4]) -> Result<Texture, Box<dyn std::error::Error>> {
        let width = 1;
        let height = 1;
        let format = vk::Format::R8G8B8A8_SRGB;
        
        // Create image
        let image_info = vk::ImageCreateInfo::builder()
            .image_type(vk::ImageType::TYPE_2D)
            .extent(vk::Extent3D { width, height, depth: 1 })
            .mip_levels(1)
            .array_layers(1)
            .format(format)
            .tiling(vk::ImageTiling::OPTIMAL)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .usage(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .samples(vk::SampleCountFlags::TYPE_1);

        let image = unsafe {
            self.device.create_image(&image_info, None)?
        };

        // Allocate memory for image
        let mem_requirements = unsafe {
            self.device.get_image_memory_requirements(image)
        };

        let memory_type = self.find_memory_type(
            mem_requirements.memory_type_bits,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        let alloc_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(mem_requirements.size)
            .memory_type_index(memory_type);

        let memory = unsafe {
            self.device.allocate_memory(&alloc_info, None)?
        };

        unsafe {
            self.device.bind_image_memory(image, memory, 0)?;
        }

        // Create image view
        let view_info = vk::ImageViewCreateInfo::builder()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(format)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });

        let view = unsafe {
            self.device.create_image_view(&view_info, None)?
        };

        // Create sampler
        let sampler_info = vk::SamplerCreateInfo::builder()
            .mag_filter(vk::Filter::LINEAR)
            .min_filter(vk::Filter::LINEAR)
            .address_mode_u(vk::SamplerAddressMode::REPEAT)
            .address_mode_v(vk::SamplerAddressMode::REPEAT)
            .address_mode_w(vk::SamplerAddressMode::REPEAT)
            .anisotropy_enable(false)
            .max_anisotropy(1.0)
            .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
            .unnormalized_coordinates(false)
            .compare_enable(false)
            .compare_op(vk::CompareOp::ALWAYS)
            .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
            .mip_lod_bias(0.0)
            .min_lod(0.0)
            .max_lod(0.0);

        let sampler = unsafe {
            self.device.create_sampler(&sampler_info, None)?
        };

        Ok(Texture {
            image,
            memory,
            view,
            sampler,
            width,
            height,
            format,
        })
    }

    fn find_memory_type(
        &self,
        type_filter: u32,
        properties: vk::MemoryPropertyFlags,
    ) -> Result<u32, vk::Result> {
        let mem_properties = unsafe {
            self.instance.get_physical_device_memory_properties(self.physical_device)
        };

        for i in 0..mem_properties.memory_type_count {
            if (type_filter & (1 << i)) != 0
                && mem_properties.memory_types[i as usize]
                    .property_flags
                    .contains(properties)
            {
                return Ok(i);
            }
        }

        Err(vk::Result::ERROR_FEATURE_NOT_PRESENT)
    }
}

impl Texture {
    pub fn cleanup(&mut self, device: &ash::Device) {
        unsafe {
            device.destroy_sampler(self.sampler, None);
            device.destroy_image_view(self.view, None);
            device.destroy_image(self.image, None);
            device.free_memory(self.memory, None);
        }
    }
}
