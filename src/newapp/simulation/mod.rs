use std::{collections::vec_deque::Iter, env::current_dir, f32::consts::PI, mem};

use rand::{rngs::StdRng, Rng, SeedableRng};

pub struct Particle {
    pub position: glam::Vec2,
    pub previous_position: glam::Vec2,
    pub velocity: glam::Vec2,
    pub radius: f32,
    pub color: glam::Vec3,
}

const BOUND_RADIUS: f32 = 50.0;
const MAX_PARTICLE_RADIUS: f32 = 0.5;

struct SpatialHash {
    grid_origin: glam::Vec2,
    height: u32,
    width: u32,
    cell_size: f32,

    indexes: Vec<usize>,
    pointers: Vec<usize>,
}

#[derive(Debug)]
struct CellCoords {
    pub x: u32,
    pub y: u32,
}

impl SpatialHash {
    pub fn new(cell_size: f32, grid_origin: glam::Vec2, height: u32, width: u32) -> Self {
        let n_cells = height as usize * width as usize;
        let pointers = vec![0; n_cells + 1];
        let indexes = vec![];
        Self {
            grid_origin,
            height,
            width,
            cell_size,
            indexes,
            pointers,
        }
    }

    pub fn get_cell_coords(&self, position: &glam::Vec2) -> CellCoords {
        let x = ((position.x - self.grid_origin.x) / self.cell_size) as u32;
        let y = ((position.y - self.grid_origin.y) / self.cell_size) as u32;
        CellCoords { x, y }
    }

    pub fn build<'a, I>(&mut self, positions: I)
    where
        I: ExactSizeIterator<Item = &'a glam::Vec2> + Clone,
    {
        let n_positions = positions.len();
        // self.indexes.reserve(n_positions);
        self.indexes.resize(n_positions, 0);
        self.pointers.fill(0);
        for position in positions.clone() {
            let cell_coords = self.get_cell_coords(position);
            let cell_index = self
                .get_cell_index(&cell_coords)
                .min(self.pointers.len() - 1);
            self.pointers[cell_index] += 1;
        }
        let mut sum = 0;
        for pointer in &mut self.pointers {
            sum += *pointer;
            *pointer = sum;
        }
        for (index, position) in positions.enumerate() {
            let cell_coords = self.get_cell_coords(position);
            let cell_index = self
                .get_cell_index(&cell_coords)
                .min(self.pointers.len() - 1);
            self.indexes[self.pointers[cell_index] - 1] = index;
            self.pointers[cell_index] -= 1;
        }
    }

    pub fn get_indexes_by_cell(&self, x: u32, y: u32) -> &[usize] {
        if x >= self.width || y >= self.height {
            return &[];
        }
        let cell_index = self.get_cell_index(&CellCoords { x, y });
        let start = self.pointers[cell_index];
        let end = self.pointers[cell_index + 1];
        &self.indexes[start..end]
    }

    fn get_cell_index(&self, coords: &CellCoords) -> usize {
        coords.x as usize + coords.y as usize * self.width as usize
    }
}

pub struct Simulation {
    particles: Vec<Particle>,
    updates: u64,
    rng: StdRng,
    spatial_hash: SpatialHash,
}

impl Simulation {
    pub fn new() -> Self {
        let seed: [u8; 32] = [
            1u8, 2u8, 3u8, 4u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0,
        ];
        let mut rng: StdRng = SeedableRng::from_seed(seed);
        let particles = vec![Particle {
            position: glam::vec2(0.0, 0.0),
            previous_position: glam::vec2(0.0, 0.0),
            radius: rng.get_random_size(),
            velocity: glam::Vec2::ZERO,
            color: rng.get_random_color(),
        }];
        let cell_size = MAX_PARTICLE_RADIUS * 2.0;
        Self {
            particles,
            updates: 0,
            rng,
            spatial_hash: SpatialHash::new(
                cell_size,
                glam::vec2(-BOUND_RADIUS, -BOUND_RADIUS),
                (BOUND_RADIUS * 2.0 / cell_size) as u32,
                (BOUND_RADIUS * 2.0 / cell_size) as u32,
            ),
        }
    }

    pub fn on_mouse_move(&mut self, position: glam::Vec2) {
        self.particles[0].position = position;
        self.particles[0].velocity = glam::Vec2::ZERO;
    }

    pub fn get_particles(&self) -> &Vec<Particle> {
        &self.particles
    }

    pub fn update(&mut self, dt: f32) {
        self.spawn();
        let steps = 8;
        let dt = dt / steps as f32;
        // dbg!(self
        //     .spatial_hash
        //     .get_cell_coords(&self.particles[0].position));
        self.spatial_hash
            .build(self.particles.iter().map(|it| &it.position));
        for _ in 0..steps {
            self.integrate(dt);
            self.apply_box_constraint(dt);
            self.apply_distance_constraints(dt);

            for particle in &mut self.particles {
                particle.velocity = (particle.position - particle.previous_position) / dt;
            }
        }
        self.updates += 1;
    }

    fn spawn(&mut self) {
        if self.particles.len() < 14750 && self.updates % 4 == 0 {
            let velocity =
                glam::Vec2::from_angle(f32::sin(self.updates as f32 / 40.0) * PI * 0.125) * 40.0;
            let offset = velocity.perp().normalize();
            for i in 0..25 {
                self.particles.push(Particle {
                    position: glam::vec2(-30.0, 20.0) + offset * i as f32,
                    previous_position: glam::Vec2::ZERO,
                    radius: self.rng.get_random_size(),
                    velocity,
                    color: self.rng.get_random_color(),
                });
            }
        }
    }

    fn integrate(&mut self, dt: f32) {
        let gravity = glam::vec2(-6.0, -9.81);
        // let gravity = glam::Vec2::ZERO;
        for particle in &mut self.particles {
            particle.previous_position = particle.position;
            particle.position += particle.velocity * dt + gravity * dt * dt;
            particle.velocity += gravity * dt;
        }
    }

    fn apply_box_constraint(&mut self, dt: f32) {
        let box_size = BOUND_RADIUS;
        for particle in &mut self.particles {
            if particle.position.y - particle.radius < -box_size {
                particle.position.y = -box_size + particle.radius;
                particle.velocity.y = -particle.velocity.y;
            }
            if particle.position.y + particle.radius > box_size {
                particle.position.y = box_size - particle.radius;
                particle.velocity.y = -particle.velocity.y;
            }
            if particle.position.x - particle.radius < -box_size {
                particle.position.x = -box_size + particle.radius;
                particle.velocity.x = -particle.velocity.x;
            }
            if particle.position.x + particle.radius > box_size {
                particle.position.x = box_size - particle.radius;
                particle.velocity.x = -particle.velocity.x;
            }
        }
    }

    fn apply_distance_constraints(&mut self, dt: f32) {
        // for y in 0..(self.spatial_hash.height) {
        //     for x in 0..(self.spatial_hash.width) {
        //         let indicies = self.spatial_hash.get_indexes_by_cell(x, y);
        //         let other_indicies = [(x + 1, y), (x, y + 1), (x + 1, y + 1)]
        //             .into_iter()
        //             .chain(if x > 0 { Some((x - 1, y + 1)) } else { None })
        //             .flat_map(|(xx, yy)| self.spatial_hash.get_indexes_by_cell(xx, yy));
        //         for (i, first_index) in indicies.iter().enumerate() {
        //             for second_index in indicies.iter().skip(i + 1).chain(other_indicies.clone()) {
        //                 let (maxi, mini) = if second_index > first_index {
        //                     (second_index, first_index)
        //                 } else {
        //                     (first_index, second_index)
        //                 };
        //                 let (head, tail) = self.particles.split_at_mut(*maxi);
        //                 let first = &mut head[*mini];
        //                 let second = &mut tail[0];
        //                 apply_distance_constraint(first, second, dt);
        //             }
        //         }
        //     }
        // }
        for i in 0..self.particles.len() {
            //can't be empty since we only enter the loop if len > 0
            let (first, others) = self.particles[i..].split_first_mut().unwrap();
            let cell_coords = self.spatial_hash.get_cell_coords(&first.position);
            let cells = [
                CellCoords {
                    x: cell_coords.x,
                    y: cell_coords.y,
                },
                CellCoords {
                    x: cell_coords.x,
                    y: cell_coords.y + 1,
                },
                CellCoords {
                    x: cell_coords.x,
                    y: (cell_coords.y as i32 - 1) as u32,
                },
                CellCoords {
                    x: cell_coords.x + 1,
                    y: cell_coords.y,
                },
                CellCoords {
                    x: cell_coords.x + 1,
                    y: cell_coords.y + 1,
                },
                CellCoords {
                    x: cell_coords.x + 1,
                    y: (cell_coords.y as i32 - 1) as u32,
                },
                CellCoords {
                    x: (cell_coords.x as i32 - 1) as u32,
                    y: cell_coords.y,
                },
                CellCoords {
                    x: (cell_coords.x as i32 - 1) as u32,
                    y: cell_coords.y + 1,
                },
                CellCoords {
                    x: (cell_coords.x as i32 - 1) as u32,
                    y: (cell_coords.y as i32 - 1) as u32,
                },
            ];

            let indicies = cells
                .iter()
                .map(|coords| self.spatial_hash.get_indexes_by_cell(coords.x, coords.y))
                .flatten()
                .filter(|&&index| index > i);

            for j in indicies {
                let second = &mut others[j - i - 1];
                apply_distance_constraint(first, second, dt);
            }
        }
    }
}
fn apply_distance_constraint(first: &mut Particle, second: &mut Particle, dt: f32) {
    let alpha = 0.00000001;
    let vector = second.position - first.position;
    let direction = vector.normalize_or(glam::vec2(1.0, 0.0));
    let lambda = (vector.length() - first.radius - second.radius) / (2.0 + alpha / (dt * dt));
    if lambda >= 0.0 {
        return;
    }
    let correction = direction * lambda;
    first.position += correction;
    second.position -= correction;
}

trait MyRng {
    fn get_random_size(&mut self) -> f32;

    fn get_random_color(&mut self) -> glam::Vec3;
}

impl MyRng for StdRng {
    fn get_random_size(&mut self) -> f32 {
        self.gen_range(0.7..=1.0) * MAX_PARTICLE_RADIUS
    }

    fn get_random_color(&mut self) -> glam::Vec3 {
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
            glam::vec3(0.996, 0.921, 0.545), // Pa.s.tel Yellow
            glam::vec3(0.835, 0.625, 0.459), // Warm Beige
        ];

        colors[self.gen_range(0..colors.len())]
    }
}
