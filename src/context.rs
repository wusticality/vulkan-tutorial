use crate::{Debugging, Device, Instance, Pipeline, PipelineSettings, Surface, Swapchain};
use anyhow::{anyhow, Result};
use ash::Entry;
use std::{
    env::current_exe, ffi::CStr, fs::canonicalize, mem::ManuallyDrop, path::PathBuf, sync::Arc
};
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
    swapchain: Swapchain,

    /// The pipeline wrapper.
    pipeline: Pipeline
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

        let assets_path = assets_path()?;

        // Create the pipeline wrapper.
        let pipeline = Pipeline::new(
            &device,
            &PipelineSettings {
                vert_shader_path: assets_path.join("shaders/shader.vert.spv"),
                frag_shader_path: assets_path.join("shaders/shader.frag.spv")
            }
        )?;

        Ok(Self {
            window,
            entry,
            instance,
            debugging,
            surface,
            device,
            swapchain,
            pipeline
        })
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe {
            // TODO: Make every component optional and destroy it if anything goes wrong!

            // Destroy the pipeline.
            self.pipeline.destroy();

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

// TODO: Don't hardcode this!
fn assets_path() -> Result<PathBuf> {
    let path = current_exe()?
        .parent()
        .map(PathBuf::from)
        .ok_or_else(|| anyhow!("Could not get parent directory"))?;
    let path = path.join("../../../assets");
    let path = canonicalize(path)?;

    Ok(path)
}
