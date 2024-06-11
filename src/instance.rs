use crate::Debugging;
use anyhow::Result;
use ash::vk;
use ash_window::enumerate_required_extensions;
use raw_window_handle::HasDisplayHandle;
use std::{ffi::CStr, ops::Deref, sync::Arc};
use tracing::info;
use winit::window::Window;

/// The Vulkan version we're using.
pub const VK_VERSION: u32 = vk::make_api_version(0, 1, 3, 0);

/// Wraps the Vulkan instance.
pub struct Instance(ash::Instance);

impl Instance {
    pub unsafe fn new(entry: &ash::Entry, window: Arc<Window>, name: &CStr) -> Result<Self> {
        // Create the application info.
        let app_info = vk::ApplicationInfo::default()
            .application_name(name)
            .application_version(0)
            .engine_name(name)
            .engine_version(0)
            .api_version(VK_VERSION);

        // The create flags. macOS requires the portability extension.
        let create_flags = if cfg!(target_os = "macos") {
            vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR
        } else {
            vk::InstanceCreateFlags::default()
        };

        // The required instance extensions. The initial ones come from
        // the ash_window crate. macOS requires the portability extension.
        let required_instance_extensions = {
            let mut extensions =
                enumerate_required_extensions(window.display_handle()?.as_raw())?.to_vec();

            // This is required on macOS.
            if cfg!(target_os = "macos") {
                extensions.push(ash::khr::portability_enumeration::NAME.as_ptr());
            }

            // If we're in debug mode, add the extension that
            // allows us to print validation layer messages.
            if cfg!(debug_assertions) {
                extensions.push(ash::ext::debug_utils::NAME.as_ptr());
            }

            extensions
        };

        // Print the required instance extensions.
        for extension in &required_instance_extensions {
            let extension = CStr::from_ptr(*extension);

            info!("Instance extension: {:?}", extension);
        }

        // Create the instance info.
        let mut instance_info = vk::InstanceCreateInfo::default()
            .flags(create_flags)
            .application_info(&app_info)
            .enabled_extension_names(&required_instance_extensions);

        // This has to live as long as the instance_info.
        let mut messenger_create_info = Debugging::messenger_create_info();

        // Capture messages for instance functions.
        if cfg!(debug_assertions) {
            instance_info = instance_info.push_next(&mut messenger_create_info);
        }

        // Create the instance.
        let instance = entry.create_instance(&instance_info, None)?;

        Ok(Self(instance))
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        unsafe {
            self.0.destroy_instance(None);
        }
    }
}

impl Deref for Instance {
    type Target = ash::Instance;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}