use crate::{
    command_pool::CommandPool, Debugging, Device, Instance, Pipeline, PipelineSettings, RenderPass,
    Surface, Swapchain
};
use anyhow::{anyhow, Result};
use ash::{vk, Entry};
use std::{cmp::max, env::current_exe, ffi::CStr, fs::canonicalize, path::PathBuf, sync::Arc};
use tracing::info;
use winit::window::Window;

/// The maximum number of frames in flight.
const FRAMES_IN_FLIGHT: u32 = 2;

/// Per-frame data.
struct PerFrameData {
    /// The command buffer.
    pub command_buffer: vk::CommandBuffer,

    /// The image ready semaphore.
    pub semaphore_image_ready: vk::Semaphore,

    /// The render done semaphore.
    pub semaphore_render_done: vk::Semaphore,

    /// The frame done fence.
    pub fence_frame_done: vk::Fence
}

impl PerFrameData {
    pub unsafe fn new(device: &Device, command_pool: &CommandPool) -> Result<Self> {
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
            command_buffer,
            semaphore_image_ready,
            semaphore_render_done,
            fence_frame_done
        })
    }

    /// Destroy the per-frame data.
    pub unsafe fn destroy(&mut self, device: &Device) {
        // Destroy the fence.
        device.destroy_fence(self.fence_frame_done, None);

        // Destroy the semaphores.
        device.destroy_semaphore(self.semaphore_image_ready, None);
        device.destroy_semaphore(self.semaphore_render_done, None);
    }
}

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

    /// The number of frames in flight.
    frames_in_flight: u32,

    /// The command pool.
    command_pool: CommandPool,

    /// The swapchain wrapper.
    swapchain: Swapchain,

    /// The render pass wrapper.
    render_pass: RenderPass,

    /// The pipeline wrapper.
    pipeline: Pipeline,

    /// The per-frame data.
    per_frame_data: Vec<PerFrameData>,

    /// The per-frame index.
    per_frame_index: usize
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

        // Compute how many frames we can have in flight.
        let frames_in_flight = Self::frames_in_flight(&device, &surface)?;

        info!("Frames in flight: {}", frames_in_flight);

        // Create the command pool wrapper.
        let command_pool = CommandPool::new(&device)?;

        // Create the swapchain wrapper.
        let swapchain = Swapchain::new(
            window.clone(),
            &instance,
            &device,
            &surface,
            frames_in_flight
        )?;

        // Create the render pass wrapper.
        let render_pass = RenderPass::new(&device, &swapchain)?;

        // The path to our assets.
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

        // Create the per-frame data.
        let per_frame_data = (0..frames_in_flight)
            .map(|_| PerFrameData::new(&device, &command_pool))
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            window,
            instance,
            debugging,
            surface,
            device,
            frames_in_flight,
            command_pool,
            swapchain,
            render_pass,
            pipeline,
            per_frame_data,
            per_frame_index: 0
        })
    }

    /// Draw the frame.
    pub unsafe fn draw(&mut self) -> Result<()> {
        // Get the per-frame data.
        let per_frame_data = &self.per_frame_data[self.per_frame_index];

        // Get references to our synchronization objects.
        let command_buffer = per_frame_data.command_buffer;
        let semaphore_image_ready = per_frame_data.semaphore_image_ready;
        let semaphore_render_done = per_frame_data.semaphore_render_done;
        let fence_frame_done = per_frame_data.fence_frame_done;

        // Wait for the fence indefinitely.
        self.device
            .wait_for_fences(&[fence_frame_done], true, std::u64::MAX)?;

        // Reset the fence.
        self.device
            .reset_fences(&[fence_frame_done])?;

        // Acquire the next swapchain image.
        let present_index = self
            .swapchain
            .acquire(&semaphore_image_ready)?;

        // Reset the command buffer.
        self.device
            .reset_command_buffer(command_buffer, vk::CommandBufferResetFlags::empty())?;

        // Create the begin info.
        let begin_info = vk::CommandBufferBeginInfo::default();

        // Begin the command buffer.
        self.device
            .begin_command_buffer(command_buffer, &begin_info)?;

        // Begin the render pass.
        self.render_pass.begin(
            &self.device,
            &self.swapchain,
            &command_buffer,
            present_index
        );

        // Draw the pipeline.
        self.pipeline
            .draw(&self.device, &self.swapchain, &command_buffer);

        // End the render pass.
        self.render_pass
            .end(&self.device, &command_buffer);

        // End the command buffer.
        self.device
            .end_command_buffer(command_buffer)?;

        // Create the submit info.
        let submit_info = vk::SubmitInfo::default()
            .wait_semaphores(std::slice::from_ref(&semaphore_image_ready))
            .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
            .command_buffers(std::slice::from_ref(&command_buffer))
            .signal_semaphores(std::slice::from_ref(&semaphore_render_done));

        // Submit the command buffer.
        self.device
            .queue_submit(*self.device.queue(), &[submit_info], fence_frame_done)?;

        // Present the image.
        self.swapchain
            .present(&self.device, &semaphore_render_done, present_index)?;

        // Advance the per-frame index.
        self.per_frame_index = (self.per_frame_index + 1) % self.frames_in_flight as usize;

        Ok(())
    }

    /// Compute the frames in flight.
    unsafe fn frames_in_flight(device: &Device, surface: &Surface) -> Result<u32> {
        let capabilities = surface.capabilities(&device.physical_device())?;

        Ok(match capabilities.max_image_count {
            0 => max(FRAMES_IN_FLIGHT, capabilities.min_image_count),
            _ => FRAMES_IN_FLIGHT.clamp(capabilities.min_image_count, capabilities.max_image_count)
        })
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

            // Destroy the per-frame data.
            self.per_frame_data
                .iter_mut()
                .for_each(|data| data.destroy(&self.device));

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
