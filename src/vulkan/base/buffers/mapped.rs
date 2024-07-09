use crate::{new_buffer, Device};
use anyhow::{anyhow, Result};
use ash::{
    util::Align,
    vk::{self}
};
use std::{
    mem::{align_of, size_of_val},
    ops::Deref,
    ptr::NonNull
};

/// Wraps a Vulkan buffer. This version does not use a staging buffer
/// but instead directly maps host-visible coherent memory. Use this
/// for things like uniform buffers that are small.
pub struct MappedBuffer<T> {
    /// The buffer.
    buffer: vk::Buffer,

    /// The memory.
    memory: vk::DeviceMemory,

    /// The memory size.
    memory_size: vk::DeviceSize,

    /// The raw memory.
    ptr: NonNull<T>,

    /// The size of the data in bytes.
    size: vk::DeviceSize
}

impl<T: Copy> MappedBuffer<T> {
    pub unsafe fn new(device: &Device, usage: vk::BufferUsageFlags, data: &[T]) -> Result<Self> {
        // Compute the size of the buffer in bytes.
        let size = size_of_val(data) as vk::DeviceSize;

        // Create the buffer.
        let (buffer, memory, memory_size) = new_buffer(
            device,
            size,
            usage,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT
        )?;

        // Map the memory and grab a raw pointer.
        let ptr = device.map_memory(memory, 0, memory_size, vk::MemoryMapFlags::empty())?;
        let ptr = NonNull::new_unchecked(ptr.cast());

        // Create the host buffer.
        let mut this = Self {
            buffer,
            memory,
            memory_size,
            ptr,
            size
        };

        // Write the data to the memory.
        this.overwrite(data)?;

        Ok(this)
    }

    /// Overwrite the buffer with new data.
    pub unsafe fn overwrite(&mut self, data: &[T]) -> Result<()> {
        if size_of_val(data) as vk::DeviceSize != self.size {
            return Err(anyhow!("Size must match when overwriting buffer."));
        }

        // Get an aligned view into the memory.
        let mut aligned = Align::new(
            self.ptr.as_ptr().cast(),
            align_of::<T>() as vk::DeviceSize,
            self.memory_size
        );

        // Copy the data to the memory.
        aligned.copy_from_slice(data);

        Ok(())
    }

    /// Destroy the buffer.
    pub unsafe fn destroy(&self, device: &Device) {
        // Unmap the memory.
        device.unmap_memory(self.memory);

        // Destroy the buffer.
        device.destroy_buffer(self.buffer, None);

        // Free the memory.
        device.free_memory(self.memory, None);
    }
}

impl<T> Deref for MappedBuffer<T> {
    type Target = vk::Buffer;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}
