use std::mem;

use bytemuck::{Pod, Zeroable};

use crate::{square_mesh::SquareMesh, wgpu_utils::round_buffer_size};

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct Arrow {
    pub position: glam::Vec2,
    pub direction: glam::Vec2,
    pub color: glam::Vec3,
    pub norm: f32,
}

impl Arrow {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        const ATTRS: [wgpu::VertexAttribute; 4] =
            wgpu::vertex_attr_array![1 => Float32x2, 2 => Float32x2, 3 => Float32x3, 4 => Float32];
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Arrow>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &ATTRS,
        }
    }
}

pub struct ArrowRenderer {
    pipeline: wgpu::RenderPipeline,
    instance_buffer: wgpu::Buffer,
}

const MAX_ARROWS: u64 = 20000;

fn get_arrow_buffer_size() -> wgpu::BufferAddress {
    round_buffer_size((MAX_ARROWS as usize * mem::size_of::<Arrow>()) as wgpu::BufferAddress)
}

impl ArrowRenderer {
    pub fn new(
        device: &wgpu::Device,
        surface_config: &wgpu::SurfaceConfiguration,
        main_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ArrowInstanceBuffer"),
            size: get_arrow_buffer_size(),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let shader = device.create_shader_module(wgpu::include_wgsl!("../shaders/arrow.wgsl"));
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ArrowPipelineLayout"),
            bind_group_layouts: &[&main_bind_group_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ArrowPipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                buffers: &[SquareMesh::desc(), Arrow::desc()],
                entry_point: None,
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: None,
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });
        Self {
            pipeline,
            instance_buffer,
        }
    }

    pub fn render(
        &self,
        render_pass: &mut wgpu::RenderPass,
        queue: &wgpu::Queue,
        square_mesh: &SquareMesh,
        arrows: &[Arrow],
    ) {
        queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(arrows));
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, square_mesh.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));

        render_pass.draw(0..4, 0..arrows.len() as u32);
    }
}
