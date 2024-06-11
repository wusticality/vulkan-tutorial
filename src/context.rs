use crate::{Debugging, Device, Instance, Surface};
use anyhow::Result;
use ash::Entry;
use std::{ffi::CStr, mem::ManuallyDrop, sync::Arc};
use winit::window::Window;

/// The Vulkan context.
pub struct Context {
    /// A handle to the window.
    pub window: Arc<Window>,

    /// A handle to the vulkan library.
    pub entry: ash::Entry,

    /// The instance wrapper.
    pub instance: ManuallyDrop<Instance>,

    /// The debugging wrapper.
    pub debugging: Option<ManuallyDrop<Debugging>>,

    /// The surface wrapper.
    pub surface: ManuallyDrop<Surface>,

    /// The device wrapper.
    pub device: ManuallyDrop<Device>
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

        Ok(Self {
            window,
            entry,
            instance,
            debugging,
            surface,
            device
        })
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe {
            // TODO: Make every component optional and destroy it if anything goes wrong!

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
