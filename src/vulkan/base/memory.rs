use crate::Device;
use anyhow::{anyhow, Result};
use ash::vk;

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
