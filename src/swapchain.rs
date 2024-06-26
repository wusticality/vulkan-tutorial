use crate::{Device, Instance, Surface};
use anyhow::{anyhow, Result};
use ash::vk::{self};
use winit::dpi::PhysicalSize;

/// Wraps a Vulkan swapchain.
pub struct Swapchain {
    /// The swapchain functions.
    functions: ash::khr::swapchain::Device,

    /// The swapchain.
    swapchain: vk::SwapchainKHR,

    // The swapchain image views.
    views: Vec<vk::ImageView>,

    // The surface format.
    format: vk::SurfaceFormatKHR,

    // The current extent.
    extent: vk::Extent2D
}

impl Swapchain {
    /// Create a new swapchain.
    pub unsafe fn new(
        size: &PhysicalSize<u32>,
        instance: &Instance,
        device: &Device,
        surface: &Surface,
        frames_in_flight: u32
    ) -> Result<Self> {
        let functions = ash::khr::swapchain::Device::new(&instance, &device);
        let (swapchain, views, format, extent) =
            Self::make(device, surface, &functions, size, frames_in_flight)?;

        Ok(Self {
            functions,
            swapchain,
            views,
            format,
            extent
        })
    }

    /// Acquire the next image in the swapchain. Returns the index of the acquired image.
    /// If the returned index is None, it means we need to recreate the swapchain first.
    pub unsafe fn acquire(&self, semaphore: &vk::Semaphore) -> Result<Option<u32>> {
        match self.functions.acquire_next_image(
            self.swapchain,
            std::u64::MAX,
            *semaphore,
            vk::Fence::null()
        ) {
            Ok((index, suboptimal)) => match suboptimal {
                true => Ok(None),
                false => Ok(Some(index))
            },
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => Ok(None),
            Err(e) => Err(e.into())
        }
    }

    /// Present the current image. Returns true if the swapchain should be recreated.
    pub unsafe fn present(
        &self,
        device: &Device,
        semaphore: &vk::Semaphore,
        present_index: u32
    ) -> Result<bool> {
        match self.functions.queue_present(
            *device.queue(),
            &vk::PresentInfoKHR::default()
                .wait_semaphores(&[*semaphore])
                .swapchains(&[self.swapchain])
                .image_indices(&[present_index])
        ) {
            Ok(suboptimal) => Ok(suboptimal),
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => Ok(true),
            Err(e) => Err(e.into())
        }
    }

    /// Create a new swapchain.
    unsafe fn make(
        device: &Device,
        surface: &Surface,
        functions: &ash::khr::swapchain::Device,
        size: &PhysicalSize<u32>,
        frames_in_flight: u32
    ) -> Result<(
        vk::SwapchainKHR,
        Vec<vk::ImageView>,
        vk::SurfaceFormatKHR,
        vk::Extent2D
    )> {
        // Get the available surface formats.
        let available_formats = surface.formats(&device.physical_device())?;

        // TODO: Add this to device selection!

        // Our preferred formats.
        let preferred_formats = [
            vk::SurfaceFormatKHR {
                format:      vk::Format::B8G8R8A8_SRGB,
                color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR
            },
            vk::SurfaceFormatKHR {
                format:      vk::Format::R8G8B8A8_SRGB,
                color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR
            }
        ];

        // TODO: Select the first one in the list if
        //  none of our preferences are available.

        // One of our formats must be supported.
        let format = preferred_formats
            .into_iter()
            .find(|x| available_formats.contains(x))
            .ok_or_else(|| anyhow!("No suitable swapchain format found."))?;

        // Get the available present modes.
        let available_present_modes = surface.present_modes(&device.physical_device())?;

        // Our preferred present modes.
        let preferred_present_modes = [vk::PresentModeKHR::FIFO, vk::PresentModeKHR::IMMEDIATE];

        // On of our present modes must be supported.
        let present_mode = preferred_present_modes
            .into_iter()
            .find(|x| available_present_modes.contains(x))
            .ok_or_else(|| anyhow!("No suitable swapchain present mode found."))?;

        // Get the capabilities of the surface.
        let capabilities = surface.capabilities(&device.physical_device())?;

        // Compute our extent.
        let extent = Self::compute_extent(size, &capabilities)?;

        // Create the swapchain info.
        let swapchain_info = vk::SwapchainCreateInfoKHR::default()
            .surface(**surface)
            .min_image_count(frames_in_flight)
            .image_format(format.format)
            .image_color_space(format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(vk::SurfaceTransformFlagsKHR::IDENTITY)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true)
            .old_swapchain(vk::SwapchainKHR::null());

        // Create the swapchain.
        let swapchain = functions.create_swapchain(&swapchain_info, None)?;

        // Get the swapchain images.
        let images = functions.get_swapchain_images(swapchain)?;

        // Create the image views.
        let views = images
            .iter()
            .map(|image| {
                // Create the image view create info.
                let create_info = vk::ImageViewCreateInfo::default()
                    .image(*image)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(format.format)
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
                    });

                // Create the image view.
                device.create_image_view(&create_info, None)
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok((swapchain, views, format, extent))
    }

    /// The image views.
    pub fn views(&self) -> &Vec<vk::ImageView> {
        &self.views
    }

    /// The current format.
    pub fn format(&self) -> vk::SurfaceFormatKHR {
        self.format
    }

    /// The current extent.
    pub fn extent(&self) -> vk::Extent2D {
        self.extent
    }

    /// Compute the extent of the swapchain.
    unsafe fn compute_extent(
        size: &PhysicalSize<u32>,
        capabilities: &vk::SurfaceCapabilitiesKHR
    ) -> Result<vk::Extent2D> {
        // If the current extent is set to the int max for both width and height,
        // then compute the extent based on the window size. Otherwise, use the
        // current extent that is provided by the surface.
        Ok(match capabilities.current_extent {
            vk::Extent2D {
                width: u32::MAX,
                height: u32::MAX
            } => vk::Extent2D {
                width:  size.width.clamp(
                    capabilities.min_image_extent.width,
                    capabilities.max_image_extent.width
                ),
                height: size.height.clamp(
                    capabilities.min_image_extent.height,
                    capabilities.max_image_extent.height
                )
            },

            _ => capabilities.current_extent
        })
    }

    /// Destroy the swapchain.
    pub unsafe fn destroy(&mut self, device: &Device) {
        // Destroy the image views.
        for view in &self.views {
            device.destroy_image_view(*view, None);
        }

        // Destroy the swapchain.
        self.functions
            .destroy_swapchain(self.swapchain, None);
    }
}
