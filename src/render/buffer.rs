// src/render/buffer.rs
use ash::vk;
use std::mem;

pub struct Buffer {
    pub buffer: vk::Buffer,
    pub memory: vk::DeviceMemory,
    pub size: vk::DeviceSize,
}

pub struct BufferBuilder<'a> {
    device: &'a ash::Device,
    instance: &'a ash::Instance,
    physical_device: vk::PhysicalDevice,
}

impl<'a> BufferBuilder<'a> {
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

    pub fn create_buffer(
        &self,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        properties: vk::MemoryPropertyFlags,
    ) -> Result<Buffer, vk::Result> {
        // Create buffer
        let buffer_info = vk::BufferCreateInfo::builder()
            .size(size)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let buffer = unsafe {
            self.device.create_buffer(&buffer_info, None)?
        };

        // Get memory requirements
        let mem_requirements = unsafe {
            self.device.get_buffer_memory_requirements(buffer)
        };

        // Find suitable memory type
        let memory_type = self.find_memory_type(mem_requirements.memory_type_bits, properties)?;

        // Allocate memory
        let alloc_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(mem_requirements.size)
            .memory_type_index(memory_type);

        let memory = unsafe {
            self.device.allocate_memory(&alloc_info, None)?
        };

        // Bind buffer memory
        unsafe {
            self.device.bind_buffer_memory(buffer, memory, 0)?;
        }

        Ok(Buffer {
            buffer,
            memory,
            size,
        })
    }

    pub fn create_vertex_buffer<T>(&self, vertices: &[T]) -> Result<Buffer, vk::Result> {
        let size = (mem::size_of::<T>() * vertices.len()) as vk::DeviceSize;
        
        // Create staging buffer
        let staging_buffer = self.create_buffer(
            size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        // Copy data to staging buffer
        unsafe {
            let data_ptr = self.device.map_memory(
                staging_buffer.memory,
                0,
                size,
                vk::MemoryMapFlags::empty(),
            )? as *mut T;
            data_ptr.copy_from_nonoverlapping(vertices.as_ptr(), vertices.len());
            self.device.unmap_memory(staging_buffer.memory);
        }

        // Create vertex buffer
        let vertex_buffer = self.create_buffer(
            size,
            vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        // Copy from staging to vertex buffer (this would need command buffer support)
        // For now, we'll return the staging buffer as a simplified implementation
        // TODO: Implement proper buffer copying with command buffers
        
        Ok(staging_buffer)
    }

    pub fn create_index_buffer(&self, indices: &[u32]) -> Result<Buffer, vk::Result> {
        let size = (mem::size_of::<u32>() * indices.len()) as vk::DeviceSize;
        
        // Create staging buffer
        let staging_buffer = self.create_buffer(
            size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        // Copy data to staging buffer
        unsafe {
            let data_ptr = self.device.map_memory(
                staging_buffer.memory,
                0,
                size,
                vk::MemoryMapFlags::empty(),
            )? as *mut u32;
            data_ptr.copy_from_nonoverlapping(indices.as_ptr(), indices.len());
            self.device.unmap_memory(staging_buffer.memory);
        }

        // For simplified implementation, return staging buffer
        // TODO: Implement proper device-local index buffer
        Ok(staging_buffer)
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

impl Buffer {
    pub fn cleanup(&mut self, device: &ash::Device) {
        unsafe {
            device.destroy_buffer(self.buffer, None);
            device.free_memory(self.memory, None);
        }
    }
}
