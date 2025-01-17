mod simulation_uniform;
use std::mem;

use egui::{vec2, Vec2};
use glam::{vec3, Vec3};
use rand::{rngs::StdRng, Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use simulation_uniform::SimulationUniform;
use wgpu::util::{DeviceExt, RenderEncoder};
use winit::{dpi::PhysicalSize, event_loop::EventLoopProxy};

use super::{
    application_handler::Event,
    rendering::{
        camera_uniform::CameraUniform, square_mesh::SquareMesh, wgpu_utils::round_buffer_size,
    },
    watch_file,
};
use bytemuck::{Pod, Zeroable};

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
struct Particle {
    pub color: Vec3,
    pub radius: f32,
    pub position: Vec2,
    pub velocity: Vec2,
}

impl Particle {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        const ATTRS: [wgpu::VertexAttribute; 4] =
            wgpu::vertex_attr_array![1 => Float32x3, 2 => Float32, 3 => Float32x2, 4 => Float32x2];
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &ATTRS,
        }
    }
}

pub struct Simulation {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
    surface: wgpu::Surface<'static>,
    square_mesh: SquareMesh,
    camera_uniform: CameraUniform,
    main_bind_group_layout: wgpu::BindGroupLayout,
    main_bind_group: wgpu::BindGroup,
    shader_module: wgpu::ShaderModule,
    particles: [Particle; COUNT],
    spawned_particles: u32,
    instance_buffer: wgpu::Buffer,
    render_pipeline: wgpu::RenderPipeline,
    compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group: wgpu::BindGroup,
    compute_bind_group_layout: wgpu::BindGroupLayout,
    simulation_uniform: SimulationUniform,
    update_count: u64,
}

const COUNT: usize = 10000;
const MAX_PARTICLE_RADIUS: f32 = 0.2;
const SHADER_FILE: &'static str = "shaders/compute.wgsl";

const BOUND_RADIUS: u32 = 10;
const FOV: f32 = BOUND_RADIUS as f32 * 2.0;

fn get_particle_buffer_size() -> wgpu::BufferAddress {
    round_buffer_size((COUNT as usize * mem::size_of::<Particle>()) as wgpu::BufferAddress)
}

impl Simulation {
    pub async fn new(
        instance: &wgpu::Instance,
        surface: wgpu::Surface<'static>,
        size: PhysicalSize<u32>,
        proxy: &EventLoopProxy<Event>,
    ) -> Self {
        watch_file::init(proxy, SHADER_FILE);
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .expect("Failed to find an appropriate adapter");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty()
                        | wgpu::Features::VERTEX_WRITABLE_STORAGE,
                    required_limits: Default::default(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .expect("Failed to create device");

        let swapchain_capabilities = surface.get_capabilities(&adapter);
        let selected_format = wgpu::TextureFormat::Bgra8Unorm;
        let swapchain_format = swapchain_capabilities
            .formats
            .iter()
            .find(|d| **d == selected_format)
            .expect("failed to select proper surface texture format!");

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: *swapchain_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 0,
            alpha_mode: swapchain_capabilities.alpha_modes[0],
            view_formats: vec![],
        };

        surface.configure(&device, &surface_config);

        let shader_module = load_shader(&device);

        let camera_uniform = CameraUniform::new(&device, size, FOV);

        let square_mesh = SquareMesh::new(&device);

        let seed: [u8; 32] = [
            1u8, 2u8, 3u8, 4u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0,
        ];
        let mut rng: StdRng = SeedableRng::from_seed(seed);
        let particles = core::array::from_fn(|_| Particle {
            color: rng.get_random_color().into(),
            position: vec2(-5.0, 5.0),
            velocity: vec2(10.0, 0.0),
            radius: rng.get_random_size(),
        });

        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("SimulationInstanceBuffer"),
            contents: bytemuck::cast_slice(&particles),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::STORAGE,
        });

        let simulation_uniform = SimulationUniform::new(&device);

        let main_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("MainBindGroupLayout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0, // camera
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT | wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let compute_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("ComputeBindGroupLayout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let main_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("MainBindGroup"),
            layout: &main_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_uniform.get_binding_resource(),
            }],
        });
        let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ComputeBindGroup"),
            layout: &compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: instance_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: simulation_uniform.get_binding_resource(),
                },
            ],
        });

        let (render_pipeline, compute_pipeline) = create_pipeline(
            &device,
            &main_bind_group_layout,
            &compute_bind_group_layout,
            &surface_config,
            &shader_module,
        );

        Self {
            surface,
            shader_module,
            square_mesh,
            camera_uniform,
            device,
            queue,
            surface_config,
            main_bind_group,
            main_bind_group_layout,
            particles,
            spawned_particles: 1,
            instance_buffer,
            render_pipeline,
            compute_pipeline,
            compute_bind_group,
            compute_bind_group_layout,
            simulation_uniform,
            update_count: 0,
        }
    }

    pub fn update(&mut self, dt: f32, profiler: &mut super::profiler::Profiler) {
        self.spawned_particles = (self.update_count / 2).min(COUNT as u64) as u32;
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        self.simulation_uniform.update(
            &self.device,
            &mut encoder,
            &self.queue,
            self.spawned_particles as u32,
            BOUND_RADIUS as f32,
            dt,
        );

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
            compute_pass.dispatch_workgroups(self.spawned_particles as u32, 1, 1);
        }
        self.queue.submit(std::iter::once(encoder.finish()));
        self.update_count += 1;
    }

    pub fn on_resize(&mut self, size: PhysicalSize<u32>) {
        self.surface_config.width = size.width;
        self.surface_config.height = size.height;
        self.surface.configure(&self.device, &self.surface_config);
        self.camera_uniform.on_resize(&self.queue, size, FOV);
    }

    pub fn render(&self, blend: f64, dt: f64) {
        let surface_texture = self
            .surface
            .get_current_texture()
            .expect("Failed to acquire next swap chain texture");

        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 0.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            render_pass.set_bind_group(0, &self.main_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.square_mesh.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.draw(0..4, 0..self.spawned_particles as u32);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();
    }

    pub fn on_event(&mut self, event: &Event) {
        match event {
            Event::FileUpdated(SHADER_FILE) => {
                self.shader_module = load_shader(&self.device);
                (self.render_pipeline, self.compute_pipeline) = create_pipeline(
                    &self.device,
                    &self.main_bind_group_layout,
                    &self.compute_bind_group_layout,
                    &self.surface_config,
                    &self.shader_module,
                )
            }
            _ => (),
        }
    }
}

fn load_shader(device: &wgpu::Device) -> wgpu::ShaderModule {
    println!("Loading shader");
    let text = std::fs::read_to_string(SHADER_FILE).expect("Shader file not found");
    let res = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(&text)),
    });
    println!("Finished Loading shader");
    res
}

fn create_pipeline(
    device: &wgpu::Device,
    main_bind_group_layout: &wgpu::BindGroupLayout,
    compute_bind_group_layout: &wgpu::BindGroupLayout,
    surface_config: &wgpu::SurfaceConfiguration,
    shader: &wgpu::ShaderModule,
) -> (wgpu::RenderPipeline, wgpu::ComputePipeline) {
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("SimulationPipelineLayout"),
        bind_group_layouts: &[&main_bind_group_layout],
        push_constant_ranges: &[],
    });
    let compute_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("ComputePipelineLayout"),
        bind_group_layouts: &[&compute_bind_group_layout],
        push_constant_ranges: &[],
    });
    let render = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("SimulationPipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            buffers: &[SquareMesh::desc(), Particle::desc()],
            entry_point: Some("vs_particles"),
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_particles"),
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
    let compute = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("IntegratePipeline"),
        layout: Some(&compute_layout),
        module: &shader,
        entry_point: Some("integrate"),
        compilation_options: Default::default(),
        cache: None,
    });
    (render, compute)
}

trait MyRng {
    fn get_random_size(&mut self) -> f32;

    fn get_random_color(&mut self) -> Color;
}

impl MyRng for StdRng {
    fn get_random_size(&mut self) -> f32 {
        self.gen_range(0.7..=1.0) * MAX_PARTICLE_RADIUS
    }

    fn get_random_color(&mut self) -> Color {
        let colors = [
            Color {
                r: 0.945,
                g: 0.769,
                b: 0.058,
            }, // Vibrant Yellow
            Color {
                r: 0.204,
                g: 0.596,
                b: 0.859,
            }, // Sky Blue
            Color {
                r: 0.608,
                g: 0.349,
                b: 0.714,
            }, // Soft Purple
            Color {
                r: 0.231,
                g: 0.764,
                b: 0.392,
            }, // Fresh Green
            Color {
                r: 0.937,
                g: 0.325,
                b: 0.314,
            }, // Coral Red
            Color {
                r: 0.180,
                g: 0.800,
                b: 0.443,
            }, // Mint Green
            Color {
                r: 0.996,
                g: 0.780,
                b: 0.345,
            }, // Soft Orange
            Color {
                r: 0.556,
                g: 0.267,
                b: 0.678,
            }, // Deep Violet
            Color {
                r: 0.870,
                g: 0.490,
                b: 0.847,
            }, // Lavender Pink
            Color {
                r: 0.529,
                g: 0.808,
                b: 0.922,
            }, // Light Blue
            Color {
                r: 0.996,
                g: 0.921,
                b: 0.545,
            }, // Pa.s.tel Yellow
            Color {
                r: 0.835,
                g: 0.625,
                b: 0.459,
            }, // Warm Beige
        ];

        colors[self.gen_range(0..colors.len())]
    }
}

impl Into<Vec3> for Color {
    fn into(self) -> Vec3 {
        vec3(self.r, self.g, self.b)
    }
}
