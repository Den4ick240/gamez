use std::mem;

use wgpu::util::DeviceExt;

use crate::{
    simulation::{Particle, Simulation},
    square_mesh::SquareMesh,
};

type Instance = Particle;

impl Instance {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        const ATTRS: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![1 => Float32x2];
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

fn round_buffer_size(unpadded_size: wgpu::BufferAddress) -> wgpu::BufferAddress {
    // 1. buffer size must be a multiple of COPY_BUFFER_ALIGNMENT.
    // 2. buffer size must be greater than 0.
    // Therefore we round the value up to the nearest multiple, and ensure it's at least COPY_BUFFER_ALIGNMENT.
    let align_mask = wgpu::COPY_BUFFER_ALIGNMENT - 1;
    let padded_size = ((unpadded_size + align_mask) & !align_mask).max(wgpu::COPY_BUFFER_ALIGNMENT);
    padded_size
}

const MAX_PARTICLES: u64 = 100;

fn get_particle_buffer_size() -> u64 {
    round_buffer_size((MAX_PARTICLES as usize * mem::size_of::<Particle>()) as wgpu::BufferAddress)
}

impl SimulationRenderer {
    pub fn new(
        device: &wgpu::Device,
        surface_config: &wgpu::SurfaceConfiguration,
        main_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("SimulationInstanceBuffer"),
            size: get_particle_buffer_size(),
            usage: wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        });
        let shader = device.create_shader_module(wgpu::include_wgsl!("../shaders/simulation.wgsl"));
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("SimulationPipelineLayout"),
            bind_group_layouts: &[&main_bind_group_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("SimulationPipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                buffers: &[SquareMesh::desc(), Instance::desc()],
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
        square_mesh: &SquareMesh,
        simulation: &Simulation,
    ) {
        self.write_buffer(simulation);
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, square_mesh.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));

        render_pass.draw_indexed(
            0..square_mesh.vertex_buffer.size() as u32,
            0,
            0..simulation.particles.len() as u32,
        );
    }

    fn write_buffer(&self, simulation: &Simulation) {
        let data = bytemuck::cast_slice(&simulation.particles);
        let data_size = data.len() as usize;
        let slice = self.instance_buffer.slice(0..data_size);
        slice.get_mapped_range_mut()[..data_size].copy_from_slice(data);
    }
}
