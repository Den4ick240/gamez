use std::sync::Arc;

use winit::{
    application::ApplicationHandler, dpi::PhysicalSize, event::WindowEvent, keyboard::PhysicalKey,
    window::Window,
};

use crate::{
    camera_uniform::CameraState, mouse_uniform::MouseState, square_mesh::SquareMesh, timer::Timer,
};

pub struct AppState {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub surface: wgpu::Surface<'static>,
    pub mouse_state: MouseState,
    pub camera_state: CameraState,
    pub square_mesh: SquareMesh,
    pub timer: Timer,
    pub bind_group: wgpu::BindGroup,
    mouse_pipeline: wgpu::RenderPipeline,
}

impl AppState {
    async fn new(
        instance: &wgpu::Instance,
        surface: wgpu::Surface<'static>,
        window: &Window,
    ) -> Self {
        let size = window.inner_size();
        let power_pref = wgpu::PowerPreference::HighPerformance;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: power_pref,
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

        let square_mesh = SquareMesh::new(&device);
        let timer = Timer::new();
        let mouse_state = MouseState::new(&device);
        let camera_state = CameraState::new(&device, size.width as f32, size.height as f32);

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("BindGroupLayout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0, // camera
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1, // mouse
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("BindGroup"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_state.get_binding_resource(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: mouse_state.get_bind_group_resource(),
                },
            ],
        });

        let mouse_shader =
            device.create_shader_module(wgpu::include_wgsl!("../shaders/mouse.wgsl"));
        let mouse_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("MousePipelineLayout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        let mouse_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("MousePipeline"),
            layout: Some(&mouse_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &mouse_shader,
                buffers: &[SquareMesh::desc()],
                entry_point: None,
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &mouse_shader,
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
            device,
            queue,
            surface,
            surface_config,
            mouse_state,
            camera_state,
            square_mesh,
            timer,
            bind_group,
            mouse_pipeline,
        }
    }

    fn resize_surface(&mut self, width: u32, height: u32) {
        self.camera_state.set_size(width as f32, height as f32);
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(&self.device, &self.surface_config);
    }
}

pub struct App {
    instance: wgpu::Instance,
    state: Option<AppState>,
    window: Option<Arc<Window>>,
}
impl App {
    pub fn new() -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
        Self {
            instance,
            state: None,
            window: None,
        }
    }

    async fn set_window(&mut self, window: Window) {
        let window = Arc::new(window);
        let initial_width = 1360;
        let initial_height = 768;

        let _ = window.request_inner_size(PhysicalSize::new(initial_width, initial_height));

        let surface = self
            .instance
            .create_surface(window.clone())
            .expect("Failed to create surface!");

        let state = AppState::new(&self.instance, surface, &window).await;

        self.window.get_or_insert(window);
        self.state.get_or_insert(state);
    }

    pub fn get_app_state(&mut self) -> &mut AppState {
        self.state.as_mut().unwrap()
    }

    fn handle_resized(&mut self, width: u32, height: u32) {
        self.state.as_mut().unwrap().resize_surface(width, height);
    }

    fn handle_redraw(&mut self) {
        let state = self.state.as_mut().unwrap();

        state.timer.update();
        state.mouse_state.update(&state.timer);
        state.mouse_state.write_buffer(&state.queue);
        state.camera_state.write_buffer(&state.queue);

        let surface_texture = state
            .surface
            .get_current_texture()
            .expect("Failed to acquire next swap chain texture");

        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = state
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
            render_pass.set_pipeline(&state.mouse_pipeline);
            render_pass.set_bind_group(0, &state.bind_group, &[]);
            render_pass.set_vertex_buffer(0, state.square_mesh.vertex_buffer.slice(..));
            render_pass.draw(0..4, 0..1);
        }

        state.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = event_loop
            .create_window(Window::default_attributes())
            .unwrap();
        pollster::block_on(self.set_window(window))
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CursorMoved {
                device_id: _,
                position,
            } => self
                .get_app_state()
                .mouse_state
                .set_position(position.into()),

            WindowEvent::MouseInput {
                device_id: _,
                state,
                button: _,
            } => self
                .get_app_state()
                .mouse_state
                .set_is_clicked(state == winit::event::ElementState::Pressed),

            WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => match event.physical_key {
                PhysicalKey::Code(winit::keyboard::KeyCode::Escape) => {
                    println!("The escape key was pressed; stopping");
                    event_loop.exit();
                }
                _ => (),
            },

            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            }

            WindowEvent::RedrawRequested => {
                self.handle_redraw();

                self.window.as_ref().unwrap().request_redraw();
            }

            WindowEvent::Resized(new_size) => {
                self.handle_resized(new_size.width, new_size.height);
            }

            _ => (),
        }
    }
}
