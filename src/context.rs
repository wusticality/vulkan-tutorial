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
    debugging: Option<ManuallyDrop<Debugging>>,

    /// The surface wrapper.
    surface: ManuallyDrop<Surface>,

    /// The device wrapper.
    device: ManuallyDrop<Device>,

    /// The swapchain wrapper.
    swapchain: ManuallyDrop<Swapchain>
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
            true => Some(ManuallyDrop::new(Debugging::new(&entry, &instance)?)),
            false => None
        };

        // Create the surface wrapper.
        let surface = ManuallyDrop::new(Surface::new(window.clone(), &entry, &instance)?);

        // Create the device wrapper.
        let device = ManuallyDrop::new(Device::new(&instance, &surface)?);

        // Create the swapchain wrapper.
        let swapchain = ManuallyDrop::new(Swapchain::new(
            window.clone(),
            &instance,
            &device,
            &surface
        )?);

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
            ManuallyDrop::drop(&mut self.swapchain);

            // Destroy the device.
            ManuallyDrop::drop(&mut self.device);

            // Destroy the surface.
            ManuallyDrop::drop(&mut self.surface);

            // Destroy the debugging data.
            if let Some(debugging) = &mut self.debugging {
                ManuallyDrop::drop(debugging);
            }

            // Destroy the instance.
            ManuallyDrop::drop(&mut self.instance);
        }
    }
}
