use std::cmp::min;

use bytemuck::{Pod, Zeroable};
use rand::Rng;

use crate::timer::Timer;

fn get_random_color() -> glam::Vec3 {
    let colors = [
        glam::vec3(0.945, 0.769, 0.058), // Vibrant Yellow
        glam::vec3(0.204, 0.596, 0.859), // Sky Blue
        glam::vec3(0.608, 0.349, 0.714), // Soft Purple
        glam::vec3(0.231, 0.764, 0.392), // Fresh Green
        glam::vec3(0.937, 0.325, 0.314), // Coral Red
        glam::vec3(0.180, 0.800, 0.443), // Mint Green
        glam::vec3(0.996, 0.780, 0.345), // Soft Orange
        glam::vec3(0.556, 0.267, 0.678), // Deep Violet
        glam::vec3(0.870, 0.490, 0.847), // Lavender Pink
        glam::vec3(0.529, 0.808, 0.922), // Light Blue
        glam::vec3(0.996, 0.921, 0.545), // Pastel Yellow
        glam::vec3(0.835, 0.625, 0.459), // Warm Beige
    ];

    colors[rand::thread_rng().gen_range(0..colors.len())]
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct Particle {
    pub position: glam::Vec2,
    pub old_position: glam::Vec2,
    pub radius: f32,
    pub color: glam::Vec3,
}

impl Particle {
    pub fn new(position: glam::Vec2) -> Self {
        Self {
            position,
            old_position: position,
            radius: rand::thread_rng().gen_range(0.5..1.5),
            color: get_random_color(),
        }
    }
}

pub struct Simulation {
    pub particles: Vec<Particle>,
    resolutions: Vec<Resolution>,
    animations: Vec<Animation>,
}

struct Animation {
    final_size: f32,
    speed: f32,
}

impl Simulation {
    pub fn new() -> Self {
        Self {
            particles: vec![],
            resolutions: vec![],
            animations: vec![],
        }
    }

    pub fn spawn(&mut self, position: glam::Vec2) {
        let final_size = rand::thread_rng().gen_range(0.5..1.5);
        self.particles.push(Particle {
            position,
            old_position: position,
            radius: 0.0000001,
            // radius: final_size,
            color: get_random_color(),
        });
        self.animations.push(Animation {
            final_size,
            // speed: rand::thread_rng().gen_range(1..2.5),
            speed: 1.0,
        })
    }

    pub fn update(&mut self, timer: &Timer) {
        let n = 20;
        let dt = timer.delta_time() / n as f32;

        self.animate(timer.delta_time());
        for _ in 0..n {
            self.apply_constraints();
            self.resolve_collisions();
            self.advance(dt);
        }
    }

    fn animate(&mut self, dt: f32) {
        for (ele, animation) in self.particles.iter_mut().zip(&self.animations) {
            if animation.final_size > ele.radius {
                ele.radius += animation.speed * dt;
            }
        }
    }

    fn apply_constraints(&mut self) {
        let bound_radius = 40.0;
        for ele in &mut self.particles {
            let bounds = bound_radius - ele.radius;
            let dst = ele.position.length();
            if dst > bounds {
                ele.position = ele.position.normalize() * bounds;
            }
        }
    }

    fn resolve_collisions(&mut self) {
        self.resolutions.reserve(self.particles.len());
        self.resolutions.clear();
        for _ in 0..self.particles.len() {
            self.resolutions.push(Resolution {
                movement: glam::vec2(0.0, 0.0),
            });
        }
        for i in 0..self.particles.len() {
            for j in (i + 1)..self.particles.len() {
                let ele = &self.particles[i];
                let ele2 = &self.particles[j];
                let dst = (ele.position - ele2.position).length();
                let min_dst = ele.radius + ele2.radius;
                if dst < min_dst {
                    let normal = (ele2.position - ele.position).normalize();
                    let penetration = f32::min(0.01, (min_dst - dst) / 2.0);
                    let res = normal * penetration;
                    self.resolutions[i].movement -= res;
                    self.resolutions[j].movement += res;
                }
            }
        }

        for (ele, res) in self.particles.iter_mut().zip(&self.resolutions) {
            ele.position += res.movement;
        }
    }

    fn advance(&mut self, dt: f32) {
        for ele in &mut self.particles {
            let acceleration = glam::vec2(0.0, -19.0);

            let speed = ele.position - ele.old_position;
            ele.old_position = ele.position;
            ele.position = ele.position + speed + acceleration * dt * dt;
        }
    }
}

struct Resolution {
    movement: glam::Vec2,
}
