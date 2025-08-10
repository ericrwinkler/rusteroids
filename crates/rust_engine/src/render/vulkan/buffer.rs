//! Buffer management for vertex data and uniforms
//! 
//! Memory management following RAII patterns with proper allocation and cleanup

use ash::{vk, Device, Instance};
use std::mem;
use crate::render::vulkan::{VulkanResult, VulkanError};

/// Buffer wrapper with memory management
pub struct Buffer {
    device: Device,
    buffer: vk::Buffer,
    memory: vk::DeviceMemory,
    size: vk::DeviceSize,
}

impl Buffer {
    /// Create a new buffer with memory allocation
    pub fn new(
        device: Device,
        instance: Instance,
        physical_device: vk::PhysicalDevice,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        properties: vk::MemoryPropertyFlags,
    ) -> VulkanResult<Self> {
        // Create buffer
        let buffer_info = vk::BufferCreateInfo::builder()
            .size(size)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
            
        let buffer = unsafe {
            device.create_buffer(&buffer_info, None)
                .map_err(VulkanError::Api)?
        };
        
        // Get memory requirements
        let mem_requirements = unsafe {
            device.get_buffer_memory_requirements(buffer)
        };
        
        // Find memory type
        let memory_type_index = find_memory_type(
            &instance,
            physical_device,
            mem_requirements.memory_type_bits,
            properties
        )?;
        
        // Allocate memory
        let alloc_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(mem_requirements.size)
            .memory_type_index(memory_type_index);
            
        let memory = unsafe {
            device.allocate_memory(&alloc_info, None)
                .map_err(VulkanError::Api)?
        };
        
        // Bind buffer to memory
        unsafe {
            device.bind_buffer_memory(buffer, memory, 0)
                .map_err(VulkanError::Api)?;
        }
        
        Ok(Self {
            device,
            buffer,
            memory,
            size,
        })
    }
    
    /// Map memory for writing
    pub fn map_memory(&self) -> VulkanResult<*mut std::ffi::c_void> {
        unsafe {
            self.device.map_memory(
                self.memory,
                0,
                self.size,
                vk::MemoryMapFlags::empty()
            ).map_err(VulkanError::Api)
        }
    }
    
    /// Unmap memory
    pub fn unmap_memory(&self) {
        unsafe {
            self.device.unmap_memory(self.memory);
        }
    }
    
    /// Write data to buffer
    pub fn write_data<T>(&self, data: &[T]) -> VulkanResult<()> {
        let data_ptr = self.map_memory()?;
        
        unsafe {
            let src_ptr = data.as_ptr() as *const std::ffi::c_void;
            let size = (data.len() * mem::size_of::<T>()) as usize;
            std::ptr::copy_nonoverlapping(src_ptr, data_ptr, size);
        }
        
        self.unmap_memory();
        Ok(())
    }
    
    /// Get buffer handle
    pub fn handle(&self) -> vk::Buffer {
        self.buffer
    }
    
    /// Get size
    pub fn size(&self) -> vk::DeviceSize {
        self.size
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_buffer(self.buffer, None);
            self.device.free_memory(self.memory, None);
        }
    }
}

/// Vertex buffer specifically for vertex data
pub struct VertexBuffer {
    buffer: Buffer,
}

impl VertexBuffer {
    /// Create vertex buffer with vertex data
    pub fn new<T>(
        device: Device,
        instance: Instance,
        physical_device: vk::PhysicalDevice,
        vertices: &[T],
    ) -> VulkanResult<Self> {
        let size = (vertices.len() * mem::size_of::<T>()) as vk::DeviceSize;
        
        let buffer = Buffer::new(
            device,
            instance,
            physical_device,
            size,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;
        
        buffer.write_data(vertices)?;
        
        Ok(Self { buffer })
    }
    
    /// Get buffer handle
    pub fn handle(&self) -> vk::Buffer {
        self.buffer.handle()
    }
    
    /// Get size
    pub fn size(&self) -> vk::DeviceSize {
        self.buffer.size()
    }
}

/// Index buffer for index data
pub struct IndexBuffer {
    buffer: Buffer,
    index_count: u32,
}

impl IndexBuffer {
    /// Create index buffer with index data
    pub fn new(
        device: Device,
        instance: Instance,
        physical_device: vk::PhysicalDevice,
        indices: &[u32],
    ) -> VulkanResult<Self> {
        let size = (indices.len() * mem::size_of::<u32>()) as vk::DeviceSize;
        
        let buffer = Buffer::new(
            device,
            instance,
            physical_device,
            size,
            vk::BufferUsageFlags::INDEX_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;
        
        buffer.write_data(indices)?;
        
        Ok(Self { 
            buffer,
            index_count: indices.len() as u32,
        })
    }
    
    /// Get buffer handle
    pub fn handle(&self) -> vk::Buffer {
        self.buffer.handle()
    }
    
    /// Get index count
    pub fn index_count(&self) -> u32 {
        self.index_count
    }
}

/// Uniform buffer for shader uniforms
pub struct UniformBuffer<T> {
    buffer: Buffer,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> UniformBuffer<T> {
    /// Create uniform buffer
    pub fn new(
        device: Device,
        instance: Instance,
        physical_device: vk::PhysicalDevice,
    ) -> VulkanResult<Self> {
        let size = mem::size_of::<T>() as vk::DeviceSize;
        
        let buffer = Buffer::new(
            device,
            instance,
            physical_device,
            size,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;
        
        Ok(Self { 
            buffer,
            _phantom: std::marker::PhantomData,
        })
    }
    
    /// Update uniform data
    pub fn update(&self, data: &T) -> VulkanResult<()> {
        self.buffer.write_data(std::slice::from_ref(data))
    }
    
    /// Get buffer handle
    pub fn handle(&self) -> vk::Buffer {
        self.buffer.handle()
    }
}

/// Find memory type with required properties
fn find_memory_type(
    instance: &Instance,
    physical_device: vk::PhysicalDevice,
    type_filter: u32,
    properties: vk::MemoryPropertyFlags,
) -> VulkanResult<u32> {
    let mem_properties = unsafe {
        instance.get_physical_device_memory_properties(physical_device)
    };
    
    for i in 0..mem_properties.memory_type_count {
        if (type_filter & (1 << i)) != 0 
            && (mem_properties.memory_types[i as usize].property_flags & properties) == properties 
        {
            return Ok(i);
        }
    }
    
    Err(VulkanError::NoSuitableMemoryType)
}
