use crate::{Device, Instance, Surface};
use anyhow::{anyhow, Result};
use ash::vk::{self};
use std::sync::Arc;
use winit::window::Window;

/// Wraps a Vulkan swapchain.
pub struct Swapchain {
    /// A handle to the window.
    window: Arc<Window>,

    /// The swapchain functions.
    functions: ash::khr::swapchain::Device,

    /// The swapchain.
    swapchain: vk::SwapchainKHR,

    // The swapchain images.
    images: Vec<vk::Image>,

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
        window: Arc<Window>,
        instance: &Instance,
        device: &Device,
        surface: &Surface
    ) -> Result<Self> {
        let functions = ash::khr::swapchain::Device::new(&instance, &device);
        let (swapchain, images, views, format, extent) =
            Self::make(window.clone(), device, surface, &functions)?;

        Ok(Self {
            window,
            functions,
            swapchain,
            images,
            views,
            format,
            extent
        })
    }

    /// Create a new swapchain.
    unsafe fn make(
        window: Arc<Window>,
        device: &Device,
        surface: &Surface,
        functions: &ash::khr::swapchain::Device
    ) -> Result<(
        vk::SwapchainKHR,
        Vec<vk::Image>,
        Vec<vk::ImageView>,
        vk::SurfaceFormatKHR,
        vk::Extent2D
    )> {
        // Get the available surface formats.
        let available_formats = surface.formats(&device.physical_device())?;

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
        // none of our preferences are available.

        // One of our formats must be supported.
        let format = preferred_formats
            .into_iter()
            .find(|x| available_formats.contains(x))
            .ok_or_else(|| anyhow!("No suitable swapchain format found."))?;

        // Get the available present modes.
        let available_present_modes = surface.present_modes(&device.physical_device())?;

        // Our preferred present modes.
        let preferred_present_modes = [vk::PresentModeKHR::MAILBOX, vk::PresentModeKHR::FIFO];

        // On of our present modes must be supported.
        let present_mode = preferred_present_modes
            .into_iter()
            .find(|x| available_present_modes.contains(x))
            .ok_or_else(|| anyhow!("No suitable swapchain present mode found."))?;

        // Get the capabilities of the surface.
        let capabilities = surface.capabilities(&device.physical_device())?;

        // Compute our extent.
        let extent = Self::compute_extent(window.clone(), &capabilities)?;

        // Compute our image count.
        let image_count = Self::compute_image_count(&capabilities);

        // Create the swapchain info.
        let swapchain_info = vk::SwapchainCreateInfoKHR::default()
            .surface(**surface)
            .min_image_count(image_count)
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

        Ok((swapchain, images, views, format, extent))
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
        window: Arc<Window>,
        capabilities: &vk::SurfaceCapabilitiesKHR
    ) -> Result<vk::Extent2D> {
        // If the current extent is set to the int max for both width and height,
        // then compute the extent based on the window size. Otherwise, use the
        // current extent that is provided by the surface.
        Ok(match capabilities.current_extent {
            vk::Extent2D {
                width: u32::MAX,
                height: u32::MAX
            } => {
                let requested = window.inner_size();

                vk::Extent2D {
                    width:  requested.width.clamp(
                        capabilities.min_image_extent.width,
                        capabilities.max_image_extent.width
                    ),
                    height: requested.height.clamp(
                        capabilities.min_image_extent.height,
                        capabilities.max_image_extent.height
                    )
                }
            },

            _ => capabilities.current_extent
        })
    }

    /// Compute the number of images in the swapchain.
    fn compute_image_count(capabilities: &vk::SurfaceCapabilitiesKHR) -> u32 {
        // The number of images in the swapchain.
        let mut image_count = capabilities.min_image_count + 1;

        // If the max image count is greater than zero and the image count
        // is greater than the max image count, then clamp the image count.
        if capabilities.max_image_count > 0 && image_count > capabilities.max_image_count {
            image_count = capabilities.max_image_count;
        }

        image_count
    }

    /// Destroy the swapchain.
    pub(crate) unsafe fn destroy(&mut self, device: &Device) {
        // Destroy the image views.
        for view in &self.views {
            device.destroy_image_view(*view, None);
        }

        // Destroy the swapchain.
        self.functions
            .destroy_swapchain(self.swapchain, None);
    }
}
