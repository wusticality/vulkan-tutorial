use crate::{
    Device, ImageSettings, ImmutableBuffer, ImmutableImage, MappedBuffer, Pipeline,
    PipelineSettings, RenderPass, Swapchain, VertexDescriptions
};
use anyhow::Result;
use ash::vk::{self};
use glam::{Mat4, Vec3};
use std::{
    mem::{offset_of, size_of},
    path::PathBuf,
    time::Instant
};

/// Our vertex type.
#[derive(Clone, Copy)]
#[repr(C)]
struct Vertex {
    position: glam::Vec2,
    color:    glam::Vec3,
    uv:       glam::Vec2
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
                binding:  0,
                location: 0,
                format:   vk::Format::R32G32_SFLOAT,
                offset:   offset_of!(Vertex, position) as u32
            },
            vk::VertexInputAttributeDescription {
                binding:  0,
                location: 1,
                format:   vk::Format::R32G32B32_SFLOAT,
                offset:   offset_of!(Vertex, color) as u32
            },
            vk::VertexInputAttributeDescription {
                binding:  0,
                location: 2,
                format:   vk::Format::R32G32_SFLOAT,
                offset:   offset_of!(Vertex, uv) as u32
            },
        ]
    }
}

/// The vertices of our triangle.
const VERTICES: [Vertex; 4] = [
    Vertex {
        position: glam::Vec2::new(-0.5, -0.5),
        color:    glam::Vec3::new(1.0, 0.0, 0.0),
        uv:       glam::Vec2::new(1.0, 0.0)
    },
    Vertex {
        position: glam::Vec2::new(0.5, -0.5),
        color:    glam::Vec3::new(0.0, 1.0, 0.0),
        uv:       glam::Vec2::new(0.0, 0.0)
    },
    Vertex {
        position: glam::Vec2::new(0.5, 0.5),
        color:    glam::Vec3::new(0.0, 0.0, 1.0),
        uv:       glam::Vec2::new(0.0, 1.0)
    },
    Vertex {
        position: glam::Vec2::new(-0.5, 0.5),
        color:    glam::Vec3::new(1.0, 1.0, 1.0),
        uv:       glam::Vec2::new(1.0, 1.0)
    }
];

/// The indices of our triangle.
const INDICES: [u16; 6] = [0, 1, 2, 2, 3, 0];

/// Our uniform buffer object.
#[derive(Clone, Copy, Default)]
#[repr(C)]
struct UniformData {
    model: glam::Mat4,
    view:  glam::Mat4,
    proj:  glam::Mat4
}

/// Per-frame data.
struct PerFrameData {
    /// The uniform buffer.
    uniforms: MappedBuffer<UniformData>,

    /// The descriptor set.
    descriptor_set: vk::DescriptorSet
}

impl PerFrameData {
    pub unsafe fn new(
        device: &Device,
        descriptor_pool: &vk::DescriptorPool,
        descriptor_set_layout: &vk::DescriptorSetLayout,
        image: &ImmutableImage,
        sampler: &vk::Sampler
    ) -> Result<Self> {
        // Create the uniform buffer.
        let uniforms = MappedBuffer::new(
            device,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            &[UniformData::default()]
        )?;

        // Create the descriptor set.
        let descriptor_set = device.allocate_descriptor_sets(
            &vk::DescriptorSetAllocateInfo::default()
                .descriptor_pool(*descriptor_pool)
                .set_layouts(&[*descriptor_set_layout])
        )?[0];

        // Update the descriptor set.
        device.update_descriptor_sets(
            &[
                vk::WriteDescriptorSet::default()
                    .dst_set(descriptor_set)
                    .dst_binding(0)
                    .dst_array_element(0)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .buffer_info(&[vk::DescriptorBufferInfo::default()
                        .buffer(*uniforms)
                        .offset(0)
                        .range(size_of::<UniformData>() as vk::DeviceSize)]),
                vk::WriteDescriptorSet::default()
                    .dst_set(descriptor_set)
                    .dst_binding(1)
                    .dst_array_element(0)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(&[vk::DescriptorImageInfo::default()
                        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                        .image_view(*image.view())
                        .sampler(*sampler)])
            ],
            &[]
        );

        Ok(Self {
            uniforms,
            descriptor_set
        })
    }

    /// Destroy the per-frame data.
    pub unsafe fn destroy(&mut self, device: &Device) {
        // Destroy the uniform buffer.
        self.uniforms.destroy(device);
    }
}

/// The triangle renderer.
pub struct TriangleRenderer {
    /// The image.
    image: ImmutableImage,

    /// The image sampler.
    sampler: vk::Sampler,

    /// The vertex buffer.
    vertices: ImmutableBuffer,

    /// The index buffer.
    indices: ImmutableBuffer,

    // The descriptor set layout.
    descriptor_set_layout: vk::DescriptorSetLayout,

    // The descriptor pool.
    descriptor_pool: vk::DescriptorPool,

    /// The per-frame data.
    per_frame_data: Vec<PerFrameData>,

    /// The per-frame index.
    per_frame_index: usize,

    /// The pipeline.
    pipeline: Pipeline,

    /// The starting time.
    start_time: std::time::Instant
}

impl TriangleRenderer {
    pub unsafe fn new(
        assets_path: &PathBuf,
        device: &Device,
        render_pass: &RenderPass,
        frames_in_flight: u32
    ) -> Result<Self> {
        // Get the physical device properties.
        let properties = device.properties();

        // The paths this renderer uses.
        let vert_shader_path = assets_path.join("shaders/shader.vert.spv");
        let frag_shader_path = assets_path.join("shaders/shader.frag.spv");
        let image_path = assets_path.join("textures/meme.jpg");

        // Load the image from disk.
        let image = ImmutableImage::new_from_file(
            device,
            &ImageSettings {
                format:  vk::Format::R8G8B8A8_SRGB,
                usage:   vk::ImageUsageFlags::SAMPLED,
                samples: vk::SampleCountFlags::TYPE_1
            },
            &image_path
        )?;

        // Create the sampler.
        let sampler = device.create_sampler(
            &vk::SamplerCreateInfo::default()
                .min_filter(vk::Filter::LINEAR)
                .mag_filter(vk::Filter::LINEAR)
                .address_mode_u(vk::SamplerAddressMode::REPEAT)
                .address_mode_v(vk::SamplerAddressMode::REPEAT)
                .address_mode_w(vk::SamplerAddressMode::REPEAT)
                .anisotropy_enable(true)
                .max_anisotropy(
                    properties
                        .limits
                        .max_sampler_anisotropy
                )
                .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
                .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
                .mip_lod_bias(0.0)
                .min_lod(0.0)
                .max_lod(0.0),
            None
        )?;

        // Create the vertex buffer.
        let vertices =
            ImmutableBuffer::new(device, vk::BufferUsageFlags::VERTEX_BUFFER, &VERTICES)?;

        // Create the index buffer.
        let indices = ImmutableBuffer::new(device, vk::BufferUsageFlags::INDEX_BUFFER, &INDICES)?;

        // Create the descriptor set layout.
        let descriptor_set_layout = device.create_descriptor_set_layout(
            &vk::DescriptorSetLayoutCreateInfo::default().bindings(&[
                vk::DescriptorSetLayoutBinding::default()
                    .binding(0)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::VERTEX),
                vk::DescriptorSetLayoutBinding::default()
                    .binding(1)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::FRAGMENT)
            ]),
            None
        )?;

        // Create the vertex descriptions.
        let vertex_descriptions = VertexDescriptions {
            bindings:   vec![Vertex::bindings()],
            attributes: Vertex::attributes()
        };

        // Create the descriptor set layouts.
        let descriptor_set_layouts = vec![descriptor_set_layout];

        // Create the descriptor pool.
        let descriptor_pool = device.create_descriptor_pool(
            &vk::DescriptorPoolCreateInfo::default()
                .pool_sizes(&[
                    vk::DescriptorPoolSize::default()
                        .ty(vk::DescriptorType::UNIFORM_BUFFER)
                        .descriptor_count(frames_in_flight),
                    vk::DescriptorPoolSize::default()
                        .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                        .descriptor_count(frames_in_flight)
                ])
                .max_sets(frames_in_flight),
            None
        )?;

        // Create the per-frame data.
        let per_frame_data = (0..frames_in_flight)
            .map(|_| {
                PerFrameData::new(
                    &device,
                    &descriptor_pool,
                    &descriptor_set_layout,
                    &image,
                    &sampler
                )
            })
            .collect::<Result<Vec<_>>>()?;

        // Create the pipeline.
        let pipeline = Pipeline::new(
            device,
            render_pass,
            &PipelineSettings {
                subpass: 0,
                vert_shader_path,
                frag_shader_path,
                vertex_descriptions: Some(vertex_descriptions),
                topology: vk::PrimitiveTopology::TRIANGLE_LIST,
                polygon_mode: vk::PolygonMode::FILL,
                cull_mode: vk::CullModeFlags::BACK,
                front_face: vk::FrontFace::COUNTER_CLOCKWISE,
                descriptor_set_layouts: Some(descriptor_set_layouts)
            }
        )?;

        Ok(Self {
            image,
            sampler,
            vertices,
            indices,
            descriptor_set_layout,
            descriptor_pool,
            per_frame_data,
            per_frame_index: 0,
            pipeline,
            start_time: Instant::now()
        })
    }

    /// Draw the pipeline.
    pub unsafe fn draw(
        &mut self,
        device: &Device,
        swapchain: &Swapchain,
        command_buffer: &vk::CommandBuffer,
        _per_frame_index: usize
    ) -> Result<()> {
        // Get the extent.
        let extent = swapchain.extent();

        // Get our uniform data.
        let uniform_data = self.get_uniform_data(&extent);

        // Get the per-frame data.
        let per_frame_data = &mut self.per_frame_data[self.per_frame_index];
        let uniforms = &mut per_frame_data.uniforms;
        let descriptor_set = &per_frame_data.descriptor_set;

        // Update the uniform buffer.
        uniforms.overwrite(&[uniform_data])?;

        // Bind the descriptor set.
        device.cmd_bind_descriptor_sets(
            *command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            *self.pipeline.pipeline_layout(),
            0,
            &[*descriptor_set],
            &[]
        );

        // First, bind the pipeline.
        device.cmd_bind_pipeline(
            *command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            *self.pipeline
        );

        // Bind the vertex buffer.
        device.cmd_bind_vertex_buffers(*command_buffer, 0, &[*self.vertices], &[0]);

        // Bind the index buffer.
        device.cmd_bind_index_buffer(*command_buffer, *self.indices, 0, vk::IndexType::UINT16);

        // Issue the draw command.
        device.cmd_draw_indexed(*command_buffer, INDICES.len() as u32, 1, 0, 0, 0);

        Ok(())
    }

    /// Update the uniform data.
    unsafe fn get_uniform_data(&self, extent: &vk::Extent2D) -> UniformData {
        // Get the elapsed time in seconds.
        let elapsed = self
            .start_time
            .elapsed()
            .as_secs_f32();

        // Compute the model matrix.
        let model = Mat4::from_rotation_z(90.0_f32.to_radians() * elapsed);

        // Compute the view matrix.
        let view = Mat4::look_at_rh(
            Vec3::new(2.0, 2.0, 2.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0)
        );

        // Compute the projection matrix.
        let mut proj = Mat4::perspective_rh(
            45.0_f32.to_radians(),
            extent.width as f32 / extent.height as f32,
            0.1,
            10.0
        );

        // Invert the y axis.
        proj.y_axis.y *= -1.0;

        UniformData { model, view, proj }
    }

    /// Destroy the renderer.
    pub unsafe fn destroy(&mut self, device: &Device) {
        // Destroy the pipeline.
        self.pipeline.destroy(device);

        // Destroy the per-frame data.
        self.per_frame_data
            .iter_mut()
            .for_each(|data| data.destroy(device));

        // Destroy the descriptor pool.
        device.destroy_descriptor_pool(self.descriptor_pool, None);

        // Destroy the descriptor set layout.
        device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);

        // Destroy the index buffer.
        self.indices.destroy(device);

        // Destroy the vertex buffer.
        self.vertices.destroy(device);

        // Destroy the sampler.
        device.destroy_sampler(self.sampler, None);

        // Destroy the image.
        self.image.destroy(device);
    }
}
