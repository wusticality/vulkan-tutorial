use crate::Device;
use anyhow::{anyhow, Result};
use ash::{
    util::Align,
    vk::{self}
};
use std::{
    mem::{align_of, size_of},
    ops::Deref
};

/// Wraps a Vulkan buffer.
pub struct Buffer {
    /// The buffer.
    buffer: vk::Buffer,

    /// The memory.
    memory: vk::DeviceMemory
}

impl Buffer {
    pub unsafe fn new<T: Copy>(
        device: &Device,
        usage: vk::BufferUsageFlags,
        data: &[T]
    ) -> Result<Self> {
        // Compute the size of the buffer in bytes.
        let size = size_of::<T>() * data.len();
        let size = size as vk::DeviceSize;

        // Create the src buffer.
        let (src_buffer, src_memory, src_memory_size) = Self::new_buffer(
            device,
            size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT
        )?;

        // Copy data to the src buffer.
        {
            // Map the memory so we can write to it.
            let ptr =
                device.map_memory(src_memory, 0, src_memory_size, vk::MemoryMapFlags::empty())?;

            // Get an aligned view into the memory.
            let mut aligned = Align::new(
                ptr,
                align_of::<T>() as vk::DeviceSize,
                src_memory_size as vk::DeviceSize
            );

            // Copy the data to the memory.
            aligned.copy_from_slice(data);

            // Unmap the memory.
            device.unmap_memory(src_memory);
        }

        // Create the dst buffer.
        let (dst_buffer, dst_memory, _dst_memory_size) = Self::new_buffer(
            device,
            size,
            usage | vk::BufferUsageFlags::TRANSFER_DST,
            vk::MemoryPropertyFlags::DEVICE_LOCAL
        )?;

        // Issue the command to copy the buffer.
        device.one_time_command(|command_buffer| {
            // Copy the staging buffer to the gpu.
            device.cmd_copy_buffer(
                command_buffer,
                src_buffer,
                dst_buffer,
                &[vk::BufferCopy {
                    src_offset: 0,
                    dst_offset: 0,
                    size
                }]
            );

            Ok(())
        })?;

        // Destroy the src buffer.
        device.destroy_buffer(src_buffer, None);

        // Free the src memory.
        device.free_memory(src_memory, None);

        Ok(Self {
            buffer: dst_buffer,
            memory: dst_memory
        })
    }

    /// Create an internal buffer.
    unsafe fn new_buffer(
        device: &Device,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        memory_properties: vk::MemoryPropertyFlags
    ) -> Result<(vk::Buffer, vk::DeviceMemory, vk::DeviceSize)> {
        // Create the buffer info.
        let buffer_info = vk::BufferCreateInfo::default()
            .size(size)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        // Create the buffer.
        let buffer = device.create_buffer(&buffer_info, None)?;

        // Get the buffer's memory requirements.
        let memory_requirements = device.get_buffer_memory_requirements(buffer);

        // Find a suitable memory type.
        let memory_index = Self::find_memory_type(device, &memory_requirements, memory_properties)?;

        // Create the memory allocation info.
        let memory_info = vk::MemoryAllocateInfo::default()
            .allocation_size(memory_requirements.size)
            .memory_type_index(memory_index);

        // Allocate the memory.
        let memory = device.allocate_memory(&memory_info, None)?;

        // Bind the memory to the buffer.
        device.bind_buffer_memory(buffer, memory, 0)?;

        Ok((buffer, memory, memory_requirements.size))
    }

    /// Find a usable memory type.
    unsafe fn find_memory_type(
        device: &Device,
        memory_requirements: &vk::MemoryRequirements,
        properties: vk::MemoryPropertyFlags
    ) -> Result<u32> {
        // Get the memory properties.
        let memory_properties = device.memory_properties();

        // Find a memory type.
        for i in 0..memory_properties.memory_type_count {
            if memory_requirements.memory_type_bits & (1 << i) != 0
                && memory_properties.memory_types[i as usize]
                    .property_flags
                    .contains(properties)
            {
                return Ok(i);
            }
        }

        Err(anyhow!("Failed to find a suitable memory type!"))
    }

    /// Destroy the buffer.
    pub unsafe fn destroy(&self, device: &Device) {
        // Destroy the buffer.
        device.destroy_buffer(self.buffer, None);

        // Free the memory.
        device.free_memory(self.memory, None);
    }
}

impl Deref for Buffer {
    type Target = vk::Buffer;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}
