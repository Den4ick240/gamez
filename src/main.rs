// mod app;
// mod arrow_renderer;
// mod border_renderer;
// mod camera_uniform;
// mod joint_renderer;
// mod mouse_renderer;
// mod mouse_uniform;
mod newapp;
// mod simulation;
// mod simulation_renderer;
// mod square_mesh;
// mod timer;
// mod wgpu_utils;

use newapp::application::Event;
use winit::event_loop::{ControlFlow, EventLoop, EventLoopBuilder};

fn main() {
    pollster::block_on(run())
}

async fn run() {
    let event_loop: EventLoop<Event> = EventLoop::with_user_event().build().unwrap();
    let proxy = event_loop.create_proxy();

    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = newapp::application::Application::new(proxy);

    event_loop.run_app(&mut app).expect("Failed to run app")
}
