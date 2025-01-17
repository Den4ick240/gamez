use std::num::NonZero;

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use winit::dpi::PhysicalSize;

#[repr(C)]
#[derive(Debug, Copy, Clone, Zeroable, Pod)]
struct Instance {
    spawned_particles: u32,
    dt: f32,
    bound_radius: f32,
}

pub struct SimulationUniform {
    staging_buffer: wgpu::Buffer,
    buffer: wgpu::Buffer,
    belt: wgpu::util::StagingBelt,
}

impl SimulationUniform {
    pub fn new(device: &wgpu::Device) -> Self {
        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("SimulationStagingBuffer"),
            size: std::mem::size_of::<Instance>() as u64,
            usage: wgpu::BufferUsages::MAP_WRITE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("SimulationUniformBuffer"),
            size: std::mem::size_of::<Instance>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        // let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        //     label: Some("SimulationUniformBuffer"),
        //     contents: bytemuck::cast_slice(&[Instance {
        //         spawned_particles: 0,
        //         dt: 0.0,
        //     }]),
        //     usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        // });
        let belt = wgpu::util::StagingBelt::new(1 << 10);
        Self {
            buffer,
            staging_buffer,
            belt,
        }
    }

    pub fn update(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        queue: &wgpu::Queue,
        spawned_particles: u32,
        bound_radius: f32,
        dt: f32,
    ) {
        queue.write_buffer(
            &self.buffer,
            0,
            bytemuck::cast_slice(&[Instance {
                spawned_particles,
                dt,
                bound_radius,
            }]),
        );
        // self.staging_buffer.slice(..).map
        // self.staging_buffer
        //     .slice(..)
        //     .get_mapped_range_mut()
        //     .copy_from_slice(bytemuck::cast_slice(&[Instance {
        //         spawned_particles,
        //         dt,
        //     }]));
        // encoder.copy_buffer_to_buffer(&self.staging_buffer, 0, &self.buffer, 0, self.buffer.size());
        // let view = self.belt.write_buffer(
        //     encoder,
        //     &self.buffer,
        //     0,
        //     NonZero::new(self.buffer.size()).unwrap(),
        //     device,
        // );
        // view.
        // view[0] = Instance {
        //     spawned_particles,
        //     dt,
        // };
        // view.finish();
    }

    pub fn get_binding_resource(&self) -> wgpu::BindingResource<'_> {
        self.buffer.as_entire_binding()
    }
}
