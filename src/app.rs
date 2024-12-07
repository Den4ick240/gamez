use std::sync::Arc;

use winit::{
    application::ApplicationHandler, dpi::PhysicalSize, event::WindowEvent, keyboard::PhysicalKey,
    window::Window,
};

pub struct AppState {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub surface: wgpu::Surface<'static>,
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

        // let egui_renderer = EguiRenderer::new(&device, surface_config.format, None, 1, window);
        //
        // let scale_factor = 1.0;

        Self {
            device,
            queue,
            surface,
            surface_config,
            // egui_renderer,
            // scale_factor,
        }
    }

    fn resize_surface(&mut self, width: u32, height: u32) {
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

    fn handle_resized(&mut self, width: u32, height: u32) {
        self.state.as_mut().unwrap().resize_surface(width, height);
    }

    fn handle_redraw(&mut self) {
        let state = self.state.as_mut().unwrap();

        // let screen_descriptor = ScreenDescriptor {
        //     size_in_pixels: [state.surface_config.width, state.surface_config.height],
        //     pixels_per_point: self.window.as_ref().unwrap().scale_factor() as f32
        //         * state.scale_factor,
        // };

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
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
        // // let egui render to process the event first
        // self.state
        //     .as_mut()
        //     .unwrap()
        //     .egui_renderer
        //     .handle_input(self.window.as_ref().unwrap(), &event);

        match event {
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
