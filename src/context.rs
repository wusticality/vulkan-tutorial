use crate::Debugging;
use anyhow::Result;
use ash::{vk, Entry};
use ash_window::enumerate_required_extensions;
use std::{ffi::CStr, sync::Arc};
use winit::{raw_window_handle::HasDisplayHandle, window::Window};

/// The Vulkan version we're using.
pub const VK_VERSION: u32 = vk::make_api_version(0, 1, 3, 0);

/// The Vulkan context.
pub struct Context {
    /// A handle to the window.
    pub window: Arc<Window>,

    /// A handle to the vulkan library.
    pub entry: ash::Entry,

    /// The vulkan instance.
    pub instance: ash::Instance,

    /// The debugging data.
    pub debugging: Option<Debugging>
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

        // Capture messages for everything else.
        let debugging = match cfg!(debug_assertions) {
            true => Some(Debugging::new(&entry, &instance)?),
            false => None
        };

        Ok(Self {
            window,
            entry,
            instance,
            debugging
        })
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe {
            // Destroy the debugging data.
            if let Some(debugging) = self.debugging.take() {
                std::mem::drop(debugging);
            }

            // Destroy the instance.
            self.instance.destroy_instance(None);
        }
    }
}
