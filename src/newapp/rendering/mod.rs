mod camera_uniform;
mod simulation;
mod square_mesh;
mod wgpu_utils;

use camera_uniform::CameraUniform;
use simulation::SimulationRenderer;
use square_mesh::SquareMesh;
use winit::{dpi::PhysicalSize, event_loop::EventLoopProxy};

use super::{
    application::{Event, T},
    simulation::Simulation,
    watch_file,
};

pub struct RenderingContext {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
    main_bind_group_layout: wgpu::BindGroupLayout,
    main_bind_group: wgpu::BindGroup,
}

pub struct Renderer {
    context: RenderingContext,
    surface: wgpu::Surface<'static>,
    shader_module: wgpu::ShaderModule,
    square_mesh: SquareMesh,
    simulation_renderer: SimulationRenderer,
    camera_uniform: CameraUniform,
}

const SHADER_FILE: &'static str = "shaders/shader.wgsl";

impl Renderer {
    pub async fn new(
        instance: &wgpu::Instance,
        surface: wgpu::Surface<'static>,
        size: &PhysicalSize<u32>,
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

        let features = wgpu::Features::empty();
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: features,
                    required_limits: Default::default(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .expect("Failed to create device");

        let swapchain_capabilities = surface.get_capabilities(&adapter);
        let selected_format = wgpu::TextureFormat::Bgra8UnormSrgb;
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
        let shader_module = load_shader(&device, &surface_config);

        let camera_uniform = CameraUniform::new(&device, size);

        let main_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("MainBindGroupLayout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0, // camera
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let main_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("MainBindGroup"),
            layout: &main_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_uniform.get_binding_resource(),
            }],
        });

        let context = RenderingContext {
            device,
            queue,
            surface_config,
            main_bind_group,
            main_bind_group_layout,
        };

        let square_mesh = SquareMesh::new(&context.device);

        let simulation_renderer = SimulationRenderer::new(&context, &shader_module);

        Self {
            context,
            surface,
            shader_module,
            square_mesh,
            simulation_renderer,
            camera_uniform,
        }
    }

    pub fn screen_size(&self) -> PhysicalSize<u32> {
        PhysicalSize {
            width: self.context.surface_config.width,
            height: self.context.surface_config.height,
        }
    }

    pub fn on_event(&mut self, event: &Event) {
        match event {
            Event::FileUpdated(SHADER_FILE) => self.load_shader(),
            _ => (),
        }
    }

    fn load_shader(&mut self) {
        self.shader_module = load_shader(&self.context.device, &self.context.surface_config);
        self.simulation_renderer
            .on_shader_updated(&self.context, &self.shader_module);
    }

    pub fn on_resize(&mut self, size: PhysicalSize<u32>) {
        self.context.surface_config.width = size.width;
        self.context.surface_config.height = size.height;
        self.surface
            .configure(&self.context.device, &self.context.surface_config);
        self.camera_uniform.on_resize(&self.context.queue, &size);
    }

    pub fn render(&mut self, simulation: &Simulation, _: f64, _: T) {
        let surface_texture = self
            .surface
            .get_current_texture()
            .expect("Failed to acquire next swap chain texture");

        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .context
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
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            render_pass.set_bind_group(0, &self.context.main_bind_group, &[]);

            self.simulation_renderer.render(
                &mut render_pass,
                &self.context.queue,
                &self.square_mesh,
                &simulation,
            );
        }

        self.context.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();
    }
}

fn load_shader(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> wgpu::ShaderModule {
    println!("Loading shader");
    let text = std::fs::read_to_string(SHADER_FILE).expect("Shader file not found");
    let res = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(&text)),
    });
    println!("Finished Loading shader");
    res
}
