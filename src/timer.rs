use std::time::{SystemTime, UNIX_EPOCH};

pub struct Timer {
    start_time: u64,
    current_time: u64,
}

impl Timer {
    pub fn new() -> Self {
        let start_time = current_ms();

        Self {
            start_time,
            current_time: start_time,
        }
    }

    pub fn update(&mut self) {
        self.current_time = current_ms()
    }

    pub fn ms_since_start(&self) -> u64 {
        return self.current_time - self.start_time;
    }
}

fn current_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_millis()
        .try_into()
        .unwrap()
}
