use anyhow::Result;
use ash::vk;
use std::ffi::{c_void, CStr};
use tracing::{debug, error, trace, warn};

/// Wraps the data for the debug messenger.
pub struct Debugging {
    /// The function pointers.
    functions: ash::ext::debug_utils::Instance,

    /// The messenger.
    messenger: vk::DebugUtilsMessengerEXT
}

impl Debugging {
    pub unsafe fn new(entry: &ash::Entry, instance: &ash::Instance) -> Result<Self> {
        // Load debug functions.
        let functions = ash::ext::debug_utils::Instance::new(entry, instance);

        // Create the messenger info.
        let messenger_info = Self::messenger_info();

        // Create the messenger.
        let messenger = functions.create_debug_utils_messenger(&messenger_info, None)?;

        Ok(Self {
            functions,
            messenger
        })
    }

    /// Create the debug messenger info.
    pub fn messenger_info<'a>() -> vk::DebugUtilsMessengerCreateInfoEXT<'a> {
        vk::DebugUtilsMessengerCreateInfoEXT::default()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                    | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                    | vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
            )
            .message_type(
                vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
            )
            .pfn_user_callback(Some(Self::c_debug_callback))
    }

    /// The debug callback entry point.
    unsafe extern "system" fn c_debug_callback(
        message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
        message_types: vk::DebugUtilsMessageTypeFlagsEXT,
        callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
        _user_data: *mut c_void
    ) -> vk::Bool32 {
        let callback_data = &*callback_data;

        Self::debug_callback(message_severity, message_types, callback_data);

        vk::FALSE
    }

    /// The debug callback.
    fn debug_callback(
        message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
        message_types: vk::DebugUtilsMessageTypeFlagsEXT,
        callback_data: &vk::DebugUtilsMessengerCallbackDataEXT
    ) {
        // TODO: Add queue and command buffer labels and objects.

        let message = unsafe {
            format!(
                "{:?}: (0x{:x?}) {}",
                message_types,
                callback_data.message_id_number,
                CStr::from_ptr(callback_data.p_message)
                    .to_str()
                    .unwrap()
            )
        };

        if message_severity.contains(vk::DebugUtilsMessageSeverityFlagsEXT::ERROR) {
            error!(message);
        } else if message_severity.contains(vk::DebugUtilsMessageSeverityFlagsEXT::WARNING) {
            warn!(message);
        } else if message_severity.contains(vk::DebugUtilsMessageSeverityFlagsEXT::INFO) {
            debug!(message);
        } else if message_severity.contains(vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE) {
            trace!(message);
        }
    }

    /// Destroy the debug messenger.
    pub unsafe fn destroy(&mut self) {
        self.functions
            .destroy_debug_utils_messenger(self.messenger, None);
    }
}
