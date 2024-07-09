use crate::{new_buffer, Device};
use anyhow::Result;
use ash::{
    util::Align,
    vk::{self}
};
use std::{
    mem::{align_of, size_of_val},
    ops::Deref
};

/// Wraps a Vulkan buffer. This version uses a staging buffer to
/// directly upload data to the GPU exactly once. No CPU-side
/// buffer is kept around for copying. Use this for data like
/// meshes that never change and should be uploaded once.
pub struct ImmutableBuffer {
    /// The buffer.
    buffer: vk::Buffer,

    /// The memory.
    memory: vk::DeviceMemory
}

impl ImmutableBuffer {
    pub unsafe fn new<T: Copy>(
        device: &Device,
        usage: vk::BufferUsageFlags,
        data: &[T]
    ) -> Result<Self> {
        // Compute the size of the buffer in bytes.
        let size = size_of_val(data) as vk::DeviceSize;

        // Create the src buffer.
        let (src_buffer, src_memory, src_memory_size) = new_buffer(
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
        let (dst_buffer, dst_memory, _dst_memory_size) = new_buffer(
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

    /// Destroy the buffer.
    pub unsafe fn destroy(&self, device: &Device) {
        // Destroy the buffer.
        device.destroy_buffer(self.buffer, None);

        // Free the memory.
        device.free_memory(self.memory, None);
    }
}

impl Deref for ImmutableBuffer {
    type Target = vk::Buffer;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}
