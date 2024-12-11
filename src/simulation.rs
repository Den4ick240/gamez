use bytemuck::{Pod, Zeroable};

use crate::timer::Timer;

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct Particle {
    pub position: glam::Vec2,
}

impl Particle {
    pub fn new(position: glam::Vec2) -> Self {
        Self { position }
    }
}

pub struct Simulation {
    pub particles: Vec<Particle>,
}

impl Simulation {
    pub fn new() -> Self {
        Self {
            particles: vec![Particle::new(glam::vec2(0.0, 0.0))],
        }
    }

    pub fn spawn(&mut self, position: glam::Vec2) {
        self.particles.push(Particle::new(position));
    }

    pub fn update(&mut self, timer: &Timer) {}
}
