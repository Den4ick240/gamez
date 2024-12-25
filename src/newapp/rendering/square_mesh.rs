use std::mem;

use wgpu::util::DeviceExt;

pub struct SquareMesh {
    pub vertex_buffer: wgpu::Buffer,
}

impl SquareMesh {
    pub fn new(device: &wgpu::Device) -> Self {
        let vertices = vec![[-1, -1], [1, -1], [-1, 1], [1, 1]]
            .iter()
            .map(|it| glam::vec2(it[0] as f32, it[1] as f32))
            .collect::<Vec<_>>();
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("SquareMesh"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        Self { vertex_buffer }
    }

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        const ATTRS: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Float32x2];
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<glam::Vec2>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &ATTRS,
        }
    }
}
