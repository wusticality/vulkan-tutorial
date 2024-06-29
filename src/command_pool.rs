use crate::Device;
use anyhow::Result;
use ash::vk;
use std::ops::Deref;

/// Wraps a Vulkan command pool.
pub struct CommandPool(vk::CommandPool);

impl CommandPool {
    pub unsafe fn new(device: &Device) -> Result<Self> {
        let queue_family_index = device.queue_family_index();

        // Create the command pool create info.
        let command_pool_info = vk::CommandPoolCreateInfo::default()
            .queue_family_index(queue_family_index)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);

        // Create the command pool.
        let command_pool = device.create_command_pool(&command_pool_info, None)?;

        Ok(Self(command_pool))
    }

    /// Create a new command buffer.
    pub unsafe fn new_command_buffer(&self, device: &Device) -> Result<vk::CommandBuffer> {
        // Create the command buffer create info.
        let command_buffer_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(self.0)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);

        // Create the command buffer.
        Ok(device.allocate_command_buffers(&command_buffer_info)?[0])
    }

    /// Destroy the command pool.
    pub unsafe fn destroy(&mut self, device: &Device) {
        device.destroy_command_pool(self.0, None);
    }
}

impl Deref for CommandPool {
    type Target = vk::CommandPool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
