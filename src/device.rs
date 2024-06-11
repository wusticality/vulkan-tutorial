use crate::{Instance, Surface};
use anyhow::{anyhow, Result};
use ash::vk;
use std::{ffi::CStr, ops::Deref};
use tracing::info;

/// Wraps a Vulkan device.
pub struct Device {
    /// The physical device.
    pub physical_device: vk::PhysicalDevice,

    /// The logical device.
    pub device: ash::Device,

    /// The graphics queue.
    pub queue: vk::Queue
}

impl Device {
    pub unsafe fn new(instance: &Instance, surface: &Surface) -> Result<Self> {
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
                    .map(move |(queue_family_index, queue)| {
                        (
                            physical_device,
                            properties,
                            features,
                            queue_family_index as u32,
                            queue
                        )
                    })
            })
            .filter(
                |(physical_device, _properties, _features, queue_family_index, queue)| {
                    // We must have a queue with graphics support.
                    let graphics_support = queue
                        .queue_flags
                        .contains(vk::QueueFlags::GRAPHICS);

                    // We must have a queue with presentation support.
                    let presentation_support =
                        surface.supports_presentation(physical_device, *queue_family_index);

                    graphics_support && presentation_support
                }
            )
            .map(
                |(physical_device, properties, features, queue_family_index, queue)| {
                    let mut score = 0;

                    // Give discrete GPUs a higher score.
                    if properties.device_type == vk::PhysicalDeviceType::DISCRETE_GPU {
                        score += 1000;
                    }

                    (
                        score,
                        physical_device,
                        properties,
                        features,
                        queue_family_index,
                        queue
                    )
                }
            )
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
            .queue_family_index(*queue_family_index)
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
        let queue = device.get_device_queue(*queue_family_index, 0);

        Ok(Self {
            physical_device: *physical_device,
            device,
            queue
        })
    }

    // Returns the physical device.
    pub fn physical_device(&self) -> &vk::PhysicalDevice {
        &self.physical_device
    }

    // Returns the queue.
    pub fn queue(&self) -> &vk::Queue {
        &self.queue
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

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            // Destroy the device.
            self.device.destroy_device(None);
        }
    }
}

impl Deref for Device {
    type Target = ash::Device;

    fn deref(&self) -> &Self::Target {
        &self.device
    }
}
