use std::{sync::mpsc, time::Duration};

use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use winit::event_loop::EventLoopProxy;

use super::application_handler::Event;

pub fn init(event_loop_proxy: &EventLoopProxy<Event>, file_path: &'static str) {
    let event_loop_proxy = event_loop_proxy.clone();

    let _handle = std::thread::spawn(move || {
        let (tx, rx) = mpsc::channel();

        let mut watcher = notify::recommended_watcher(tx).unwrap();
        // RecommendedWatcher::new(
        //     tx,
        //     Config::default().with_poll_interval(Duration::from_secs(2)),
        // )
        // .unwrap();
        watcher
            .watch(file_path.as_ref(), RecursiveMode::Recursive)
            .unwrap();
        println!("Unwraped");

        for res in rx {
            match dbg!(res) {
                Ok(notify::Event {
                    kind: notify::EventKind::Modify(_),
                    ..
                }) => {
                    println!("On file changed");
                    event_loop_proxy
                        .send_event(Event::FileUpdated(file_path))
                        .unwrap();
                }
                Err(e) => println!("Watch error {:?}", e),
                _ => (),
            }
        }

        println!("Finished");
    });
}
