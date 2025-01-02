use std::{sync::Arc, time::Instant};

use image::GenericImageView;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::EventLoopProxy,
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

use super::{application_handler::Event, rendering::Renderer, simulation::Simulation, watch_file};

pub type T = f64;
type Time = Instant;

pub struct Application {
    renderer: Renderer,
    simulation: Simulation,

    fixed_dt: T,
    max_fixed_dt: T,
    last_instant: Time,
    physics_lag: T,

    pub should_exit: bool,
    window: Arc<Window>,
    instance: wgpu::Instance,
}

impl Application {
    pub async fn new(
        event_loop: &winit::event_loop::ActiveEventLoop,
        proxy: &EventLoopProxy<Event>,
    ) -> Self {
        let size = event_loop.primary_monitor().unwrap().size();
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_transparent(true)
                        .with_inner_size(size),
                )
                .unwrap(),
        );

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
        let surface = instance
            .create_surface(window.clone())
            .expect("Failed to create surface!");
        let renderer = Renderer::new(&instance, surface, size, proxy).await;
        Self {
            instance: wgpu::Instance::new(wgpu::InstanceDescriptor::default()),
            renderer,
            should_exit: false,
            simulation: Simulation::new(),
            fixed_dt: 0.016666,
            max_fixed_dt: 0.1,
            last_instant: Instant::now(),
            physics_lag: 0.0,
            window,
        }
    }

    pub fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::RedrawRequested => {
                if !self.should_exit {
                    self.next_frame();
                    self.window.request_redraw();
                } else {
                    event_loop.exit();
                }
            }
            WindowEvent::Resized(physical_size) => self.on_resize(physical_size),
            WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => self.on_keyboard_input(event),
            WindowEvent::CursorMoved {
                device_id: _,
                position,
            } => self.on_cursor_moved(position),
            WindowEvent::MouseInput {
                device_id: _,
                state,
                button,
            } => self.on_mouse_input(state, button),
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::DroppedFile(path) => self.on_file_dropped(path),
            _ => (),
        }
    }

    pub fn next_frame(&mut self) {
        let new_time = Instant::now();
        let frame_time = new_time.duration_since(self.last_instant).as_secs_f64();
        if frame_time > self.max_fixed_dt {
            println!("Lagging")
        }
        let frame_time = frame_time.min(self.max_fixed_dt);
        self.last_instant = new_time;
        self.physics_lag += frame_time;
        self.before_fixed_updates();
        while self.physics_lag >= self.fixed_dt {
            self.fixed_update(self.fixed_dt as f32);
            self.physics_lag -= self.fixed_dt;
        }
        self.after_fixed_updates();
        self.update(frame_time);

        let blend = self.physics_lag / self.fixed_dt;
        self.render(blend, frame_time);
    }

    pub fn on_resize(&mut self, size: PhysicalSize<u32>) {
        self.renderer.on_resize(size);
    }

    pub fn render(&mut self, blend: f64, dt: T) {
        self.renderer.render(&mut self.simulation, blend, dt);
    }

    pub fn before_fixed_updates(&mut self) {}

    pub fn fixed_update(&mut self, dt: f32) {
        self.simulation.update(dt);
    }

    pub fn after_fixed_updates(&mut self) {}

    pub fn update(&mut self, _: T) {}

    pub fn on_keyboard_input(&mut self, event: KeyEvent) {
        match event {
            KeyEvent {
                physical_key: PhysicalKey::Code(KeyCode::KeyC),
                repeat: false,
                state: ElementState::Pressed,
                ..
            } => self.simulation.toggle_collision_detection_mode(),
            KeyEvent {
                physical_key: PhysicalKey::Code(code),
                ..
            } => self.on_physical_key(code),
            _ => (),
        }
    }
    fn on_physical_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Escape => self.should_exit = true,
            _ => (),
        }
    }

    pub fn on_cursor_moved(&mut self, position: PhysicalPosition<f64>) {
        let size = self.renderer.screen_size();
        let scale = 100.0 / size.width as f64;
        let x = position.x - size.width as f64 / 2.0;
        let y = position.y - size.height as f64 / 2.0;
        self.simulation
            .on_mouse_move(glam::vec2((x * scale) as f32, -(y * scale) as f32));
    }

    pub fn on_mouse_input(&mut self, _: winit::event::ElementState, _: winit::event::MouseButton) {}

    pub fn on_user_event(&mut self, event: &Event) {
        self.renderer.on_event(event);
    }

    pub fn on_file_dropped(&mut self, path: std::path::PathBuf) {
        println!("on file dropped {path:?}");
        let img = image::open(path).expect("Failed to load image");
        let (width, height) = img.dimensions();
        println!("width: {width}, height: {height}");
        self.simulation.on_image_loaded(img);
    }
}
