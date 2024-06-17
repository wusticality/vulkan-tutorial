use crate::{
    command_pool::CommandPool, Debugging, Device, Instance, Pipeline, PipelineSettings, RenderPass,
    Surface, Swapchain
};
use anyhow::{anyhow, Result};
use ash::{vk, Entry};
use std::{env::current_exe, ffi::CStr, fs::canonicalize, path::PathBuf, sync::Arc};
use winit::window::Window;

/// The Vulkan context.
pub struct Context {
    /// A handle to the window.
    window: Arc<Window>,

    /// The instance wrapper.
    instance: Instance,

    /// The debugging wrapper.
    debugging: Option<Debugging>,

    /// The surface wrapper.
    surface: Surface,

    /// The device wrapper.
    device: Device,

    /// The command pool.
    command_pool: CommandPool,

    /// The swapchain wrapper.
    swapchain: Swapchain,

    /// The render pass wrapper.
    render_pass: RenderPass,

    /// The pipeline wrapper.
    pipeline: Pipeline,

    /// The command buffer.
    command_buffer: vk::CommandBuffer,

    /// The image ready semaphore.
    semaphore_image_ready: vk::Semaphore,

    /// The render done semaphore.
    semaphore_render_done: vk::Semaphore,

    /// The frame done fence.
    fence_frame_done: vk::Fence
}

impl Context {
    /// Create a new Vulkan instance.
    pub unsafe fn new(window: Arc<Window>, name: &CStr) -> Result<Self> {
        // Load the Vulkan library.
        let entry = Entry::linked();

        // Create the instance wrapper.
        let instance = Instance::new(window.clone(), &entry, name)?;

        // Capture messages for everything else.
        let debugging = match cfg!(debug_assertions) {
            true => Some(Debugging::new(&entry, &instance)?),
            false => None
        };

        // Create the surface wrapper.
        let surface = Surface::new(window.clone(), &entry, &instance)?;

        // Create the device wrapper.
        let device = Device::new(&instance, &surface)?;

        // Create the command pool wrapper.
        let command_pool = CommandPool::new(&device)?;

        // Create the swapchain wrapper.
        let swapchain = Swapchain::new(window.clone(), &instance, &device, &surface)?;

        // Create the render pass wrapper.
        let render_pass = RenderPass::new(&device, &swapchain)?;

        let assets_path = assets_path()?;

        // Create the pipeline wrapper.
        let pipeline = Pipeline::new(
            &device,
            &render_pass,
            &PipelineSettings {
                vert_shader_path: assets_path.join("shaders/shader.vert.spv"),
                frag_shader_path: assets_path.join("shaders/shader.frag.spv")
            }
        )?;

        // Create the command buffer.
        let command_buffer = command_pool.new_command_buffer(&device)?;

        // Create the semaphores.
        let semaphore_image_ready = device.create_semaphore(&Default::default(), None)?;
        let semaphore_render_done = device.create_semaphore(&Default::default(), None)?;

        // Create the fence. Start in the signaled state so that the first
        // frame doesn't wait indefinitely for the fence to be signaled.
        let fence_frame_done = device.create_fence(
            &vk::FenceCreateInfo {
                flags: vk::FenceCreateFlags::SIGNALED,
                ..Default::default()
            },
            None
        )?;

        Ok(Self {
            window,
            instance,
            debugging,
            surface,
            device,
            command_pool,
            swapchain,
            render_pass,
            pipeline,
            command_buffer,
            semaphore_image_ready,
            semaphore_render_done,
            fence_frame_done
        })
    }

    /// Draw the frame.
    pub unsafe fn draw(&self) -> Result<()> {
        // Wait for the fence indefinitely.
        self.device
            .wait_for_fences(&[self.fence_frame_done], true, std::u64::MAX)?;

        // Reset the fence.
        self.device
            .reset_fences(&[self.fence_frame_done])?;

        // Acquire the next swapchain image.
        let present_index = self
            .swapchain
            .acquire(&self.semaphore_image_ready)?;

        // Get a reference to the command buffer.
        let command_buffer = &self.command_buffer;

        // Reset the command buffer.
        self.device
            .reset_command_buffer(self.command_buffer, vk::CommandBufferResetFlags::empty())?;

        // Create the begin info.
        let begin_info = vk::CommandBufferBeginInfo::default();

        // Begin the command buffer.
        self.device
            .begin_command_buffer(*command_buffer, &begin_info)?;

        // Begin the render pass.
        self.render_pass
            .begin(&self.device, &self.swapchain, command_buffer, present_index);

        // Draw the pipeline.
        self.pipeline
            .draw(&self.device, &self.swapchain, command_buffer);

        // End the render pass.
        self.render_pass
            .end(&self.device, command_buffer);

        // End the command buffer.
        self.device
            .end_command_buffer(*command_buffer)?;

        // Create the submit info.
        let submit_info = vk::SubmitInfo::default()
            .wait_semaphores(std::slice::from_ref(&self.semaphore_image_ready))
            .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
            .command_buffers(std::slice::from_ref(command_buffer))
            .signal_semaphores(std::slice::from_ref(&self.semaphore_render_done));

        // Submit the command buffer.
        self.device
            .queue_submit(*self.device.queue(), &[submit_info], self.fence_frame_done)?;

        // Present the image.
        self.swapchain
            .present(&self.device, &self.semaphore_render_done, present_index)?;

        Ok(())
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe {
            // Wait for the device to finish. We must do this or
            // we may be in the middle of rendering on the GPU.
            // This ensures that the GPU is done before we start
            // destroying all the various Vulkan resources.
            self.device
                .device_wait_idle()
                .unwrap();

            // Destroy the fence.
            self.device
                .destroy_fence(self.fence_frame_done, None);

            // Destroy the semaphores.
            self.device
                .destroy_semaphore(self.semaphore_image_ready, None);
            self.device
                .destroy_semaphore(self.semaphore_render_done, None);

            // Destroy the pipeline.
            self.pipeline.destroy(&self.device);

            // Destroy the render pass.
            self.render_pass
                .destroy(&self.device);

            // Destroy the swapchain.
            self.swapchain.destroy(&self.device);

            // Destroy the command pool.
            self.command_pool
                .destroy(&self.device);

            // Destroy the device.
            self.device.destroy();

            // Destroy the surface.
            self.surface.destroy();

            // Destroy the debugging data.
            if let Some(debugging) = &mut self.debugging {
                debugging.destroy();
            }

            // Destroy the instance.
            self.instance.destroy();
        }
    }
}

// TODO: Don't hardcode this!
fn assets_path() -> Result<PathBuf> {
    let path = current_exe()?
        .parent()
        .map(PathBuf::from)
        .ok_or_else(|| anyhow!("Could not get parent directory"))?;
    let path = path.join("../../../assets");
    let path = canonicalize(path)?;

    Ok(path)
}
