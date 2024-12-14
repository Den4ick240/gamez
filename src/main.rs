mod app;
mod arrow_renderer;
mod border_renderer;
mod camera_uniform;
mod mouse_renderer;
mod mouse_uniform;
mod simulation;
mod simulation_renderer;
mod square_mesh;
mod timer;
mod wgpu_utils;

use winit::event_loop::{ControlFlow, EventLoop};

fn main() {
    pollster::block_on(run())
}

async fn run() {
    let event_loop = EventLoop::new().unwrap();

    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = app::App::new();

    event_loop.run_app(&mut app).expect("Failed to run app")
}
