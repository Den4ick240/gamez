use std::{string, sync::Arc, time::Instant};

use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::EventLoopProxy,
    window::{Fullscreen, Window, WindowLevel},
};

use super::application_state::ApplicationState;

pub struct Application {
    window: Option<Arc<Window>>,
    state: Option<ApplicationState>,
    instance: wgpu::Instance,
    proxy: EventLoopProxy<Event>,
}

#[derive(Debug)]
pub enum Event {
    FileUpdated(&'static str),
}

impl Application {
    pub fn new(proxy: EventLoopProxy<Event>) -> Self {
        Self {
            instance: wgpu::Instance::new(wgpu::InstanceDescriptor::default()),
            window: None,
            state: None,
            proxy,
        }
    }

    fn get_state(&mut self) -> &mut ApplicationState {
        self.state.as_mut().unwrap()
    }
}

impl ApplicationHandler<Event> for Application {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let size = event_loop.primary_monitor().unwrap().size();
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_transparent(true)
                        .with_window_level(WindowLevel::AlwaysOnTop)
                        .with_resizable(false)
                        .with_inner_size(size), // .with_fullscreen(Some(Fullscreen::Exclusive(
                                                //     event_loop
                                                //         .primary_monitor()
                                                //         .unwrap()
                                                //         .video_modes()
                                                //         .next()
                                                //         .unwrap(),
                                                // ))),
                                                // .with_fullscreen(Some(Fullscreen::Borderless(
                                                //     event_loop.primary_monitor(),
                                                // ))),
                )
                .unwrap(),
        );
        // let _ = window.request_inner_size(PhysicalSize::new(1300, 1300));

        let surface = self
            .instance
            .create_surface(window.clone())
            .expect("Failed to create surface!");

        let state = pollster::block_on(ApplicationState::new(
            &self.instance,
            surface,
            &window.inner_size(),
            &self.proxy,
        ));

        self.window = Some(window);
        self.state = Some(state);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::RedrawRequested => {
                if !self.get_state().should_exit {
                    {
                        let this = &mut *self;
                        this.state.as_mut().unwrap().next_frame();
                    };
                    self.window.as_ref().unwrap().request_redraw();
                } else {
                    event_loop.exit();
                }
            }
            WindowEvent::Resized(physical_size) => self.get_state().on_resize(physical_size),
            WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => self.get_state().on_keyboard_input(event),
            WindowEvent::CursorMoved {
                device_id: _,
                position,
            } => self.get_state().on_cursor_moved(position),
            WindowEvent::MouseInput {
                device_id: _,
                state,
                button,
            } => self.get_state().on_mouse_input(state, button),
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::DroppedFile(path) => self.get_state().on_file_dropped(path),
            _ => (),
        }
    }

    fn user_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, event: Event) {
        self.get_state().on_user_event(&event);
    }
}
