use std::cmp::min;

use bytemuck::{Pod, Zeroable};
use rand::Rng;

use crate::{arrow_renderer::Arrow, timer::Timer};

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
    pub velocity: glam::Vec2,
    pub radius: f32,
    pub color: glam::Vec3,
}

impl Particle {
    pub fn new(position: glam::Vec2) -> Self {
        Self {
            position,
            velocity: glam::vec2(1.0, 1.0),
            radius: rand::thread_rng().gen_range(0.5..1.5),
            color: get_random_color(),
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct Border {
    pub top: f32,
    pub left: f32,
    pub right: f32,
    pub bottom: f32,
}

impl Border {
    pub fn top_right(&self) -> glam::Vec2 {
        glam::vec2(self.right, self.top)
    }

    pub fn top_left(&self) -> glam::Vec2 {
        glam::vec2(self.left, self.top)
    }

    pub fn bottom_right(&self) -> glam::Vec2 {
        glam::vec2(self.right, self.bottom)
    }

    pub fn bottom_left(&self) -> glam::Vec2 {
        glam::vec2(self.left, self.bottom)
    }
}

pub struct Simulation {
    pub particles: Vec<Particle>,
    resolutions: Vec<Resolution>,
    animations: Vec<Animation>,
    pub square_width: f32,
    pub square_height: f32,
    pub spawning_particle: Option<Particle>,
    pub show_debug: bool,
    pub animate: bool,
    tracked_particle: Option<usize>,
    pub borders: Vec<Border>,
}

struct Animation {
    final_size: f32,
}

impl Simulation {
    pub fn new() -> Self {
        let mut balls = get_balls(glam::vec2(12.0, 0.0), 1.0);
        balls.push(Particle {
            position: glam::vec2(-10.0, 0.0),
            velocity: glam::vec2(0.0, 0.0),
            radius: 1.0,
            color: get_random_color(),
        });
        let animations = balls
            .iter()
            .map(|_| Animation { final_size: 1.0 })
            .collect();

        Self {
            // particles: vec![],
            particles: balls,
            resolutions: vec![],
            // animations: vec![],
            animations,
            square_width: 200.0,
            square_height: 200.0,
            spawning_particle: None,
            show_debug: false,
            animate: false,
            tracked_particle: None,
            borders: vec![],
        }
    }

    pub fn on_camera_size(&mut self, width: f32, height: f32, fov: f32) {
        self.square_width = fov;
        self.square_height = fov * height / width;
        self.borders = get_borders(self.square_height, self.square_width);
    }

    pub fn spawn(&mut self, position: glam::Vec2) {
        // let final_size = rand::thread_rng().gen_range(0.5..1.5);
        let final_size = 1.0;
        self.particles.push(Particle {
            position,
            velocity: glam::vec2(0.0, 0.0),
            radius: 0.0000001,
            // radius: final_size,
            color: get_random_color(),
        });
        self.animations.push(Animation { final_size })
    }

    pub fn update(&mut self, timer: &Timer) {
        let n = 40;
        let dt = timer.delta_time() / n as f32;

        self.animate(timer.delta_time());
        for _ in 0..n {
            self.apply_constraints();
            self.resolve_collisions();
            self.advance(dt);
        }
        self.delete_outsiders();
    }

    fn animate(&mut self, dt: f32) {
        for (ele, animation) in self.particles.iter_mut().zip(&self.animations) {
            if !self.animate {
                ele.radius = animation.final_size;
            } else if animation.final_size > ele.radius {
                ele.radius += 1.0 * dt;
            }
        }
    }

    fn delete_outsiders(&mut self) {
        let mut i = 0;
        while i < self.particles.len() {
            let ele = &self.particles[i];
            let right_bound = self.square_width / 2.0 + ele.radius;
            let left_bound = -self.square_width / 2.0 - ele.radius;
            let top_bound = self.square_height / 2.0 + ele.radius;
            let bottom_bound = -self.square_height / 2.0 - ele.radius;
            if ele.position.y > top_bound
                || ele.position.y < bottom_bound
                || ele.position.x > right_bound
                || ele.position.x < left_bound
            {
                if Some(i) == self.tracked_particle {
                    self.tracked_particle = None;
                }
                if Some(self.particles.len() - 1) == self.tracked_particle {
                    self.tracked_particle = Some(i);
                }
                self.particles.swap_remove(i);
                self.animations.swap_remove(i);
                continue;
            }
            i += 1;
        }
    }

    fn collide_point(particle: &mut Particle, point: glam::Vec2, distance: f32) -> bool {
        //     let bounds = bound_radius - ele.radius;
        //     let dst = ele.position.length();
        //     if dst > bounds {
        //         ele.position = ele.position.normalize() * bounds;
        //         ele.velocity = ele.velocity.reflect(ele.position.normalize() * 0.95);
        //     }
        let v = particle.position - point;
        let dst = v.length();
        let norm = v.normalize();
        if dst < distance + particle.radius {
            particle.position = point + norm * (distance + particle.radius);
            particle.velocity = particle.velocity.reflect(norm);
            true
        } else {
            false
        }
    }

    fn apply_constraints(&mut self) {
        for ele in &mut self.particles {
            for border in &self.borders {
                if (border.left..border.right).contains(&ele.position.x) {
                    Self::collide_point(
                        ele,
                        glam::vec2((border.bottom + border.top) / 2.0, ele.position.y),
                        (border.right - border.left) / 2.0,
                    );
                } else if (border.bottom..border.top).contains(&ele.position.y) {
                    Self::collide_point(
                        ele,
                        glam::vec2(ele.position.x, (border.top + border.bottom) / 2.0),
                        (border.top - border.bottom) / 2.0,
                    );
                } else {
                    let _ = Self::collide_point(ele, border.top_right(), 0.0)
                        || Self::collide_point(ele, border.top_left(), 0.0)
                        || Self::collide_point(ele, border.bottom_right(), 0.0)
                        || Self::collide_point(ele, border.bottom_left(), 0.0);
                }
            }
        }
        // for ele in &mut self.particles {
        //     let right_bound = self.square_width / 2.0 - ele.radius;
        //     let left_bound = -self.square_width / 2.0 + ele.radius;
        //     let top_bound = self.square_height / 2.0 - ele.radius;
        //     let bottom_bound = -self.square_height / 2.0 + ele.radius;
        //     if ele.position.y > top_bound {
        //         ele.position.y = top_bound;
        //         ele.velocity.y = -ele.velocity.y;
        //     }
        //     if ele.position.y < bottom_bound {
        //         ele.position.y = bottom_bound;
        //         ele.velocity.y = -ele.velocity.y;
        //     }
        //     if ele.position.x > right_bound {
        //         ele.position.x = right_bound;
        //         ele.velocity.x = -ele.velocity.x;
        //     }
        // if ele.position.x < left_bound {
        //     ele.position.x = left_bound;
        //     ele.velocity.x = -ele.velocity.x;
        // }
        // }
        // let bound_radius = f32::min(self.square_width, self.square_height) / 2.0;
        // for ele in &mut self.particles {
        //     let bounds = bound_radius - ele.radius;
        //     let dst = ele.position.length();
        //     if dst > bounds {
        //         ele.position = ele.position.normalize() * bounds;
        //         ele.velocity = ele.velocity.reflect(ele.position.normalize() * 0.95);
        //     }
        // }
    }

    fn resolve_collisions(&mut self) {
        self.resolutions.reserve(self.particles.len());
        self.resolutions.clear();
        for particle in &self.particles {
            self.resolutions.push(Resolution {
                movement: glam::vec2(0.0, 0.0),
                velocity: particle.velocity,
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

                    let zmf_velocity = get_zmf_velocity(ele, ele2);
                    const DAMPING: f32 = 0.90;
                    let ele_zmf_v = (ele.velocity - zmf_velocity).length() * DAMPING;
                    let ele2_zmf_v = (ele2.velocity - zmf_velocity).length() * DAMPING;
                    let ele_zmf_u = -normal * ele_zmf_v;
                    let ele2_zmf_u = normal * ele2_zmf_v;

                    self.resolutions[i].velocity = zmf_velocity + ele_zmf_u;
                    self.resolutions[j].velocity = zmf_velocity + ele2_zmf_u;
                }
            }
        }

        for (i, (ele, res)) in self.particles.iter_mut().zip(&self.resolutions).enumerate() {
            if Some(i) == self.tracked_particle {
                continue;
            }
            ele.position += res.movement;
            ele.velocity = res.velocity;
        }
    }

    fn advance(&mut self, dt: f32) {
        for (i, ele) in self.particles.iter_mut().enumerate() {
            if Some(i) == self.tracked_particle {
                continue;
            }
            // const gravity = 19.0;
            const GRAVITY: f32 = 0.0;
            let acceleration = glam::vec2(0.0, -GRAVITY) - ele.velocity * 0.5;

            ele.position = ele.position + ele.velocity * dt + 0.5 * acceleration * dt * dt;
            ele.velocity = ele.velocity + acceleration * dt;
        }
    }

    pub fn get_arrows(&self) -> Vec<Arrow> {
        if !self.show_debug {
            return self
                .spawning_particle
                .iter()
                .chain(self.tracked_particle.map(|ele| &self.particles[ele]))
                .map(|ele| Arrow::from(ele))
                .collect();
        }
        self.particles
            .iter()
            .chain(self.spawning_particle.iter())
            .map(|ele| Arrow::from(ele))
            .chain(self.get_zmf_arrows().into_iter())
            .collect::<Vec<Arrow>>()
    }

    pub fn set_spawn_velocity_position(&mut self, position: glam::Vec2) {
        if let Some(tracked_particle) = self.tracked_particle {
            let particle = &mut self.particles[tracked_particle];
            particle.velocity = (position - particle.position) * 3.0;
        }
        if let Some(particle) = &mut self.spawning_particle {
            particle.velocity = (position - particle.position) * 3.0;
        }
    }

    pub fn release_spawn(&mut self) {
        self.tracked_particle = None;
        if let Some(mut particle) = self.spawning_particle.take() {
            self.animations.push(Animation {
                final_size: particle.radius,
            });
            particle.radius = 0.0001;
            self.particles.push(particle);
        }
    }

    pub fn setup_spawning(&mut self, position: glam::Vec2) {
        // let final_size = rand::thread_rng().gen_range(0.5..1.5);
        for (i, particle) in self.particles.iter_mut().enumerate() {
            if (particle.position - position).length() < particle.radius {
                self.tracked_particle = Some(i);
                particle.velocity = glam::vec2(0.0, 0.0);
                return;
            }
        }
        return;
        let final_size = 1.0;
        self.spawning_particle = Some(Particle {
            position,
            velocity: glam::vec2(0.0, 0.0),
            radius: final_size,
            color: get_random_color(),
        })
    }

    pub fn get_zmf_particle(&self) -> Option<Particle> {
        if !self.show_debug {
            return None;
        }
        let particle_count = self.particles.len();
        if particle_count < 2 {
            return None;
        }

        let first = &self.particles[particle_count - 1];
        let second = &self.particles[particle_count - 2];

        Some(get_zmf(first, second))
    }

    fn get_zmf_arrows(&self) -> Vec<Arrow> {
        if !self.show_debug {
            return vec![];
        }
        let particle_count = self.particles.len();
        if particle_count < 2 {
            return vec![];
        }

        let mut first = self.particles[particle_count - 1];
        let mut second = self.particles[particle_count - 2];

        let zmf = get_zmf(&first, &second);

        first.velocity -= zmf.velocity;
        second.velocity -= zmf.velocity;
        first.color = glam::vec3(1.0, 1.0, 1.0);
        second.color = glam::vec3(1.0, 1.0, 1.0);
        vec![Arrow::from(&first), Arrow::from(&second), Arrow::from(&zmf)]
    }

    pub fn get_optional_particles(&self) -> Vec<Particle> {
        self.spawning_particle
            .into_iter()
            .chain(self.get_zmf_particle().into_iter())
            .collect::<Vec<_>>()
    }
}

fn get_zmf(first: &Particle, second: &Particle) -> Particle {
    Particle {
        position: (first.position * first.radius + second.position * second.radius)
            / (first.radius + second.radius),
        velocity: get_zmf_velocity(first, second),
        color: glam::vec3(1.0, 1.0, 1.0),
        radius: 0.3,
    }
}

fn get_zmf_velocity(first: &Particle, second: &Particle) -> glam::Vec2 {
    (first.velocity * first.radius + second.velocity * second.radius)
        / (first.radius + second.radius)
}

impl From<&Particle> for Arrow {
    fn from(particle: &Particle) -> Self {
        Arrow {
            position: particle.position,
            direction: particle.velocity.normalize(),
            color: glam::vec3(1.0, 1.0, 1.0) - particle.color,
            norm: particle.velocity.length() / 3.0,
        }
    }
}

struct Resolution {
    movement: glam::Vec2,
    velocity: glam::Vec2,
}

fn get_balls(at: glam::Vec2, radius: f32) -> Vec<Particle> {
    let placing_radius = radius;
    let dx = placing_radius * 1.73205080757;
    let dy = placing_radius * 2.0;
    let offset = at - glam::vec2(2.5 * dx, 2.5 * dy);

    let mut res: Vec<Particle> = vec![];
    res.reserve(15);
    for x in 0..5 {
        for y in 0..(x + 1) {
            res.push(Particle {
                position: offset
                    + glam::vec2(x as f32 * dx, (y as f32 + 0.5 * (5 - x) as f32) * dy),
                velocity: glam::vec2(0.0, 0.0),
                radius,
                color: get_random_color(),
            });
        }
    }
    res
}

pub fn get_borders(square_height: f32, square_width: f32) -> Vec<Border> {
    const HOLE_RADIUS: f32 = 3.0;
    const BORDER_WIDTH: f32 = 1.0;

    vec![
        Border {
            top: square_height / 2.0 - HOLE_RADIUS,
            left: -square_width / 2.0,
            right: BORDER_WIDTH - square_width / 2.0,
            bottom: -square_height / 2.0 + HOLE_RADIUS,
        },
        Border {
            top: square_height / 2.0 - HOLE_RADIUS,
            left: square_width / 2.0 - BORDER_WIDTH,
            right: square_width / 2.0,
            bottom: -square_height / 2.0 + HOLE_RADIUS,
        },
        Border {
            top: square_height / 2.0,
            bottom: square_height / 2.0 - BORDER_WIDTH,
            left: -square_width / 2.0 + HOLE_RADIUS,
            right: -HOLE_RADIUS / 2.0,
        },
        Border {
            top: square_height / 2.0,
            bottom: square_height / 2.0 - BORDER_WIDTH,
            left: HOLE_RADIUS / 2.0,
            right: square_width / 2.0 - HOLE_RADIUS,
        },
        Border {
            top: BORDER_WIDTH - square_height / 2.0,
            bottom: -square_height / 2.0,
            left: -square_width / 2.0 + HOLE_RADIUS,
            right: -HOLE_RADIUS / 2.0,
        },
        Border {
            top: BORDER_WIDTH - square_height / 2.0,
            bottom: -square_height / 2.0,
            left: HOLE_RADIUS / 2.0,
            right: square_width / 2.0 - HOLE_RADIUS,
        },
    ]
}
