use crate::{Device, Swapchain};
use anyhow::{anyhow, Result};
use ash::vk;
use bytemuck::cast_slice;
use std::{ffi::CStr, fs::read, path::PathBuf};

/// The pipeline settings.
pub struct PipelineSettings {
    /// The vert shader path.
    pub vert_shader_path: PathBuf,

    /// The frag shader path.
    pub frag_shader_path: PathBuf
}

/// Wraps a Vulkan pipeline.
pub struct Pipeline {
    /// The pipeline layout.
    pipeline_layout: vk::PipelineLayout
}

impl Pipeline {
    pub unsafe fn new(device: &Device, settings: &PipelineSettings) -> Result<Self> {
        // Create the shaders.
        let vert_shader = Self::load_shader(device, &settings.vert_shader_path)?;
        let frag_shader = Self::load_shader(device, &settings.frag_shader_path)?;

        // This is the entry function for the shaders.
        let shader_entry_name = CStr::from_bytes_with_nul_unchecked(b"main\0");

        // Setup the shader stage create infos.
        let shader_stage_create_infos = [
            vk::PipelineShaderStageCreateInfo::default()
                .module(vert_shader)
                .name(&shader_entry_name)
                .stage(vk::ShaderStageFlags::VERTEX),
            vk::PipelineShaderStageCreateInfo::default()
                .module(frag_shader)
                .name(&shader_entry_name)
                .stage(vk::ShaderStageFlags::FRAGMENT)
        ];

        // Setup the dynamic state create info.
        let dynamic_state_create_info = vk::PipelineDynamicStateCreateInfo::default()
            .dynamic_states(&[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR]);

        // Setup the vertex input state create info.
        let vertex_input_state_create_info = vk::PipelineVertexInputStateCreateInfo::default();

        // Setup the input assembly state create info.
        let input_assembly_state_create_info = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST);

        // The pipeline viewport state create info.
        let viewport_state_create_info = vk::PipelineViewportStateCreateInfo::default()
            .viewport_count(1)
            .scissor_count(1);

        // The rasterization state create info.
        let rasterization_state_create_info = vk::PipelineRasterizationStateCreateInfo::default()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .cull_mode(vk::CullModeFlags::BACK)
            .front_face(vk::FrontFace::CLOCKWISE)
            .depth_bias_enable(false);

        // The multisample state create info.
        let multisample_state_create_info = vk::PipelineMultisampleStateCreateInfo::default()
            .sample_shading_enable(false)
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        // The color blend attachment state.
        let color_blend_attachment_states = [vk::PipelineColorBlendAttachmentState::default()
            .color_write_mask(vk::ColorComponentFlags::RGBA)
            .blend_enable(false)];

        // The color blend state create info.
        let color_blend_state_create_info = vk::PipelineColorBlendStateCreateInfo::default()
            .attachments(&color_blend_attachment_states);

        // The pipeline layout create info.
        let pipeline_layout_create_info = vk::PipelineLayoutCreateInfo::default();

        // Create the pipeline layout.
        let pipeline_layout = device.create_pipeline_layout(&pipeline_layout_create_info, None)?;

        // Destroy the shaders.
        device.destroy_shader_module(vert_shader, None);
        device.destroy_shader_module(frag_shader, None);

        Ok(Self { pipeline_layout })
    }

    /// Load a shader.
    unsafe fn load_shader(device: &Device, path: &PathBuf) -> Result<vk::ShaderModule> {
        // Read the file from disk.
        let bytes = read(path)?;

        // Error if the SPIR-V shader is not aligned to 4 bytes.
        if bytes.len() % 4 != 0 {
            return Err(anyhow!("The SPIR-V shader is not aligned to 4 bytes."));
        }

        // We must pass the data to Vulkan as u32's.
        let bytes: &[u32] = cast_slice(&bytes);

        // Create the shader create info.
        let shader_create_info = vk::ShaderModuleCreateInfo::default().code(&bytes);

        // Create the shader.
        let shader = device.create_shader_module(&shader_create_info, None)?;

        Ok(shader)
    }

    /// Destroy the pipeline.
    pub(crate) unsafe fn destroy(&mut self, device: &Device) {
        // Destroy the pipeline layout.
        device.destroy_pipeline_layout(self.pipeline_layout, None);
    }
}
