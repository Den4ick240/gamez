use image::GenericImageView;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, KeyEvent},
    event_loop::EventLoopProxy,
    keyboard::{KeyCode, PhysicalKey},
};

use super::{
    application::{Event, T},
    rendering::Renderer,
    simulation::Simulation,
    watch_file,
};

pub struct ApplicationState {
    renderer: Renderer,
    simulation: Simulation,
    pub should_exit: bool,
}

impl ApplicationState {
    pub async fn new(
        instance: &wgpu::Instance,
        surface: wgpu::Surface<'static>,
        size: &PhysicalSize<u32>,
        proxy: &EventLoopProxy<Event>,
    ) -> Self {
        let renderer = Renderer::new(instance, surface, size, proxy).await;
        Self {
            renderer,
            should_exit: false,
            simulation: Simulation::new(),
        }
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
