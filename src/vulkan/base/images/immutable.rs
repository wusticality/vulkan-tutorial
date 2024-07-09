use crate::{new_image, Device, ImageSettings, MappedBuffer};
use anyhow::Result;
use ash::vk;
use image::io::Reader;
use std::{ops::Deref, path::Path};

/// Wraps a Vulkan image. This version uses a staging buffer to
/// directly upload data to the GPU exactly once. No CPU-side
/// buffer is kept around for copying. Use this for data like
/// textures that never change and should be uploaded once.
pub struct ImmutableImage {
    /// The image.
    image: vk::Image,

    /// The memory.
    memory: vk::DeviceMemory,

    /// The image view.
    view: vk::ImageView
}

impl ImmutableImage {
    /// Create a new image from raw data.
    pub unsafe fn new(
        device: &Device,
        settings: &ImageSettings,
        data: &[u8],
        size: &vk::Extent2D
    ) -> Result<Self> {
        // We need a 3D size.
        let size = vk::Extent3D {
            width:  size.width,
            height: size.height,
            depth:  1
        };

        // Create the src buffer.
        let src = MappedBuffer::new(device, vk::BufferUsageFlags::TRANSFER_SRC, data)?;

        // Create the dst image.
        let (image, memory, _memory_size) = new_image(
            device,
            settings,
            &size,
            vk::MemoryPropertyFlags::DEVICE_LOCAL
        )?;

        // Issue the command to copy the image.
        device.one_time_command(|command_buffer| {
            // Prepare the image for transfer.
            device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::TRANSFER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[vk::ImageMemoryBarrier::default()
                    .old_layout(vk::ImageLayout::UNDEFINED)
                    .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                    .src_access_mask(vk::AccessFlags::empty())
                    .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                    .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                    .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                    .image(image)
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask:      vk::ImageAspectFlags::COLOR,
                        base_mip_level:   0,
                        level_count:      1,
                        base_array_layer: 0,
                        layer_count:      1
                    })]
            );

            // Copy the buffer to the image.
            device.cmd_copy_buffer_to_image(
                command_buffer,
                *src,
                image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[vk::BufferImageCopy::default()
                    .image_subresource(vk::ImageSubresourceLayers {
                        aspect_mask:      vk::ImageAspectFlags::COLOR,
                        mip_level:        0,
                        base_array_layer: 0,
                        layer_count:      1
                    })
                    .image_extent(size)]
            );

            // Prepare the image for shader reads.
            device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[vk::ImageMemoryBarrier::default()
                    .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                    .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                    .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                    .dst_access_mask(vk::AccessFlags::SHADER_READ)
                    .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                    .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                    .image(image)
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask:      vk::ImageAspectFlags::COLOR,
                        base_mip_level:   0,
                        level_count:      1,
                        base_array_layer: 0,
                        layer_count:      1
                    })]
            );

            Ok(())
        })?;

        // Create the image view.
        let view = device.create_image_view(
            &vk::ImageViewCreateInfo::default()
                .image(image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(settings.format)
                .components(vk::ComponentMapping {
                    r: vk::ComponentSwizzle::IDENTITY,
                    g: vk::ComponentSwizzle::IDENTITY,
                    b: vk::ComponentSwizzle::IDENTITY,
                    a: vk::ComponentSwizzle::IDENTITY
                })
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask:      vk::ImageAspectFlags::COLOR,
                    base_mip_level:   0,
                    level_count:      1,
                    base_array_layer: 0,
                    layer_count:      1
                }),
            None
        )?;

        // Destroy the src buffer.
        src.destroy(device);

        Ok(Self {
            image,
            memory,
            view
        })
    }

    /// Create a new image from a file.
    pub unsafe fn new_from_file(
        device: &Device,
        settings: &ImageSettings,
        path: &Path
    ) -> Result<Self> {
        // Load the texture from disk.
        let data = Reader::open(path)?
            .decode()?
            .to_rgba8();

        // Get the image size.
        let size = data.dimensions();
        let size = vk::Extent2D {
            width:  size.0,
            height: size.1
        };

        // Create the image.
        let image = Self::new(device, settings, &data, &size)?;

        Ok(image)
    }

    /// Returns the image view.
    pub fn view(&self) -> &vk::ImageView {
        &self.view
    }

    /// Destroy the image.
    pub unsafe fn destroy(&self, device: &Device) {
        // Destroy the image view.
        device.destroy_image_view(self.view, None);

        // Destroy the image.
        device.destroy_image(self.image, None);

        // Free the memory.
        device.free_memory(self.memory, None);
    }
}

impl Deref for ImmutableImage {
    type Target = vk::Image;

    fn deref(&self) -> &Self::Target {
        &self.image
    }
}
