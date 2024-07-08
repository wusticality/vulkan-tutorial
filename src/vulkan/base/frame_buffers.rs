use crate::{Device, Swapchain};
use anyhow::Result;
use ash::vk::{self, RenderPass};
use std::ops::Deref;

/// Wraps the Vulkan frame buffers.
pub struct FrameBuffers(Vec<vk::Framebuffer>);

impl FrameBuffers {
    pub unsafe fn new(
        device: &Device,
        swapchain: &Swapchain,
        render_pass: &RenderPass
    ) -> Result<Self> {
        // The swapchain extent.
        let extent = swapchain.extent();

        // Create the frame buffers.
        let frame_buffers = swapchain
            .views()
            .iter()
            .map(|view| {
                // The framebuffer attachments.
                let attachments = [*view];

                // Create the frame buffer create info.
                let framebuffer_create_info = vk::FramebufferCreateInfo::default()
                    .render_pass(*render_pass)
                    .attachments(&attachments)
                    .width(extent.width)
                    .height(extent.height)
                    .layers(1);

                // Create the frame buffer.
                device.create_framebuffer(&framebuffer_create_info, None)
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self(frame_buffers))
    }

    /// Destroy the frame buffers.
    pub unsafe fn destroy(&mut self, device: &Device) {
        for frame_buffer in &self.0 {
            device.destroy_framebuffer(*frame_buffer, None);
        }
    }
}

impl Deref for FrameBuffers {
    type Target = Vec<vk::Framebuffer>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
