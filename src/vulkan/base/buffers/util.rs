use crate::Device;
use anyhow::{anyhow, Result};
use ash::vk;

/// Create an internal buffer.
pub unsafe fn new_buffer(
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
    let memory_index = find_memory_type(device, &memory_requirements, memory_properties)?;

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
pub unsafe fn find_memory_type(
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
