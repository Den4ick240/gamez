use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoopProxy},
};

use super::application::Application;

pub struct WindowedApplicationHandler {
    inner_handler: Option<Application<T>>,
    proxy: EventLoopProxy<T>,
}

impl<T> WindowedApplicationHandler {
    pub fn new(event_loop: EventLoop<T>) -> Self {
        Self { state: None, proxy }
    }

    fn get_state(&mut self) -> &mut Application {
        self.state.as_mut().unwrap()
    }
}

impl ApplicationHandler<Event> for WindowedApplicationHandler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_none() {
            self.state = pollster::block_on(Application::new(event_loop, &self.proxy)).into();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _: winit::window::WindowId,
        event: WindowEvent,
    ) {
        self.get_state().window_event(event_loop, event);
    }

    fn user_event(&mut self, _: &ActiveEventLoop, event: Event) {
        self.get_state().on_user_event(&event);
    }
}

pub trait Application<T> {
    async fn new(event_loop: &ActiveEventLoop) -> Self;
}
