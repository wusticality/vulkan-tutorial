use std::ops::Deref;

use anyhow::Result;
use ash::vk;

use crate::{Device, Swapchain};

/// Wraps a Vulkan render pass.
pub struct RenderPass {
    /// The render pass.
    render_pass: vk::RenderPass,

    /// The frame buffers.
    frame_buffers: Vec<vk::Framebuffer>
}

impl RenderPass {
    /// Create a new render pass.
    pub unsafe fn new(device: &Device, swapchain: &Swapchain) -> Result<Self> {
        // Get the swapchain's format.
        let format = swapchain.format();

        // Create the render pass.
        let render_pass = device.create_render_pass(
            &vk::RenderPassCreateInfo::default()
                .attachments(&[vk::AttachmentDescription {
                    format: format.format,
                    samples: vk::SampleCountFlags::TYPE_1,
                    load_op: vk::AttachmentLoadOp::CLEAR,
                    store_op: vk::AttachmentStoreOp::STORE,
                    stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
                    stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
                    initial_layout: vk::ImageLayout::UNDEFINED,
                    final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
                    ..Default::default()
                }])
                .subpasses(&[vk::SubpassDescription::default()
                    .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
                    .color_attachments(&[vk::AttachmentReference {
                        attachment: 0,
                        layout:     vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL
                    }])]),
            None
        )?;

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
                    .render_pass(render_pass)
                    .attachments(&attachments)
                    .width(extent.width)
                    .height(extent.height)
                    .layers(1);

                // Create the frame buffer.
                device.create_framebuffer(&framebuffer_create_info, None)
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            render_pass,
            frame_buffers
        })
    }

    /// Destroy the render pass.
    pub(crate) unsafe fn destroy(&mut self, device: &Device) {
        // Destroy the frame buffers.
        for frame_buffer in &self.frame_buffers {
            device.destroy_framebuffer(*frame_buffer, None);
        }

        // Destroy the render pass.
        device.destroy_render_pass(self.render_pass, None);
    }
}

impl Deref for RenderPass {
    type Target = vk::RenderPass;

    fn deref(&self) -> &Self::Target {
        &self.render_pass
    }
}
