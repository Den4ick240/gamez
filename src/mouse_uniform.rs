use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::timer::Timer;

#[repr(C)]
#[derive(Debug, Copy, Clone, Zeroable, Pod)]
struct MouseUniform {
    position: glam::Vec2,
    animation_progress: f32,
    is_clicked: u32,
}

pub struct MouseState {
    was_clicked: bool,
    click_ts: u64,
    uniform: MouseUniform,
    buffer: wgpu::Buffer,
}

impl MouseUniform {
    fn set_is_clicked(&mut self, value: bool) {
        self.is_clicked = if value { 1 } else { 0 };
    }

    fn get_is_clicked(&self) -> bool {
        self.is_clicked > 0
    }
}

impl MouseState {
    pub fn new(device: &wgpu::Device) -> Self {
        let uniform = MouseUniform {
            position: glam::Vec2::new(0.0, 0.0),
            animation_progress: 1.0,
            is_clicked: 0,
        };
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("MouseUniform"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        Self {
            was_clicked: false,
            click_ts: 0,
            uniform,
            buffer,
        }
    }

    pub fn update(&mut self, timer: &Timer) {
        if self.was_clicked != self.uniform.get_is_clicked() {
            self.click_ts = timer.ms_since_start();
        }
        self.was_clicked = self.uniform.get_is_clicked();
        self.uniform.animation_progress =
            f32::min((timer.ms_since_start() - self.click_ts) as f32 / 100.0, 1.0);
    }

    pub fn write_buffer(&self, queue: &wgpu::Queue) {
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.uniform]));
    }

    pub fn get_bind_group_resource(&self) -> wgpu::BindingResource<'_> {
        self.buffer.as_entire_binding()
    }

    pub fn set_position(&mut self, position: (f64, f64)) {
        self.uniform.position = glam::Vec2::new(position.0 as f32, position.1 as f32);
    }

    pub fn set_is_clicked(&mut self, is_clicked: bool) {
        self.uniform.set_is_clicked(is_clicked);
    }
}
