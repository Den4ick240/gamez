use std::{
    f32::consts::PI,
    fs::{File, OpenOptions},
};

use image::{GenericImageView, Pixel};
use rand::{rngs::StdRng, Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};

pub struct Particle {
    pub position: glam::Vec2,
    pub previous_position: glam::Vec2,
    pub velocity: glam::Vec2,
    pub radius: f32,
}

const BOUND_RADIUS: f32 = 80.0;
const MAX_PARTICLE_RADIUS: f32 = 0.50;

struct SpatialHash {
    grid_origin: glam::Vec2,
    height: u32,
    width: u32,
    cell_size: f32,

    indexes: Vec<usize>,
    pointers: Vec<usize>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
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
    colors: Vec<Color>,
    colors_changed: bool,
}

impl Simulation {
    pub fn new() -> Self {
        let seed: [u8; 32] = [
            1u8, 2u8, 3u8, 4u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0,
        ];
        let rng: StdRng = SeedableRng::from_seed(seed);
        let particles = vec![];
        let colors = load_vector_from_file("colors.bin")
            .unwrap()
            .unwrap_or(vec![]);
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
            colors,
            colors_changed: true,
        }
    }

    pub fn on_mouse_move(&mut self, _position: glam::Vec2) {}

    pub fn get_particles(&self) -> &Vec<Particle> {
        &self.particles
    }

    pub fn get_colors(&mut self) -> Option<&Vec<Color>> {
        if self.colors_changed {
            self.colors_changed = false;
            Some(&self.colors)
        } else {
            None
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.spawn();
        let steps = 8;
        let dt = dt / steps as f32;
        self.spatial_hash
            .build(self.particles.iter().map(|it| &it.position));
        for _ in 0..steps {
            self.integrate(dt);
            self.apply_box_constraint(dt);
            self.apply_distance_constraints(dt);

            for particle in &mut self.particles {
                particle.velocity = (particle.position - particle.previous_position) / dt;
                let damp = 80.0 / particle.velocity.length();
                if damp < 1.0 {
                    particle.velocity *= damp;
                }
            }
        }
        self.updates += 1;
    }

    fn spawn(&mut self) {
        let count = 38900;
        if self.particles.len() < count && self.updates % 4 == 0 {
            let velocity =
                glam::Vec2::from_angle(f32::sin(self.updates as f32 / 40.0) * PI * 0.125) * 80.0;
            let offset = velocity.perp().normalize();
            for i in 0..45 {
                self.particles.push(Particle {
                    position: glam::vec2(-5.0, -55.0) + offset * i as f32,
                    previous_position: glam::Vec2::ZERO,
                    radius: self.rng.get_random_size(),
                    velocity,
                });
                if self.particles.len() > self.colors.len() {
                    self.colors_changed = true;
                    self.colors.push(self.rng.get_random_color());
                }
            }
        }
    }

    fn integrate(&mut self, dt: f32) {
        let len = self.particles.len();
        let gravity = glam::vec2(-6.0, -9.81)
            * if len < 14000 || len > 20000 && len < 29000 {
                -1.0
            } else {
                1.0
            };
        // let gravity = glam::Vec2::ZERO;
        for particle in &mut self.particles {
            particle.previous_position = particle.position;
            particle.position += particle.velocity * dt + gravity * dt * dt;
            particle.velocity += gravity * dt;
        }
    }

    fn apply_box_constraint(&mut self, _dt: f32) {
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
        for y in 0..(self.spatial_hash.height) {
            for x in 0..(self.spatial_hash.width) {
                let indicies = self.spatial_hash.get_indexes_by_cell(x, y);
                let other_indicies = [(x + 1, y), (x, y + 1), (x + 1, y + 1)]
                    .into_iter()
                    .chain(if x > 0 { Some((x - 1, y + 1)) } else { None })
                    .flat_map(|(xx, yy)| self.spatial_hash.get_indexes_by_cell(xx, yy));
                for (i, first_index) in indicies.iter().enumerate() {
                    for second_index in indicies.iter().skip(i + 1).chain(other_indicies.clone()) {
                        let (maxi, mini) = if second_index > first_index {
                            (second_index, first_index)
                        } else {
                            (first_index, second_index)
                        };
                        let (head, tail) = self.particles.split_at_mut(*maxi);
                        let first = &mut head[*mini];
                        let second = &mut tail[0];
                        apply_distance_constraint(first, second, dt);
                    }
                }
            }
        }
        //     for i in 0..self.particles.len() {
        //         //can't be empty since we only enter the loop if len > 0
        //         let (first, others) = self.particles[i..].split_first_mut().unwrap();
        //         let cell_coords = self.spatial_hash.get_cell_coords(&first.position);
        //         let cells = [
        //             CellCoords {
        //                 x: cell_coords.x,
        //                 y: cell_coords.y,
        //             },
        //             CellCoords {
        //                 x: cell_coords.x,
        //                 y: cell_coords.y + 1,
        //             },
        //             CellCoords {
        //                 x: cell_coords.x,
        //                 y: (cell_coords.y as i32 - 1) as u32,
        //             },
        //             CellCoords {
        //                 x: cell_coords.x + 1,
        //                 y: cell_coords.y,
        //             },
        //             CellCoords {
        //                 x: cell_coords.x + 1,
        //                 y: cell_coords.y + 1,
        //             },
        //             CellCoords {
        //                 x: cell_coords.x + 1,
        //                 y: (cell_coords.y as i32 - 1) as u32,
        //             },
        //             CellCoords {
        //                 x: (cell_coords.x as i32 - 1) as u32,
        //                 y: cell_coords.y,
        //             },
        //             CellCoords {
        //                 x: (cell_coords.x as i32 - 1) as u32,
        //                 y: cell_coords.y + 1,
        //             },
        //             CellCoords {
        //                 x: (cell_coords.x as i32 - 1) as u32,
        //                 y: (cell_coords.y as i32 - 1) as u32,
        //             },
        //         ];
        //
        //         let indicies = cells
        //             .iter()
        //             .map(|coords| self.spatial_hash.get_indexes_by_cell(coords.x, coords.y))
        //             .flatten()
        //             .filter(|&&index| index > i);
        //
        //         for j in indicies {
        //             let second = &mut others[j - i - 1];
        //             apply_distance_constraint(first, second, dt);
        //         }
        //     }
    }

    pub fn on_image_loaded(&mut self, img: image::DynamicImage) {
        let (width, height) = img.dimensions();
        let width = width as f32;
        let height = height as f32;
        let dim = width.max(height) as f32;
        let offset = glam::vec2(
            ((dim - width as f32) / 2.0).min(0.0),
            ((dim - height as f32) / 2.0).min(0.0),
        );
        for i in 0..self.particles.len() {
            let particle = &self.particles[i];
            let pos = (glam::vec2(particle.position.x, -particle.position.y)
                + glam::vec2(BOUND_RADIUS, BOUND_RADIUS))
                * dim
                / (BOUND_RADIUS * 2.0);
            if pos.x < offset.x
                || pos.x > offset.x + width
                || pos.y < offset.y
                || pos.y > offset.y + height
            {
                self.colors[i] = Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                };
            } else {
                let pos = pos - offset;
                let color = img.get_pixel(pos.x as u32, pos.y as u32);
                let (r, g, b, _) = color.channels4();
                let color = Color {
                    r: r as f32 / 255.0,
                    g: g as f32 / 255.0,
                    b: b as f32 / 255.0,
                };
                self.colors[i] = color;
            }
        }
        save_vector_to_file(&self.colors, "colors.bin").unwrap();
        self.colors_changed = true;
    }
}

impl Default for Simulation {
    fn default() -> Self {
        Self::new()
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

    fn get_random_color(&mut self) -> Color;
}

impl MyRng for StdRng {
    fn get_random_size(&mut self) -> f32 {
        self.gen_range(0.7..=1.0) * MAX_PARTICLE_RADIUS
    }

    fn get_random_color(&mut self) -> Color {
        let colors = [
            Color {
                r: 0.945,
                g: 0.769,
                b: 0.058,
            }, // Vibrant Yellow
            Color {
                r: 0.204,
                g: 0.596,
                b: 0.859,
            }, // Sky Blue
            Color {
                r: 0.608,
                g: 0.349,
                b: 0.714,
            }, // Soft Purple
            Color {
                r: 0.231,
                g: 0.764,
                b: 0.392,
            }, // Fresh Green
            Color {
                r: 0.937,
                g: 0.325,
                b: 0.314,
            }, // Coral Red
            Color {
                r: 0.180,
                g: 0.800,
                b: 0.443,
            }, // Mint Green
            Color {
                r: 0.996,
                g: 0.780,
                b: 0.345,
            }, // Soft Orange
            Color {
                r: 0.556,
                g: 0.267,
                b: 0.678,
            }, // Deep Violet
            Color {
                r: 0.870,
                g: 0.490,
                b: 0.847,
            }, // Lavender Pink
            Color {
                r: 0.529,
                g: 0.808,
                b: 0.922,
            }, // Light Blue
            Color {
                r: 0.996,
                g: 0.921,
                b: 0.545,
            }, // Pa.s.tel Yellow
            Color {
                r: 0.835,
                g: 0.625,
                b: 0.459,
            }, // Warm Beige
        ];

        colors[self.gen_range(0..colors.len())]
    }
}

fn save_vector_to_file(vec: &Vec<Color>, file_path: &str) -> io::Result<()> {
    let mut file = File::create(file_path)?;
    let encoded: Vec<u8> = bincode::serialize(vec).unwrap();
    file.write_all(&encoded)?;
    Ok(())
}

fn load_vector_from_file(file_path: &str) -> io::Result<Option<Vec<Color>>> {
    if let Ok(mut file) = OpenOptions::new().read(true).open(file_path) {
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        let vec: Vec<Color> = bincode::deserialize(&buffer).unwrap();
        Ok(Some(vec))
    } else {
        Ok(None)
    }
}
