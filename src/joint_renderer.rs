use std::mem;

use wgpu::vertex_attr_array;

use crate::{
    simulation::{Border, JointInstance, Simulation},
    square_mesh::SquareMesh,
    wgpu_utils::round_buffer_size,
};

impl JointInstance {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        const ATTRS: [wgpu::VertexAttribute; 2] =
            vertex_attr_array![1 => Float32x2, 2 => Float32x2];
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<JointInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &ATTRS,
        }
    }
}

pub struct JointRenderer {
    pipeline: wgpu::RenderPipeline,
    instance_buffer: wgpu::Buffer,
}

const MAX_JOINTS: u64 = 20000;

fn get_joint_buffer_size() -> wgpu::BufferAddress {
    round_buffer_size(
        (MAX_JOINTS as usize * mem::size_of::<JointInstance>()) as wgpu::BufferAddress,
    )
}

impl JointRenderer {
    pub fn new(
        device: &wgpu::Device,
        surface_config: &wgpu::SurfaceConfiguration,
        main_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("JointInstanceBuffer"),
            size: get_joint_buffer_size(),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let shader = device.create_shader_module(wgpu::include_wgsl!("../shaders/joint.wgsl"));
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("JointPipelineLayout"),
            bind_group_layouts: &[&main_bind_group_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("JointPipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                buffers: &[SquareMesh::desc(), JointInstance::desc()],
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
        simulation: &Simulation,
    ) {
        let joints: Vec<JointInstance> = simulation.get_joints();
        queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&joints));
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, square_mesh.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));

        render_pass.draw(0..4, 0..joints.len() as u32);
    }
}
