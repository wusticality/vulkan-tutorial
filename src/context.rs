use crate::{Debugging, Instance};
use anyhow::{anyhow, Result};
use ash::{
    vk::{self},
    Entry
};
use ash_window::create_surface;
use raw_window_handle::HasWindowHandle;
use std::{ffi::CStr, mem::ManuallyDrop, sync::Arc};
use tracing::info;
use winit::{raw_window_handle::HasDisplayHandle, window::Window};

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

    /// The surface functions.
    pub surface_fn: ash::khr::surface::Instance,

    /// The surface.
    pub surface: vk::SurfaceKHR,

    /// The physical device.
    pub physical_device: vk::PhysicalDevice,

    /// The logical device.
    pub device: ash::Device,

    /// The graphics queue.
    pub queue: vk::Queue
}

impl Context {
    /// Create a new Vulkan instance.
    pub unsafe fn new(window: Arc<Window>, name: &CStr) -> Result<Self> {
        // Load the Vulkan library.
        let entry = Entry::linked();

        // Create the instance wrapper.
        let instance = ManuallyDrop::new(Instance::new(&entry, window.clone(), name)?);

        // Capture messages for everything else.
        let debugging = match cfg!(debug_assertions) {
            true => Some(ManuallyDrop::new(Debugging::new(&entry, &instance)?)),
            false => None
        };

        // Load the surface functions.
        let surface_fn = ash::khr::surface::Instance::new(&entry, &instance);

        // Create the surface.
        let surface = create_surface(
            &entry,
            &instance,
            window.display_handle()?.as_raw(),
            window.window_handle()?.as_raw(),
            None
        )?;

        // Pick the device.
        let (physical_device, device, queue) = Self::pick_device(&instance, &surface_fn, &surface)?;

        Ok(Self {
            window,
            entry,
            instance,
            debugging,
            surface_fn,
            surface,
            physical_device,
            device,
            queue
        })
    }

    /// Pick a physical device.
    unsafe fn pick_device(
        instance: &ash::Instance,
        surface_fn: &ash::khr::surface::Instance,
        surface: &vk::SurfaceKHR
    ) -> Result<(vk::PhysicalDevice, ash::Device, vk::Queue)> {
        // We at least require the swapchain extension.
        let required_extensions = vec![ash::khr::swapchain::NAME];

        // Print the required device extensions.
        for extension in &required_extensions {
            info!("Device extension: {:?}", extension);
        }

        // First, get a list of all candidates and their properties. Filter out
        // the ones that we can't use. Score candidates, prefering descrete GPUs.
        let mut candidates = instance
            .enumerate_physical_devices()?
            .into_iter()
            .filter(|physical_device| {
                // Potential devices must support our required extensions.
                Self::device_has_extensions(instance, physical_device, &required_extensions)
            })
            .flat_map(|physical_device| {
                let properties = instance.get_physical_device_properties(physical_device);
                let features = instance.get_physical_device_features(physical_device);

                // Create tuples for each queue family and its index.
                instance
                    .get_physical_device_queue_family_properties(physical_device)
                    .into_iter()
                    .enumerate()
                    .map(move |(index, queue)| {
                        (physical_device, properties, features, index, queue)
                    })
            })
            .filter(|(physical_device, _properties, _features, index, queue)| {
                // We must have a queue with graphics support.
                let graphics_support = queue
                    .queue_flags
                    .contains(vk::QueueFlags::GRAPHICS);

                // We must have a queue with presentation support.
                let presentation_support = surface_fn
                    .get_physical_device_surface_support(*physical_device, *index as u32, *surface)
                    .unwrap_or(false);

                graphics_support && presentation_support
            })
            .map(|(physical_device, properties, features, index, queue)| {
                let mut score = 0;

                // Give discrete GPUs a higher score.
                if properties.device_type == vk::PhysicalDeviceType::DISCRETE_GPU {
                    score += 1000;
                }

                (score, physical_device, properties, features, index, queue)
            })
            .collect::<Vec<_>>();

        // Score the candidates by score.
        candidates.sort_by(|a, b| b.0.cmp(&a.0));

        // Take the highest scoring candidate.
        let (_score, physical_device, _properties, _features, queue_family_index, _queue) =
            candidates
                .first()
                .ok_or_else(|| anyhow!("No suitable physical device found!"))?;

        // Create one queue for graphics and presentation.
        let queue_create_info = vk::DeviceQueueCreateInfo::default()
            .queue_family_index(*queue_family_index as u32)
            .queue_priorities(&[1.0]);

        // Create our device features.
        let physical_device_features = vk::PhysicalDeviceFeatures::default();

        // We have to pass this as &[*const c_char].
        let required_extensions = required_extensions
            .iter()
            .map(|extension| extension.as_ptr())
            .collect::<Vec<_>>();

        // Create the device info.
        let device_info = vk::DeviceCreateInfo::default()
            .enabled_extension_names(&required_extensions)
            .queue_create_infos(std::slice::from_ref(&queue_create_info))
            .enabled_features(&physical_device_features);

        // Create the device.
        let device = instance.create_device(*physical_device, &device_info, None)?;

        // Get the queue.
        let queue = device.get_device_queue(*queue_family_index as u32, 0);

        Ok((*physical_device, device, queue))
    }

    // Checks if the device has the required extensions.
    unsafe fn device_has_extensions(
        instance: &ash::Instance,
        physical_device: &vk::PhysicalDevice,
        required_extension: &[&CStr]
    ) -> bool {
        match instance.enumerate_device_extension_properties(*physical_device) {
            Ok(available_extensions) => {
                let available_extensions = available_extensions
                    .iter()
                    .map(|available_extension| {
                        CStr::from_ptr(
                            available_extension
                                .extension_name
                                .as_ptr()
                        )
                    })
                    .collect::<Vec<_>>();

                required_extension
                    .iter()
                    .all(|required_extension| {
                        available_extensions
                            .iter()
                            .any(|available_extension| available_extension == required_extension)
                    })
            },

            _ => false
        }
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe {
            // TODO: Make every component optional and destroy it if anything goes wrong!

            // Destroy the device.
            self.device.destroy_device(None);

            // Destroy the surface.
            self.surface_fn
                .destroy_surface(self.surface, None);

            // Destroy the debugging data.
            if let Some(debugging) = &mut self.debugging {
                ManuallyDrop::drop(debugging);
            }

            // Destroy the instance.
            ManuallyDrop::drop(&mut self.instance);
        }
    }
}
