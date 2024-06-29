use crate::{Device, Pipeline, PipelineSettings, RenderPass};
use anyhow::Result;
use ash::vk;
use std::{mem::offset_of, path::PathBuf};

/// Our vertex type.
struct Vertex {
    position: glam::Vec2,
    color:    glam::Vec3
}

impl Vertex {
    /// Get the binding description.
    fn binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription {
            binding:    0,
            stride:     std::mem::size_of::<Vertex>() as u32,
            input_rate: vk::VertexInputRate::VERTEX
        }
    }

    /// Get the attribute descriptions.
    fn attribute_descriptions() -> Vec<vk::VertexInputAttributeDescription> {
        vec![
            vk::VertexInputAttributeDescription {
                location: 0,
                binding:  0,
                format:   vk::Format::R32G32_SFLOAT,
                offset:   offset_of!(Vertex, position) as u32
            },
            vk::VertexInputAttributeDescription {
                location: 1,
                binding:  0,
                format:   vk::Format::R32G32B32_SFLOAT,
                offset:   offset_of!(Vertex, color) as u32
            },
        ]
    }
}

/// The vertices of our triangle.
const VERTICES: [Vertex; 3] = [
    Vertex {
        position: glam::Vec2::new(0.0, -0.5),
        color:    glam::Vec3::new(1.0, 0.0, 0.0)
    },
    Vertex {
        position: glam::Vec2::new(0.5, 0.5),
        color:    glam::Vec3::new(0.0, 1.0, 0.0)
    },
    Vertex {
        position: glam::Vec2::new(-0.5, 0.5),
        color:    glam::Vec3::new(0.0, 0.0, 1.0)
    }
];

/// The triangle renderer.
pub struct TriangleRenderer {
    /// The pipeline.
    pipeline: Pipeline
}

impl TriangleRenderer {
    pub unsafe fn new(
        assets_path: &PathBuf,
        device: &Device,
        render_pass: &RenderPass
    ) -> Result<Self> {
        let vert_shader_path = assets_path.join("shaders/shader.vert.spv");
        let frag_shader_path = assets_path.join("shaders/shader.frag.spv");

        // Create the pipeline.
        let pipeline = Pipeline::new(
            device,
            render_pass,
            &PipelineSettings {
                subpass:             0,
                vert_shader_path:    vert_shader_path,
                frag_shader_path:    frag_shader_path,
                vertex_descriptions: None,
                topology:            vk::PrimitiveTopology::TRIANGLE_LIST,
                polygon_mode:        vk::PolygonMode::FILL,
                cull_mode:           vk::CullModeFlags::BACK,
                front_face:          vk::FrontFace::CLOCKWISE
            }
        )?;

        Ok(Self { pipeline })
    }

    /// Draw the pipeline.
    pub unsafe fn draw(&self, device: &Device, command_buffer: &vk::CommandBuffer) {
        // First, bind the pipeline.
        device.cmd_bind_pipeline(
            *command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            *self.pipeline
        );

        // Issue the draw command.
        device.cmd_draw(*command_buffer, 3, 1, 0, 0);
    }

    /// Destroy the renderer.
    pub unsafe fn destroy(&mut self, device: &Device) {
        self.pipeline.destroy(device);
    }
}
