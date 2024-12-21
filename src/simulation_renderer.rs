use std::mem;

use wgpu::util::DeviceExt;

use crate::{
    arrow_renderer::ArrowRenderer,
    border_renderer::BorderRenderer,
    joint_renderer::JointRenderer,
    simulation::{Particle, Simulation},
    square_mesh::SquareMesh,
    wgpu_utils::round_buffer_size,
};

type Instance = Particle;

impl Instance {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        const ATTRS: [wgpu::VertexAttribute; 4] =
            wgpu::vertex_attr_array![1 => Float32x2, 2 => Float32x2, 3 => Float32, 4 => Float32x3];
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
    arrow_renderer: ArrowRenderer,
    border_renderer: BorderRenderer,
    joint_renderer: JointRenderer,
}

const MAX_PARTICLES: u64 = 10000;

fn get_particle_buffer_size() -> wgpu::BufferAddress {
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
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
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
            arrow_renderer: ArrowRenderer::new(device, surface_config, main_bind_group_layout),
            border_renderer: BorderRenderer::new(device, surface_config, main_bind_group_layout),
            joint_renderer: JointRenderer::new(device, surface_config, main_bind_group_layout),
        }
    }

    pub fn render(
        &self,
        render_pass: &mut wgpu::RenderPass,
        queue: &wgpu::Queue,
        square_mesh: &SquareMesh,
        simulation: &Simulation,
    ) {
        self.border_renderer
            .render(render_pass, queue, square_mesh, &simulation.borders);

        self.write_buffer(queue, simulation);
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, square_mesh.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));

        let particles_to_draw =
            simulation.particles.len() + simulation.get_optional_particles().len();
        render_pass.draw(0..4, 0..particles_to_draw as u32);

        self.joint_renderer
            .render(render_pass, queue, square_mesh, simulation);

        self.arrow_renderer
            .render(render_pass, queue, square_mesh, &simulation.get_arrows());
    }

    fn write_buffer(&self, queue: &wgpu::Queue, simulation: &Simulation) {
        queue.write_buffer(
            &self.instance_buffer,
            0,
            bytemuck::cast_slice(&simulation.particles),
        );
        let optionals = simulation.get_optional_particles();
        if optionals.len() > 0 {
            queue.write_buffer(
                &self.instance_buffer,
                (mem::size_of::<Instance>() * simulation.particles.len()) as wgpu::BufferAddress,
                bytemuck::cast_slice(&optionals),
            )
        }
    }
}
