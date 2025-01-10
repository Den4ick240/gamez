use std::time::Instant;
use strum::{EnumCount, EnumIter, IntoEnumIterator};

#[derive(Debug, Copy, Clone, EnumIter, EnumCount)]
pub enum Kind {
    Frame,
    Rendering,
    UpdatesWhole,
    FixedUpdate,
    BulidSpatialHash,
    UpdateParticles,
    Sort,
    CollisionDetectionAndResolution,
}

impl Kind {
    pub const fn as_index(self) -> usize {
        self as usize
    }
}

const BUFFER_SIZE: usize = 60;

#[derive(Clone, Copy)]
struct CircularBuffer {
    array: [f32; BUFFER_SIZE],
    last: usize,
}

impl CircularBuffer {
    pub fn new() -> Self {
        Self {
            array: [0.0; BUFFER_SIZE],
            last: 0,
        }
    }

    pub fn avg(&self) -> f32 {
        let slice = self.get_slice();
        slice.iter().sum::<f32>() / slice.len() as f32
    }

    pub fn max(&self) -> f32 {
        self.get_slice()
            .iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .copied()
            .unwrap_or(0.0)
    }

    pub fn min(&self) -> f32 {
        self.get_slice()
            .iter()
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .copied()
            .unwrap_or(0.0)
    }

    fn get_slice(&self) -> &[f32] {
        &self.array[0..self.last.min(BUFFER_SIZE)]
    }

    pub fn add(&mut self, value: f32) {
        self.last += 1;
        self.array[self.last % BUFFER_SIZE] = value;
    }
}

#[derive(Clone, Copy)]
struct Data {
    min: f32,
    max: f32,
    total: f32,
    frames: u32,
    buff: CircularBuffer,
}

pub struct Profiler {
    variants: [Data; Kind::COUNT],
    starts: [Instant; Kind::COUNT],
}

impl Profiler {
    pub fn new() -> Self {
        Self {
            variants: [Data {
                min: f32::MAX,
                max: f32::MIN,
                total: 0.0,
                frames: 0,
                buff: CircularBuffer::new(),
            }; Kind::COUNT],
            starts: [Instant::now(); Kind::COUNT],
        }
    }

    pub fn start(&mut self, kind: Kind) {
        self.starts[kind.as_index()] = Instant::now();
    }

    pub fn end(&mut self, kind: Kind) {
        let elapsed = self.starts[kind.as_index()].elapsed().as_secs_f32();
        let data = &mut self.variants[kind.as_index()];
        data.min = data.min.min(elapsed);
        data.max = data.max.max(elapsed);
        data.total += elapsed;
        data.buff.add(elapsed);
        data.frames += 1;
    }

    pub fn display(&self) {
        for kind in Kind::iter() {
            let data = &self.variants[kind.as_index()];
            if data.frames > 0 {
                let average = data.buff.avg();
                let min = data.buff.min();
                let max = data.buff.max();
                println!(
                    "{:?}: min: {:.3}ms, max: {:.3}ms, avg: {:.3}ms",
                    kind as Kind,
                    min * 1000.0,
                    max * 1000.0,
                    average * 1000.0
                );
            }
        }
        println!();
    }
}
