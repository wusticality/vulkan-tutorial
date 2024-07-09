use crate::{CommandPool, Instance, Surface};
use anyhow::{anyhow, Result};
use ash::vk::{self};
use std::{ffi::CStr, ops::Deref, slice::from_ref};
use tracing::info;

/// Wraps a Vulkan device.
pub struct Device {
    /// The physical device.
    physical_device: vk::PhysicalDevice,

    /// The physical device properties.
    properties: vk::PhysicalDeviceProperties,

    /// The physical device features.
    features: vk::PhysicalDeviceFeatures,

    /// The memory properties.
    memory_properties: vk::PhysicalDeviceMemoryProperties,

    /// The logical device.
    device: ash::Device,

    /// The graphics queue.
    queue: vk::Queue,

    /// The queue family index.
    queue_family_index: u32,

    /// The regular command pool.
    command_pool: CommandPool,

    /// The transient command pool.
    transient_command_pool: CommandPool
}

impl Device {
    pub unsafe fn new(instance: &Instance, surface: &Surface) -> Result<Self> {
        // We at least require the swapchain extension.
        let mut required_extensions = vec![ash::khr::swapchain::NAME];

        // On macOS, we also require the portability extension.
        if cfg!(target_os = "macos") {
            required_extensions.push(ash::khr::portability_subset::NAME);
        }

        // Print the required device extensions.
        for extension in &required_extensions {
            info!("Device extension: {:?}", extension);
        }

        // First, get a list of all candidates and their properties. Filter
        // out the ones that we can't use and compute a score for each one.
        let mut candidates = instance
            .enumerate_physical_devices()?
            .into_iter()
            // Compute a tuple for each queue family and its index.
            .flat_map(|physical_device| {
                let properties = instance.get_physical_device_properties(physical_device);
                let features = instance.get_physical_device_features(physical_device);

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
            // Filter out unsuitable candidates.
            .filter(
                |(physical_device, properties, features, queue_family_index, queue)| {
                    Self::is_suitable(
                        instance,
                        surface,
                        &required_extensions,
                        physical_device,
                        properties,
                        features,
                        *queue_family_index,
                        queue
                    )
                    .unwrap_or(false)
                }
            )
            // Compute a score for each candidate.
            .map(
                |(physical_device, properties, features, queue_family_index, queue)| {
                    let score = Self::score(
                        &physical_device,
                        &properties,
                        &features,
                        queue_family_index,
                        &queue
                    );

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
        let (_score, physical_device, properties, features, queue_family_index, _queue) =
            candidates
                .first()
                .ok_or_else(|| anyhow!("No suitable physical device found!"))?;

        // Get the memory properties.
        let memory_properties = instance.get_physical_device_memory_properties(*physical_device);

        // Create one queue for graphics and presentation.
        let queue_info = vk::DeviceQueueCreateInfo::default()
            .queue_family_index(*queue_family_index)
            .queue_priorities(&[1.0]);

        // Create our device features.
        let enabled_features = vk::PhysicalDeviceFeatures::default().sampler_anisotropy(true);

        // We have to pass this as &[*const c_char].
        let required_extensions = required_extensions
            .iter()
            .map(|extension| extension.as_ptr())
            .collect::<Vec<_>>();

        // Create the device info.
        let device_info = vk::DeviceCreateInfo::default()
            .enabled_extension_names(&required_extensions)
            .queue_create_infos(from_ref(&queue_info))
            .enabled_features(&enabled_features);

        // Create the device.
        let device = instance.create_device(*physical_device, &device_info, None)?;

        // Get the queue.
        let queue = device.get_device_queue(*queue_family_index, 0);

        // Create the command pool.
        let command_pool = CommandPool::new(
            &device,
            *queue_family_index,
            vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER
        )?;

        // Create the transient command pool.
        let transient_command_pool = CommandPool::new(
            &device,
            *queue_family_index,
            vk::CommandPoolCreateFlags::TRANSIENT
        )?;

        Ok(Self {
            physical_device: *physical_device,
            properties: *properties,
            features: *features,
            memory_properties,
            device,
            queue,
            queue_family_index: *queue_family_index,
            command_pool,
            transient_command_pool
        })
    }

    /// Returns the physical device.
    pub fn physical_device(&self) -> &vk::PhysicalDevice {
        &self.physical_device
    }

    /// Returns the physical device properties.
    pub fn properties(&self) -> &vk::PhysicalDeviceProperties {
        &self.properties
    }

    /// Returns the physical device features.
    pub fn features(&self) -> &vk::PhysicalDeviceFeatures {
        &self.features
    }

    /// Returns the memory properties.
    pub fn memory_properties(&self) -> &vk::PhysicalDeviceMemoryProperties {
        &self.memory_properties
    }

    /// Returns the queue.
    pub fn queue(&self) -> &vk::Queue {
        &self.queue
    }

    /// Returns the queue family index.
    pub fn queue_family_index(&self) -> u32 {
        self.queue_family_index
    }

    /// Returns the command pool.
    pub fn command_pool(&self) -> &CommandPool {
        &self.command_pool
    }

    /// Execute a one-time command.
    pub unsafe fn one_time_command<F>(&self, f: F) -> Result<()>
    where
        F: FnOnce(vk::CommandBuffer) -> Result<()>
    {
        // Create the command buffer.
        let command_buffer = self
            .transient_command_pool
            .new_command_buffer(self, true)?;

        // Create the fence so we can wait for completion.
        let fence = self.create_fence(&vk::FenceCreateInfo::default(), None)?;

        // Create the begin info.
        let begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        // Begin the command buffer.
        self.begin_command_buffer(command_buffer, &begin_info)?;

        // Do the actual work.
        f(command_buffer)?;

        // End the command buffer.
        self.end_command_buffer(command_buffer)?;

        // Create the submit info.
        let submit_info = vk::SubmitInfo::default().command_buffers(from_ref(&command_buffer));

        // Submit the command buffer.
        self.device
            .queue_submit(self.queue, &[submit_info], fence)?;

        // Wait for the fence indefinitely.
        self.device
            .wait_for_fences(&[fence], true, std::u64::MAX)?;

        // Destroy the fence.
        self.destroy_fence(fence, None);

        // Free the command buffer.
        self.free_command_buffers(*self.transient_command_pool, from_ref(&command_buffer));

        Ok(())
    }

    /// Checks if the device has the required extensions.
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

    /// Returns true if the device is suitable.
    unsafe fn is_suitable(
        instance: &Instance,
        surface: &Surface,
        required_extensions: &Vec<&CStr>,
        physical_device: &vk::PhysicalDevice,
        _properties: &vk::PhysicalDeviceProperties,
        features: &vk::PhysicalDeviceFeatures,
        queue_family_index: u32,
        queue: &vk::QueueFamilyProperties
    ) -> Result<bool> {
        // A candidate must support our required extensions.
        if !Self::device_has_extensions(instance, physical_device, required_extensions) {
            return Ok(false);
        }

        // We require ansitropic filtering.
        if features.sampler_anisotropy == 0 {
            return Ok(false);
        }

        let formats = surface.formats(&physical_device)?;
        let present_modes = surface.present_modes(&physical_device)?;

        // We'd better have at least one surface format and present mode.
        if formats.is_empty() || present_modes.is_empty() {
            return Ok(false);
        }

        // We must have a queue with graphics support.
        let graphics_support = queue
            .queue_flags
            .contains(vk::QueueFlags::GRAPHICS);

        // We must have a queue with presentation support.
        let presentation_support =
            surface.supports_presentation(physical_device, queue_family_index);

        Ok(graphics_support && presentation_support)
    }

    // Computes a score for the physical device.
    unsafe fn score(
        _physical_device: &vk::PhysicalDevice,
        properties: &vk::PhysicalDeviceProperties,
        _features: &vk::PhysicalDeviceFeatures,
        _queue_family_index: u32,
        _queue: &vk::QueueFamilyProperties
    ) -> u32 {
        let mut score = 0;

        // Give discrete GPUs a higher score.
        if properties.device_type == vk::PhysicalDeviceType::DISCRETE_GPU {
            score += 1000;
        }

        score
    }

    /// Destroy the device.
    pub unsafe fn destroy(&mut self) {
        // Destroy the transient command pool.
        self.transient_command_pool
            .destroy(&self.device);

        // Destroy the command pool.
        self.command_pool
            .destroy(&self.device);

        // Destroy the device.
        self.device.destroy_device(None);
    }
}

impl Deref for Device {
    type Target = ash::Device;

    fn deref(&self) -> &Self::Target {
        &self.device
    }
}
