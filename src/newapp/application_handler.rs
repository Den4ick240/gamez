use winit::{application::ApplicationHandler, event::WindowEvent, event_loop::EventLoopProxy};

use super::application::Application;

pub struct ApplicationHandlerImpl {
    state: Option<Application>,
    proxy: EventLoopProxy<Event>,
}

#[derive(Debug)]
pub enum Event {
    FileUpdated(&'static str),
}

impl ApplicationHandlerImpl {
    pub fn new(proxy: EventLoopProxy<Event>) -> Self {
        Self { state: None, proxy }
    }

    fn get_state(&mut self) -> &mut Application {
        self.state.as_mut().unwrap()
    }
}

impl ApplicationHandler<Event> for ApplicationHandlerImpl {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.state.is_none() {
            self.state = pollster::block_on(Application::new(event_loop, &self.proxy)).into();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _: winit::window::WindowId,
        event: WindowEvent,
    ) {
        self.get_state().window_event(event_loop, event);
    }

    fn user_event(&mut self, _: &winit::event_loop::ActiveEventLoop, event: Event) {
        self.get_state().on_user_event(&event);
    }
}
