use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Debug, Copy, Clone, Zeroable, Pod)]
struct CameraUniform {
    width: f32,
    height: f32,
    position: glam::Vec2,
    fov: f32,
    _padding: [f32; 3],
}

pub struct CameraState {
    uniform: CameraUniform,
    buffer: wgpu::Buffer,
    size_changed: bool,
}

impl CameraState {
    pub fn new(device: &wgpu::Device, width: f32, height: f32) -> Self {
        let uniform = CameraUniform {
            width,
            height,
            position: glam::vec2(0.0, 0.0),
            fov: 50.0,
            _padding: [0.0; 3],
        };
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("CameraUniform"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        Self {
            uniform,
            buffer,
            size_changed: false,
        }
    }

    pub fn set_size(&mut self, width: f32, height: f32) {
        if self.uniform.width != width || self.uniform.height != height {
            self.size_changed = true;
            self.uniform.width = width;
            self.uniform.height = height;
        }
    }

    pub fn get_size(&self) -> (f32, f32) {
        (self.uniform.width, self.uniform.height)
    }

    pub fn get_fov(&self) -> f32 {
        self.uniform.fov
    }

    pub fn write_buffer(&mut self, queue: &wgpu::Queue) {
        if self.size_changed {
            queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.uniform]));
            self.size_changed = false;
        }
    }

    pub fn get_binding_resource(&self) -> wgpu::BindingResource<'_> {
        self.buffer.as_entire_binding()
    }

    pub fn get_world_position(&self) -> glam::Vec2 {
        self.uniform.position
    }
}
