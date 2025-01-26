mod simulation_uniform;
use std::mem;

use glam::{uvec2, vec2, vec3, UVec2, Vec2, Vec3};
use rand::{rngs::StdRng, Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use simulation_uniform::SimulationUniform;
use wgpu::util::{DeviceExt, RenderEncoder};
use winit::{dpi::PhysicalSize, event_loop::EventLoopProxy};

use crate::rand::MyRng;

use super::{
    application_handler::Event,
    rendering::{
        camera_uniform::CameraUniform, square_mesh::SquareMesh, wgpu_utils::round_buffer_size,
    },
    simulation::{box_constraint::BoxConstraint, spatial_hash::fixed_size_grid::FixedSizeGrid},
    watch_file,
};
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
struct Sort {
    pass_index: u32,
    sorting_length: u32,
    grid_size: UVec2,
    cell_size: Vec2,
    origin: Vec2,
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
    particles: Box<[Particle; COUNT]>,
    spawned_particles: u32,
    instance_buffer: wgpu::Buffer,
    render_pipeline: wgpu::RenderPipeline,
    compute_pipeline: ComputePipeline,
    compute_bind_group: wgpu::BindGroup,
    compute_bind_group_layout: wgpu::BindGroupLayout,
    simulation_uniform: SimulationUniform,
    update_count: u64,
    grid_buffer: wgpu::Buffer,
    sort_buffer: wgpu::Buffer,
    grid: FixedSizeGrid,
}

const GROUP_SIZE: u32 = 256;
const GRID_GROUP_SIZE: u32 = 16;
const COUNT: usize = 1 << 11;
const MAX_PARTICLE_RADIUS: f32 = 0.4;
const SHADER_FILE: &'static str = "shaders/compute.wgsl";

const BOUND_RADIUS: u32 = 40;
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
        watch_file::init(proxy, "shaders");
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

        let mut rng = MyRng::new();
        let mut particles: Box<[Particle; COUNT]> = Box::from(
            [Particle {
                color: vec3(0.0, 0.0, 0.0),
                position: vec2(0.0, 5.0),
                velocity: vec2(0.0, 0.0),
                radius: 0.0,
            }; COUNT],
        );
        let grid = FixedSizeGrid::new(
            MAX_PARTICLE_RADIUS * 2.0,
            BoxConstraint::around_center(BOUND_RADIUS as f32),
        );
        for (i, particle) in particles.as_mut().iter_mut().enumerate() {
            let i = COUNT - i - 1;
            *particle = Particle {
                color: vec3(1.0, 1.0, 0.0),
                // color: vec3(1.0, 1.0, 1.0) * i as f32 / COUNT as f32,
                // color: rng.get_random_color().into(),
                //
                // position: vec2(-5.0 + i as f32 * 0.1, 5.0 + rng.get_random_size(1.0..10.6)),
                position: vec2(
                    (((i % grid.size.x as usize) as f32) * MAX_PARTICLE_RADIUS as f32 * 2.0)
                        + grid.origin.x
                        + MAX_PARTICLE_RADIUS,
                    -((((i / grid.size.y as usize) as f32) * MAX_PARTICLE_RADIUS as f32 * 2.0)
                        + grid.origin.y
                        + MAX_PARTICLE_RADIUS),
                ),
                velocity: vec2(00.0, 0.0),
                // radius: rng.get_random_size(0.5..=1.0) * MAX_PARTICLE_RADIUS,
                radius: MAX_PARTICLE_RADIUS,
            };
        }

        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("SimulationInstanceBuffer"),
            contents: bytemuck::cast_slice(particles.as_ref()),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::STORAGE,
        });
        let grid_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("GridBuffer"),
            // contents: bytemuck::cast_slice(&[0u32; (grid.size.x * grid.size.y) as usize]),
            usage: wgpu::BufferUsages::STORAGE,
            size: (4 * grid.size.x * grid.size.y) as u64,
            mapped_at_creation: false,
        });
        let sort_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("SortBuffer"),
            contents: bytemuck::cast_slice(&[Sort {
                pass_index: 0,
                sorting_length: COUNT as u32,
                grid_size: grid.size,
                cell_size: grid.cell_size,
                origin: grid.origin,
            }]),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let simulation_uniform = SimulationUniform::new(&device);

        let main_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("MainBindGroupLayout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0, // camera
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
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
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
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
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: grid_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: sort_buffer.as_entire_binding(),
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
            grid,
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
            spawned_particles: COUNT as u32,
            // spawned_particles: 1,
            instance_buffer,
            grid_buffer,
            sort_buffer,
            render_pipeline,
            compute_pipeline,
            compute_bind_group,
            compute_bind_group_layout,
            simulation_uniform,
            update_count: 0,
        }
    }

    pub fn update(&mut self, dt: f32, profiler: &mut super::profiler::Profiler) {
        // self.spawned_particles = (self.update_count / 3).min(COUNT as u64) as u32;
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        let substeps = 8;
        let dt = dt / substeps as f32;

        self.simulation_uniform.update(
            &self.device,
            &mut encoder,
            &self.queue,
            self.spawned_particles as u32,
            BOUND_RADIUS as f32,
            dt,
        );

        {
            for s in 0..substeps {
                let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Compute pass"),
                    timestamp_writes: None,
                });
                compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
                compute_pass.set_pipeline(&self.compute_pipeline.update);
                compute_pass.dispatch_workgroups(self.spawned_particles.div_ceil(GROUP_SIZE), 1, 1);
                drop(compute_pass);

                if true {
                    let mut compute_pass =
                        encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                            label: Some("Compute pass"),
                            timestamp_writes: None,
                        });
                    compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
                    compute_pass.set_pipeline(&self.compute_pipeline.sort);
                    compute_pass.dispatch_workgroups(
                        self.spawned_particles.div_ceil(GROUP_SIZE * 2),
                        1,
                        1,
                    );
                    drop(compute_pass);
                    let mut compute_pass =
                        encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                            label: Some("Compute pass"),
                            timestamp_writes: None,
                        });
                    compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
                    compute_pass.set_pipeline(&self.compute_pipeline.clear_grid);
                    compute_pass.dispatch_workgroups(
                        self.grid.size.x.div_ceil(GRID_GROUP_SIZE),
                        self.grid.size.y.div_ceil(GRID_GROUP_SIZE),
                        1,
                    );
                    drop(compute_pass);
                    let mut compute_pass =
                        encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                            label: Some("Compute pass"),
                            timestamp_writes: None,
                        });
                    compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
                    compute_pass.set_pipeline(&self.compute_pipeline.fill_grid);
                    compute_pass.dispatch_workgroups(
                        self.spawned_particles.div_ceil(GROUP_SIZE),
                        1,
                        1,
                    );
                    drop(compute_pass);
                    let mut compute_pass =
                        encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                            label: Some("Compute pass"),
                            timestamp_writes: None,
                        });
                    compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
                    compute_pass.set_pipeline(&self.compute_pipeline.colorize_grid);
                    compute_pass.dispatch_workgroups(
                        self.grid.size.x.div_ceil(GRID_GROUP_SIZE),
                        self.grid.size.y.div_ceil(GRID_GROUP_SIZE),
                        1,
                    );
                    drop(compute_pass);
                }
                if true {
                    for pip in [
                        &self.compute_pipeline.collide_grid4,
                        &self.compute_pipeline.collide_grid5,
                        &self.compute_pipeline.collide_grid6,
                        &self.compute_pipeline.collide_grid1,
                        &self.compute_pipeline.collide_grid2,
                        &self.compute_pipeline.collide_grid3,
                    ] {
                        let mut compute_pass =
                            encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                                label: Some("Compute pass"),
                                timestamp_writes: None,
                            });
                        compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
                        compute_pass.set_pipeline(pip);
                        compute_pass.dispatch_workgroups(
                            self.grid.size.x.div_ceil(8 * 3),
                            self.grid.size.y.div_ceil(32 * 2),
                            1,
                        );
                        drop(compute_pass);
                        // self.queue.submit(std::iter::once(encoder.finish()));
                        // encoder =
                        //     self.device
                        //         .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        //             label: None,
                        //         });
                    }
                } else if true {
                    let mut compute_pass =
                        encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                            label: Some("Compute pass"),
                            timestamp_writes: None,
                        });
                    compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
                    compute_pass.set_pipeline(&self.compute_pipeline.collide);
                    compute_pass.dispatch_workgroups(
                        self.spawned_particles.div_ceil(GROUP_SIZE),
                        1,
                        1,
                    );
                    drop(compute_pass);
                }
                let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Compute pass"),
                    timestamp_writes: None,
                });
                compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
                compute_pass.set_pipeline(&self.compute_pipeline.finalize);
                compute_pass.dispatch_workgroups(self.spawned_particles.div_ceil(GROUP_SIZE), 1, 1);
                drop(compute_pass);
            }
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
            Event::FileUpdated(_) => {
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
) -> (wgpu::RenderPipeline, ComputePipeline) {
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

    let [update, sort, clear_grid, fill_grid, colorize_grid, collide_grid1, collide_grid2, collide_grid3, collide_grid4, collide_grid5, collide_grid6, collide, finalize] =
        [
            "update_entry",
            "sort_particles_entry",
            "clear_grid_entry",
            "fill_grid_entry",
            "colorize_grid_entry",
            "collide_grid_entry1",
            "collide_grid_entry2",
            "collide_grid_entry3",
            "collide_grid_entry4",
            "collide_grid_entry5",
            "collide_grid_entry6",
            "naive_collisions_entry",
            "finalize_speed_entry",
        ]
        .map(|fn_name| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(fn_name),
                layout: Some(&compute_layout),
                module: &shader,
                entry_point: Some(fn_name),
                compilation_options: Default::default(),
                cache: None,
            })
        });
    (
        render,
        ComputePipeline {
            update,
            sort,
            clear_grid,
            colorize_grid,
            collide_grid1,
            collide_grid2,
            collide_grid3,
            collide_grid4,
            collide_grid5,
            collide_grid6,
            fill_grid,
            collide,
            finalize,
        },
    )
}

struct ComputePipeline {
    pub update: wgpu::ComputePipeline,
    pub collide: wgpu::ComputePipeline,
    pub clear_grid: wgpu::ComputePipeline,
    pub fill_grid: wgpu::ComputePipeline,
    pub colorize_grid: wgpu::ComputePipeline,
    pub collide_grid1: wgpu::ComputePipeline,
    pub collide_grid2: wgpu::ComputePipeline,
    pub collide_grid3: wgpu::ComputePipeline,
    pub collide_grid4: wgpu::ComputePipeline,
    pub collide_grid5: wgpu::ComputePipeline,
    pub collide_grid6: wgpu::ComputePipeline,
    pub finalize: wgpu::ComputePipeline,
    pub sort: wgpu::ComputePipeline,
}
