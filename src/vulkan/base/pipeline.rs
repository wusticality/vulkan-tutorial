use crate::{Device, RenderPass};
use anyhow::{anyhow, Result};
use ash::vk;
use bytemuck::cast_slice;
use std::{ffi::CStr, fs::read, ops::Deref, path::PathBuf};

/// The vertex descriptions.
pub struct VertexDescriptions {
    /// The binding descriptions.
    pub bindings: Vec<vk::VertexInputBindingDescription>,

    /// The attribute descriptions.
    pub attributes: Vec<vk::VertexInputAttributeDescription>
}

/// The pipeline settings.
pub struct PipelineSettings {
    /// What subpass to render to.
    pub subpass: u32,

    /// The vert shader path.
    pub vert_shader_path: PathBuf,

    /// The frag shader path.
    pub frag_shader_path: PathBuf,

    /// The vertex descriptions.
    pub vertex_descriptions: Option<VertexDescriptions>,

    /// The topology.
    pub topology: vk::PrimitiveTopology,

    /// The polygon mode.
    pub polygon_mode: vk::PolygonMode,

    /// The cull mode.
    pub cull_mode: vk::CullModeFlags,

    /// The front face.
    pub front_face: vk::FrontFace,

    /// The descriptor set layouts.
    pub descriptor_set_layouts: Option<Vec<vk::DescriptorSetLayout>>
}

/// Wraps a Vulkan pipeline.
pub struct Pipeline {
    /// The pipeline layout.
    pipeline_layout: vk::PipelineLayout,

    /// The pipeline.
    pipeline: vk::Pipeline
}

impl Pipeline {
    pub unsafe fn new(
        device: &Device,
        render_pass: &RenderPass,
        settings: &PipelineSettings
    ) -> Result<Self> {
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
        let vertex_input_state_create_info = match &settings.vertex_descriptions {
            Some(descriptions) => vk::PipelineVertexInputStateCreateInfo::default()
                .vertex_binding_descriptions(&descriptions.bindings)
                .vertex_attribute_descriptions(&descriptions.attributes),
            None => vk::PipelineVertexInputStateCreateInfo::default()
        };

        // Setup the input assembly state create info.
        let input_assembly_state_create_info =
            vk::PipelineInputAssemblyStateCreateInfo::default().topology(settings.topology);

        // The pipeline viewport state create info.
        let viewport_state_create_info = vk::PipelineViewportStateCreateInfo::default()
            .viewport_count(1)
            .scissor_count(1);

        // The rasterization state create info.
        let rasterization_state_create_info = vk::PipelineRasterizationStateCreateInfo::default()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(settings.polygon_mode)
            .line_width(1.0)
            .cull_mode(settings.cull_mode)
            .front_face(settings.front_face)
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
        let pipeline_layout_create_info = match &settings.descriptor_set_layouts {
            Some(set_layouts) => vk::PipelineLayoutCreateInfo::default().set_layouts(set_layouts),
            None => vk::PipelineLayoutCreateInfo::default()
        };

        // Create the pipeline layout.
        let pipeline_layout = device.create_pipeline_layout(&pipeline_layout_create_info, None)?;

        // Create the pipeline create info.
        let pipeline_create_info = vk::GraphicsPipelineCreateInfo::default()
            .stages(&shader_stage_create_infos)
            .vertex_input_state(&vertex_input_state_create_info)
            .input_assembly_state(&input_assembly_state_create_info)
            .viewport_state(&viewport_state_create_info)
            .rasterization_state(&rasterization_state_create_info)
            .multisample_state(&multisample_state_create_info)
            .color_blend_state(&color_blend_state_create_info)
            .dynamic_state(&dynamic_state_create_info)
            .layout(pipeline_layout)
            .render_pass(**render_pass)
            .subpass(settings.subpass);

        // Create the pipeline.
        let pipeline = match device.create_graphics_pipelines(
            vk::PipelineCache::null(),
            &[pipeline_create_info],
            None
        ) {
            Ok(pipelines) => pipelines,
            _ => return Err(anyhow!("Failed to create graphics pipeline."))
        }[0];

        // Destroy the shaders.
        device.destroy_shader_module(vert_shader, None);
        device.destroy_shader_module(frag_shader, None);

        Ok(Self {
            pipeline_layout,
            pipeline
        })
    }

    /// The pipeline layout.
    pub fn pipeline_layout(&self) -> &vk::PipelineLayout {
        &self.pipeline_layout
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
    pub unsafe fn destroy(&mut self, device: &Device) {
        // Destroy the pipeline.
        device.destroy_pipeline(self.pipeline, None);

        // Destroy the pipeline layout.
        device.destroy_pipeline_layout(self.pipeline_layout, None);
    }
}

impl Deref for Pipeline {
    type Target = vk::Pipeline;

    fn deref(&self) -> &Self::Target {
        &self.pipeline
    }
}
