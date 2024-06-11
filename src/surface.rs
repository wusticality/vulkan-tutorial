use crate::Instance;
use anyhow::Result;
use ash::{vk, Entry};
use ash_window::create_surface;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use std::{ops::Deref, sync::Arc};
use winit::window::Window;

/// Wraps a Vulkan surface.
pub struct Surface {
    /// The surface functions.
    pub functions: ash::khr::surface::Instance,

    /// The surface.
    pub surface: vk::SurfaceKHR
}

impl Surface {
    pub unsafe fn new(window: Arc<Window>, entry: &Entry, instance: &Instance) -> Result<Self> {
        // Load the surface functions.
        let functions = ash::khr::surface::Instance::new(&entry, &instance);

        // Create the surface.
        let surface = create_surface(
            &entry,
            &instance,
            window.display_handle()?.as_raw(),
            window.window_handle()?.as_raw(),
            None
        )?;

        Ok(Self { functions, surface })
    }

    /// Whether or not the surface supports presentation.
    pub unsafe fn supports_presentation(
        &self,
        physical_device: &vk::PhysicalDevice,
        queue_family_index: u32
    ) -> bool {
        self.functions
            .get_physical_device_surface_support(*physical_device, queue_family_index, self.surface)
            .unwrap_or(false)
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            self.functions
                .destroy_surface(self.surface, None);
        }
    }
}

impl Deref for Surface {
    type Target = vk::SurfaceKHR;

    fn deref(&self) -> &Self::Target {
        &self.surface
    }
}
