use crate::{Buffer, Device, Pipeline, PipelineSettings, RenderPass, VertexDescriptions};
use anyhow::Result;
use ash::vk;
use std::{
    mem::{offset_of, size_of},
    path::PathBuf
};

/// Our vertex type.
#[derive(Clone, Copy)]
struct Vertex {
    position: glam::Vec2,
    color:    glam::Vec3
}

impl Vertex {
    /// Get the binding description.
    fn bindings() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription {
            binding:    0,
            stride:     size_of::<Vertex>() as u32,
            input_rate: vk::VertexInputRate::VERTEX
        }
    }

    /// Get the attribute descriptions.
    fn attributes() -> Vec<vk::VertexInputAttributeDescription> {
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
    pipeline: Pipeline,

    /// The vertex buffer.
    vertices: Buffer
}

impl TriangleRenderer {
    pub unsafe fn new(
        assets_path: &PathBuf,
        device: &Device,
        render_pass: &RenderPass
    ) -> Result<Self> {
        // The paths to the shaders.
        let vert_shader_path = assets_path.join("shaders/shader.vert.spv");
        let frag_shader_path = assets_path.join("shaders/shader.frag.spv");

        // Our vertex descriptions.
        let vertex_descriptions = VertexDescriptions {
            bindings:   vec![Vertex::bindings()],
            attributes: Vertex::attributes()
        };

        // Create the pipeline.
        let pipeline = Pipeline::new(
            device,
            render_pass,
            &PipelineSettings {
                subpass:             0,
                vert_shader_path:    vert_shader_path,
                frag_shader_path:    frag_shader_path,
                vertex_descriptions: Some(vertex_descriptions),
                topology:            vk::PrimitiveTopology::TRIANGLE_LIST,
                polygon_mode:        vk::PolygonMode::FILL,
                cull_mode:           vk::CullModeFlags::BACK,
                front_face:          vk::FrontFace::CLOCKWISE
            }
        )?;

        // Create the vertex buffer.
        let vertices = Buffer::new(device, vk::BufferUsageFlags::VERTEX_BUFFER, &VERTICES)?;

        Ok(Self { pipeline, vertices })
    }

    /// Draw the pipeline.
    pub unsafe fn draw(&self, device: &Device, command_buffer: &vk::CommandBuffer) {
        // First, bind the pipeline.
        device.cmd_bind_pipeline(
            *command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            *self.pipeline
        );

        // Bind the vertex buffer.
        device.cmd_bind_vertex_buffers(*command_buffer, 0, &[*self.vertices], &[0]);

        // Issue the draw command.
        device.cmd_draw(*command_buffer, 3, 1, 0, 0);
    }

    /// Destroy the renderer.
    pub unsafe fn destroy(&mut self, device: &Device) {
        // Destroy the vertex buffer.
        self.vertices.destroy(device);

        // Destroy the pipeline.
        self.pipeline.destroy(device);
    }
}
