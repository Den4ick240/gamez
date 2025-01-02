use std::sync::mpsc;

use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use winit::event_loop::EventLoopProxy;

use super::application_handler::Event;

pub fn init(event_loop_proxy: &EventLoopProxy<Event>, file_path: &'static str) {
    let event_loop_proxy = event_loop_proxy.clone();

    let _handle = std::thread::spawn(move || {
        let (tx, rx) = mpsc::channel();

        let mut watcher = RecommendedWatcher::new(tx, Config::default()).unwrap();
        watcher
            .watch(file_path.as_ref(), RecursiveMode::Recursive)
            .unwrap();

        for res in rx {
            match res {
                Ok(notify::Event {
                    kind: notify::EventKind::Modify(_),
                    ..
                }) => {
                    event_loop_proxy
                        .send_event(Event::FileUpdated(file_path))
                        .unwrap();
                }
                Err(e) => println!("Watch error {:?}", e),
                _ => (),
            }
        }
    });
}
