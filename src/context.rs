use crate::{Debugging, Device, Instance, Surface, Swapchain};
use anyhow::Result;
use ash::Entry;
use std::{ffi::CStr, mem::ManuallyDrop, sync::Arc};
use winit::window::Window;

/// The Vulkan context.
pub struct Context {
    /// A handle to the window.
    window: Arc<Window>,

    /// A handle to the vulkan library.
    entry: ash::Entry,

    /// The instance wrapper.
    instance: ManuallyDrop<Instance>,

    /// The debugging wrapper.
    debugging: Option<Debugging>,

    /// The surface wrapper.
    surface: Surface,

    /// The device wrapper.
    device: Device,

    /// The swapchain wrapper.
    swapchain: Swapchain
}

impl Context {
    /// Create a new Vulkan instance.
    pub unsafe fn new(window: Arc<Window>, name: &CStr) -> Result<Self> {
        // Load the Vulkan library.
        let entry = Entry::linked();

        // Create the instance wrapper.
        let instance = ManuallyDrop::new(Instance::new(window.clone(), &entry, name)?);

        // Capture messages for everything else.
        let debugging = match cfg!(debug_assertions) {
            true => Some(Debugging::new(&entry, &instance)?),
            false => None
        };

        // Create the surface wrapper.
        let surface = Surface::new(window.clone(), &entry, &instance)?;

        // Create the device wrapper.
        let device = Device::new(&instance, &surface)?;

        // Create the swapchain wrapper.
        let swapchain = Swapchain::new(window.clone(), &instance, &device, &surface)?;

        Ok(Self {
            window,
            entry,
            instance,
            debugging,
            surface,
            device,
            swapchain
        })
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe {
            // TODO: Make every component optional and destroy it if anything goes wrong!

            // Destroy the swapchain.
            self.swapchain.destroy(&self.device);

            // Destroy the device.
            self.device.destroy();

            // Destroy the surface.
            self.surface.destroy();

            // Destroy the debugging data.
            if let Some(debugging) = &mut self.debugging {
                debugging.destroy();
            }

            // Destroy the instance.
            self.instance.destroy();
        }
    }
}
