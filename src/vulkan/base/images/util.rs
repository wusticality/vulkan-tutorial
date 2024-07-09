use crate::{find_memory_type, Device};
use anyhow::Result;
use ash::vk;

/// The image settings.
pub struct ImageSettings {
    /// The image format.
    pub format: vk::Format,

    /// The usage flags.
    pub usage: vk::ImageUsageFlags,

    /// The multisampling flags.
    pub samples: vk::SampleCountFlags
}

/// Create an internal image.
pub unsafe fn new_image(
    device: &Device,
    settings: &ImageSettings,
    size: &vk::Extent3D,
    memory_properties: vk::MemoryPropertyFlags
) -> Result<(vk::Image, vk::DeviceMemory, vk::DeviceSize)> {
    // Make sure the image is a transfer destination.
    let usage = settings.usage | vk::ImageUsageFlags::TRANSFER_DST;

    // Create the image info.
    let image_info = vk::ImageCreateInfo::default()
        .image_type(vk::ImageType::TYPE_2D)
        .extent(*size)
        .mip_levels(1)
        .array_layers(1)
        .format(settings.format)
        .tiling(vk::ImageTiling::OPTIMAL)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .usage(usage)
        .sharing_mode(vk::SharingMode::EXCLUSIVE)
        .samples(settings.samples);

    // Create the image.
    let image = device.create_image(&image_info, None)?;

    // Get the image's memory requirements.
    let memory_requirements = device.get_image_memory_requirements(image);

    // Find a suitable memory type.
    let memory_index = find_memory_type(device, &memory_requirements, memory_properties)?;

    // Create the memory allocation info.
    let memory_info = vk::MemoryAllocateInfo::default()
        .allocation_size(memory_requirements.size)
        .memory_type_index(memory_index);

    // Allocate the memory.
    let memory = device.allocate_memory(&memory_info, None)?;

    // Bind the memory to the image.
    device.bind_image_memory(image, memory, 0)?;

    Ok((image, memory, memory_requirements.size))
}
