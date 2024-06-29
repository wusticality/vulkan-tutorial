use crate::{Device, FrameBuffers, Swapchain};
use anyhow::Result;
use ash::vk;
use std::ops::Deref;

/// Wraps a Vulkan render pass.
pub struct RenderPass(vk::RenderPass);

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
                    }])])
                .dependencies(&[vk::SubpassDependency {
                    src_subpass: vk::SUBPASS_EXTERNAL,
                    dst_subpass: 0,
                    src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                    src_access_mask: vk::AccessFlags::empty(),
                    dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                    dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                    ..Default::default()
                }]),
            None
        )?;

        Ok(Self(render_pass))
    }

    /// Begin the render pass.
    pub unsafe fn begin(
        &self,
        device: &Device,
        swapchain: &Swapchain,
        frame_buffers: &FrameBuffers,
        command_buffer: &vk::CommandBuffer,
        present_index: u32
    ) {
        // The swapchain extent.
        let extent = swapchain.extent();

        // Create the begin info.
        let begin_info = vk::RenderPassBeginInfo::default()
            .render_pass(self.0)
            .framebuffer(frame_buffers[present_index as usize])
            .render_area(extent.into())
            .clear_values(&[vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 1.0]
                }
            }]);

        // Begin the render pass.
        device.cmd_begin_render_pass(*command_buffer, &begin_info, vk::SubpassContents::INLINE);
    }

    /// End the render pass.
    pub unsafe fn end(&self, device: &Device, command_buffer: &vk::CommandBuffer) {
        // End the render pass.
        device.cmd_end_render_pass(*command_buffer);
    }

    /// Destroy the render pass.
    pub unsafe fn destroy(&mut self, device: &Device) {
        device.destroy_render_pass(self.0, None);
    }
}

impl Deref for RenderPass {
    type Target = vk::RenderPass;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
