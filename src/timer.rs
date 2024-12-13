use std::time::{SystemTime, UNIX_EPOCH};

pub struct Timer {
    start_time: u64,
    previous_time: u64,
    current_time: u64,
}

impl Timer {
    pub fn new() -> Self {
        let start_time = current_ms();

        Self {
            start_time,
            current_time: start_time,
            previous_time: start_time,
        }
    }

    pub fn delta_time(&self) -> f32 {
        (self.current_time - self.previous_time) as f32 / 1000.0
    }

    pub fn update(&mut self) {
        self.previous_time = self.current_time;
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
