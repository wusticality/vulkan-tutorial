use std::{ffi::CStr, sync::Arc};

use anyhow::Result;
use ash::{vk, Entry};
use ash_window::enumerate_required_extensions;
use tracing::debug;
use winit::{raw_window_handle::HasDisplayHandle, window::Window};

/// The Vulkan version we're using.
pub const VK_VERSION: u32 = vk::make_api_version(0, 1, 3, 0);

pub struct Context {
    /// A handle to the window.
    window: Arc<Window>,

    /// A handle to the vulkan library.
    entry: ash::Entry,

    /// The vulkan instance.
    instance: ash::Instance
}

impl Context {
    /// Create a new Vulkan instance.
    pub unsafe fn new(window: Arc<Window>) -> Result<Self> {
        // Load the Vulkan library.
        let entry = Entry::linked();

        // The application name.
        let name = CStr::from_bytes_with_nul_unchecked(b"vulkan-tutorial\0");

        // Create the application info.
        let app_info = vk::ApplicationInfo::default()
            .application_name(name)
            .application_version(0)
            .engine_name(name)
            .engine_version(0)
            .api_version(VK_VERSION);

        // Enumerate instance extensions.
        let instance_extensions = entry.enumerate_instance_extension_properties(None)?;

        debug!("Instance extensions:");
        debug!("--------------------");

        for instance_extension in instance_extensions {
            if let Ok(extension_name) = instance_extension.extension_name_as_c_str() {
                debug!("{}", extension_name.to_string_lossy());
            }
        }

        // The create flags. macOS is special.
        let create_flags = if cfg!(target_os = "macos") {
            vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR
        } else {
            vk::InstanceCreateFlags::default()
        };

        // The required instance extensions. macOS is special.
        let required_instance_extensions = {
            let mut extensions =
                enumerate_required_extensions(window.display_handle()?.as_raw())?.to_vec();

            if cfg!(target_os = "macos") {
                extensions.push(ash::khr::portability_enumeration::NAME.as_ptr());
            }

            extensions
        };

        // Create the instance info.
        let instance_info = vk::InstanceCreateInfo::default()
            .flags(create_flags)
            .application_info(&app_info)
            .enabled_extension_names(&required_instance_extensions);

        // Create the instance.
        let instance = entry.create_instance(&instance_info, None)?;

        Ok(Self {
            window,
            entry,
            instance
        })
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe {
            // Destroy the instance.
            self.instance.destroy_instance(None);
        }
    }
}
