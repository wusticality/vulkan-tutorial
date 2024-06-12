use crate::Device;
use anyhow::{anyhow, Result};
use ash::vk;
use bytemuck::cast_slice;
use std::{fs::read, path::PathBuf};

/// The pipeline settings.
pub struct PipelineSettings {
    /// The vert shader path.
    pub vert_shader_path: PathBuf,

    /// The frag shader path.
    pub frag_shader_path: PathBuf
}

/// Wraps a Vulkan pipeline.
pub struct Pipeline;

impl Pipeline {
    pub unsafe fn new(device: &Device, settings: &PipelineSettings) -> Result<Self> {
        // Create the shaders.
        let vert_shader = Self::load_shader(device, &settings.vert_shader_path)?;
        let frag_shader = Self::load_shader(device, &settings.frag_shader_path)?;

        // Destroy the shaders.
        device.destroy_shader_module(vert_shader, None);
        device.destroy_shader_module(frag_shader, None);

        Ok(Self {})
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
    pub(crate) unsafe fn destroy(&mut self) {}
}
