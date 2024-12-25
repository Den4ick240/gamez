use std::mem;

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::newapp::simulation::Simulation;

use super::{square_mesh::SquareMesh, wgpu_utils::round_buffer_size, RenderingContext};

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
struct Instance {
    pub position: glam::Vec2,
    pub radius: f32,
    pub color: glam::Vec3,
}

impl Instance {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        const ATTRS: [wgpu::VertexAttribute; 3] =
            wgpu::vertex_attr_array![1 => Float32x2, 2 => Float32, 3 => Float32x3];
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Instance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &ATTRS,
        }
    }
}

pub struct SimulationRenderer {
    pipeline: wgpu::RenderPipeline,
    instance_buffer: wgpu::Buffer,
}

const MAX_PARTICLES: u64 = 20000;

fn get_particle_buffer_size() -> wgpu::BufferAddress {
    round_buffer_size((MAX_PARTICLES as usize * mem::size_of::<Instance>()) as wgpu::BufferAddress)
}

impl SimulationRenderer {
    pub fn new(context: &RenderingContext, shader_module: &wgpu::ShaderModule) -> Self {
        let instance_buffer = context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("SimulationInstanceBuffer"),
            size: get_particle_buffer_size(),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let pipeline = create_pipeline(context, shader_module);
        Self {
            pipeline,
            instance_buffer,
        }
    }

    pub fn on_shader_updated(
        &mut self,
        context: &RenderingContext,
        shader_module: &wgpu::ShaderModule,
    ) {
        self.pipeline = create_pipeline(context, shader_module);
    }

    pub fn render(
        &self,
        render_pass: &mut wgpu::RenderPass,
        queue: &wgpu::Queue,
        square_mesh: &SquareMesh,
        simulation: &Simulation,
    ) {
        let particles = simulation
            .get_particles()
            .iter()
            .map(|it| Instance {
                position: it.position,
                radius: it.radius,
                color: it.color,
            })
            .collect::<Vec<_>>();
        queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&particles));
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, square_mesh.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        render_pass.draw(0..4, 0..particles.len() as u32);
    }
}

fn create_pipeline(
    context: &RenderingContext,
    shader: &wgpu::ShaderModule,
) -> wgpu::RenderPipeline {
    let pipeline_layout = context
        .device
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("SimulationPipelineLayout"),
            bind_group_layouts: &[&context.main_bind_group_layout],
            push_constant_ranges: &[],
        });
    context
        .device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("SimulationPipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                buffers: &[SquareMesh::desc(), Instance::desc()],
                entry_point: Some("vs_simulation"),
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_simulation"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: context.surface_config.format,
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
        })
}
