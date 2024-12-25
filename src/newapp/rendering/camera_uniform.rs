use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use winit::dpi::PhysicalSize;

#[repr(C)]
#[derive(Debug, Copy, Clone, Zeroable, Pod)]
struct Instance {
    width: f32,
    height: f32,
}

pub struct CameraUniform {
    buffer: wgpu::Buffer,
}

impl CameraUniform {
    pub fn new(device: &wgpu::Device, size: &PhysicalSize<u32>) -> Self {
        let instance = Instance {
            width: size.width as f32,
            height: size.height as f32,
        };
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("CameraUniform"),
            contents: bytemuck::cast_slice(&[instance]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        Self { buffer }
    }

    pub fn on_resize(&self, queue: &wgpu::Queue, size: &PhysicalSize<u32>) {
        let instance = Instance {
            width: size.width as f32,
            height: size.height as f32,
        };
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[instance]))
    }

    pub fn get_binding_resource(&self) -> wgpu::BindingResource<'_> {
        self.buffer.as_entire_binding()
    }
}
