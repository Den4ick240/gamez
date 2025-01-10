mod box_constraint;
mod integrator;
mod sorted_store;
mod spatial_hash;

use std::{
    f32::consts::PI,
    fs::{File, OpenOptions},
    ops::Div,
    sync::{Barrier, Mutex},
    time::Instant,
};

use box_constraint::BoxConstraint;
use glam::{uvec2, vec2, UVec2, Vec2};
use image::{GenericImageView, Pixel};
use itertools::Itertools;
use rand::{rngs::StdRng, Rng, SeedableRng};
use rayon::{ThreadPool, ThreadPoolBuilder};
use serde::{Deserialize, Serialize};
use spatial_hash::{
    fixed_size_grid::FixedSizeGrid,
    pointer_hash::{HashReference, PointerHash},
    sorting_hash::{Positioned, SortingHash},
    SpatialGrid,
};
use std::io::{self, Read, Write};

use super::profiler::{self, Profiler};

pub struct Particle {
    pub initial_id: usize,
    pub position: Vec2,
    pub velocity: Vec2,
    pub radius: f32,
}

const BOUND_RADIUS: f32 = 300.0;
const MAX_PARTICLE_RADIUS: f32 = 1.0;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

pub struct Simulation {
    particles: Vec<Particle>,
    previous_positions: Vec<Vec2>,
    updates: u64,
    rng: StdRng,
    spatial_hash: PointerHash<FixedSizeGrid>,
    sorting_hash: SortingHash<FixedSizeGrid>,
    pub colors: Vec<Color>,
    colors_changed: bool,
    collision_detection_mode: u32,
    elapsed: Option<f64>,
    thread_pool: ThreadPool,
}

const NUM_THREADS: usize = 8;

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

        let min_cell_size = MAX_PARTICLE_RADIUS * 2.0;
        let grid = FixedSizeGrid::new(min_cell_size, BoxConstraint::around_center(BOUND_RADIUS));

        Self {
            particles,
            previous_positions: vec![],
            updates: 0,
            rng,
            spatial_hash: PointerHash::new(grid.clone()),
            sorting_hash: SortingHash::new(grid),
            colors,
            colors_changed: true,
            collision_detection_mode: 1,
            elapsed: None,
            thread_pool: ThreadPoolBuilder::new()
                .num_threads(NUM_THREADS)
                .build()
                .unwrap(),
        }
    }

    pub fn toggle_collision_detection_mode(&mut self) {
        self.collision_detection_mode = (self.collision_detection_mode + 1) % 2;
        println!("Collision mode: {}", self.collision_detection_mode);
    }

    pub fn on_mouse_move(&mut self, _position: Vec2) {}

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

    pub fn update(&mut self, dt: f32, profiler: &mut Profiler) {
        self.spawn();
        let steps = 8;

        match self.collision_detection_mode {
            0 => {
                profiler.start(profiler::Kind::BulidSpatialHash);
                self.spatial_hash
                    .build(self.particles.iter().map(|it| &it.position));
                profiler.end(profiler::Kind::BulidSpatialHash);
            }
            _ => {
                profiler.start(profiler::Kind::Sort);
                self.sorting_hash.build(&mut self.particles);
                profiler.end(profiler::Kind::Sort);
            } // 0 => self.particles.sort_by_cached_key(|p| {
              //     self.spatial_hash.grid().get_position_cell_index(p.position)
              // }),
              // _ => self.particles.sort_unstable_by_key(|p| {
              //     self.spatial_hash.grid().get_position_cell_index(p.position)
              // }),
        }
        {
            let dt = dt / steps as f32;
            for _ in 0..steps {
                profiler.start(profiler::Kind::UpdateParticles);
                self.update_particles(dt);
                profiler.end(profiler::Kind::UpdateParticles);
                profiler.start(profiler::Kind::CollisionDetectionAndResolution);
                self.apply_distance_constraints(dt);
                // self.apply_fancy(dt);
                profiler.end(profiler::Kind::CollisionDetectionAndResolution);
                for (particle, previous_position) in self
                    .particles
                    .iter_mut()
                    .zip(self.previous_positions.iter())
                {
                    particle.velocity = (particle.position - previous_position) / dt;
                    let damp = 100.0 / particle.velocity.length();
                    if damp < 1.0 {
                        particle.velocity *= damp;
                    }
                }
            }
        }
        self.updates += 1;
    }

    // fn apply_fancy(&mut self, dt: f32) {
    //     let height = self.spatial_hash.grid().size().y as usize;
    //     let num_threads = NUM_THREADS.min(height / 4);
    //
    //     let even_height = height & !1;
    //     let even_chunk_size = (even_height / num_threads) & !1;
    //
    //     // always overflow < num_threads * 2 and is even
    //     let overflow = even_height - even_chunk_size * num_threads;
    //
    //     // always < num_threads and divides evenly
    //     let num_chunks_to_increase = overflow / 2; // some chunks will be increased by 2
    //
    //     let small_chunk_size = even_chunk_size;
    //     let big_chunk_suze = even_chunk_size + 2;
    //
    //     let chunk_starts = (0..num_chunks_to_increase)
    //         .map(|i| (i * big_chunk_suze) as u32)
    //         .chain(
    //             ((num_chunks_to_increase + 1)..num_threads)
    //                 .map(|i| (i * small_chunk_size + num_chunks_to_increase * 2) as u32),
    //         );
    //
    //     let mut hash = self.spatial_hash.reference(&mut self.particles);
    //
    //     // unshared_chunk[i] is processed only by thread i
    //     let mut unshared_chunks = Vec::with_capacity(num_threads);
    //     // shared_row[i] is processed by thread i + 1 first, and then by thread i - 1
    //     let mut shared_rows = Vec::with_capacity(num_threads - 1);
    //
    //     self.thread_pool.scope(|s| {
    //         let mut last_chunk = chunk_starts.skip(1).fold(hash, |hash, end| {
    //             let (chunk, hash) = hash.split_at_row(end);
    //             let (overlap, hash) = hash.split_at_row(end + 1);
    //             unshared_chunks.push(chunk);
    //             shared_rows.push(Mutex::new(overlap));
    //             hash
    //         });
    //
    //         let (first_chunk, other_chunks) = unshared_chunks.split_first_mut().unwrap();
    //
    //         // we want lower rows to start earlier and immideately lock their first shared row
    //         Self::run_end(&shared_rows[shared_rows.len() - 1], &mut last_chunk);
    //
    //         for (i, other_chunk) in other_chunks.iter_mut().enumerate().rev() {
    //             Self::run_middle(&shared_rows[i], other_chunk, &shared_rows[i + 1]);
    //         }
    //
    //         Self::run_start(first_chunk, &shared_rows[0]);
    //     });
    //
    //     let chunk_size = height / num_threads;
    //     let last_chunk_size = height - chunk_size * (num_threads - 1);
    // }
    //
    // fn run_start(
    //     free: &mut HashReference<'_, Particle, FixedSizeGrid>,
    //     overlap: &Mutex<HashReference<'_, Particle, FixedSizeGrid>>,
    // ) {
    //     let mut guard = overlap.lock().unwrap();
    //     guard.get(uvec2(0, 0));
    //     println!()
    // }
    // fn run_middle(
    //     pre_overlap: &Mutex<HashReference<'_, Particle, FixedSizeGrid>>,
    //     free: &mut HashReference<'_, Particle, FixedSizeGrid>,
    //     overlap: &Mutex<HashReference<'_, Particle, FixedSizeGrid>>,
    // ) {
    // }
    // fn run_end(
    //     pre_overlap: &Mutex<HashReference<'_, Particle, FixedSizeGrid>>,
    //     free: &mut HashReference<'_, Particle, FixedSizeGrid>,
    // ) {
    // }
    //
    // fn run_rows(
    //     row1: &mut HashReference<'_, Particle, FixedSizeGrid>,
    //     row2: &mut HashReference<'_, Particle, FixedSizeGrid>,
    // ) {
    // }

    fn spawn(&mut self) {
        // let count = 8900;
        // let count = 35200; //175
        let count = 107500;
        if self.particles.len() < count && self.updates % 4 == 0 {
            let velocity =
                // glam::Vec2::from_angle(f32::sin(self.updates as f32 / 40.0) * PI * 0.125) * 80.0;
                vec2(20.0, 0.0);
            let offset = velocity.perp().normalize() * 2.0;
            for i in 0..85 {
                self.particles.push(Particle {
                    initial_id: self.particles.len(),
                    position: vec2(-170.0, 40.0) + offset * i as f32,
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

    fn update_particles(&mut self, dt: f32) {
        let len = self.particles.len();
        let gravity = vec2(0.0, -30.00);
        // let gravity = glam::vec2(0.0, -30.81)
        //     * if len < 34000 || len > 50000 && len < 69000 {
        //         -1.0
        //     } else {
        //         1.0
        //     };
        let constraint = BoxConstraint::around_center(BOUND_RADIUS);
        self.previous_positions.reserve(self.particles.len());
        // we will write to the whole length of this vec in the following code, without reading
        unsafe { self.previous_positions.set_len(self.particles.len()) };

        let chunk_size = self.particles.len().div_ceil(NUM_THREADS);
        self.thread_pool.scope(|s| {
            self.particles
                .chunks_mut(chunk_size)
                .zip(self.previous_positions.chunks_mut(chunk_size))
                .for_each(|(particles, previous_positions)| {
                    s.spawn(move |_| {
                        particles.iter_mut().zip(previous_positions).for_each(
                            |(particle, previous_position)| {
                                *previous_position = particle.position;
                                particle.velocity += gravity * dt;
                                particle.position += particle.velocity * dt;
                                constraint.apply(particle, dt);
                            },
                        );
                    })
                });
        });
    }

    fn apply_distance_constraints(&mut self, dt: f32) {
        // let now = Instant::now();
        match self.collision_detection_mode {
            0 => self.apply_stagger_threads(dt),
            _ => self.apply_sorted_threads(dt),
            // // _ => self.apply_stagger_threads(dt),
        }
        // self.apply_stagger_threads(dt);
        // let elapsed = now.elapsed().as_secs_f64();
        // self.elapsed = self
        //     .elapsed
        //     .map_or(elapsed, |it| it * 0.9 + elapsed * 0.1)
        //     .into();
    }

    fn apply_stagger_threads(&mut self, dt: f32) {
        const N: usize = NUM_THREADS;

        let height = self.spatial_hash.grid().size().y;

        let chunk_size = height as usize / N;

        let data_ptr = self.particles.as_mut_ptr() as u64;

        let barrier = &Barrier::new(N);
        let spatial_hash = &self.spatial_hash;
        self.thread_pool.scope(|s| {
            for n in 0..(N - 1) {
                let start = n * chunk_size;
                let end = (start + chunk_size) as u32;
                let start1 = if start % 2 == 0 { start } else { start + 1 } as u32;
                let start2 = if start % 2 == 0 { start + 1 } else { start } as u32;
                s.spawn(move |_| {
                    Self::run_collision(
                        (start1..end).step_by(2),
                        spatial_hash,
                        data_ptr as *mut Particle,
                        dt,
                    );

                    barrier.wait();

                    Self::run_collision(
                        (start2..end).step_by(2),
                        spatial_hash,
                        data_ptr as *mut Particle,
                        dt,
                    );
                });
            }

            let start = (N - 1) * chunk_size;
            let end = height;
            let start1 = if start % 2 == 0 { start } else { start + 1 } as u32;
            let start2 = if start % 2 == 0 { start + 1 } else { start } as u32;
            Self::run_collision(
                (start1..end).step_by(2),
                spatial_hash,
                data_ptr as *mut Particle,
                dt,
            );

            barrier.wait();

            Self::run_collision(
                (start2..end).step_by(2),
                spatial_hash,
                data_ptr as *mut Particle,
                dt,
            );
        });
    }

    fn apply_sorted_threads(&mut self, dt: f32) {
        const N: usize = NUM_THREADS;

        let height = self.spatial_hash.grid().size().y;

        let chunk_size = height as usize / N;

        let data_ptr = self.particles.as_mut_ptr() as u64;

        let barrier = &Barrier::new(N);
        let spatial_hash = &self.sorting_hash;
        self.thread_pool.scope(|s| {
            for n in 0..(N - 1) {
                let start = n * chunk_size;
                let end = (start + chunk_size) as u32;
                let start1 = if start % 2 == 0 { start } else { start + 1 } as u32;
                let start2 = if start % 2 == 0 { start + 1 } else { start } as u32;
                s.spawn(move |_| {
                    Self::run_sorted_collision(
                        (start1..end).step_by(2),
                        spatial_hash,
                        data_ptr as *mut Particle,
                        dt,
                    );

                    barrier.wait();

                    Self::run_sorted_collision(
                        (start2..end).step_by(2),
                        spatial_hash,
                        data_ptr as *mut Particle,
                        dt,
                    );
                });
            }

            let start = (N - 1) * chunk_size;
            let end = height;
            let start1 = if start % 2 == 0 { start } else { start + 1 } as u32;
            let start2 = if start % 2 == 0 { start + 1 } else { start } as u32;
            Self::run_sorted_collision(
                (start1..end).step_by(2),
                spatial_hash,
                data_ptr as *mut Particle,
                dt,
            );

            barrier.wait();

            Self::run_sorted_collision(
                (start2..end).step_by(2),
                spatial_hash,
                data_ptr as *mut Particle,
                dt,
            );
        });
    }

    pub fn on_image_loaded(&mut self, img: image::DynamicImage) {
        let (width, height) = img.dimensions();
        let width = width as f32;
        let height = height as f32;
        let dim = width.max(height) as f32;
        let offset = vec2(
            ((dim - width as f32) / 2.0).min(0.0),
            ((dim - height as f32) / 2.0).min(0.0),
        );
        for i in 0..self.particles.len() {
            let particle = &self.particles[i];
            let pos = (vec2(particle.position.x, -particle.position.y)
                + vec2(BOUND_RADIUS, BOUND_RADIUS))
                * dim
                / (BOUND_RADIUS * 2.0);
            if pos.x < offset.x
                || pos.x > offset.x + width
                || pos.y < offset.y
                || pos.y > offset.y + height
            {
                self.colors[particle.initial_id] = Color {
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
                self.colors[particle.initial_id] = color;
            }
        }
        save_vector_to_file(&self.colors, "colors.bin").unwrap();
        self.colors_changed = true;
    }

    fn run_sorted_collision(
        y: std::iter::StepBy<std::ops::Range<u32>>,
        spatial_hash: &SortingHash<FixedSizeGrid>,
        data_ptr: *mut Particle,
        dt: f32,
    ) {
        let UVec2 {
            x: width,
            y: height,
        } = spatial_hash.grid().size();
        for y in y {
            for x in 0..width {
                let (this_start, this_end) = spatial_hash.get_pointers(x, y);
                for i in this_start..this_end {
                    for j in (i + 1)..this_end {
                        let first = unsafe { &mut *(data_ptr.add(i)) };
                        let second = unsafe { &mut *(data_ptr.add(j)) };
                        apply_distance_constraint(first, second, dt);
                    }
                    if x < width - 1 {
                        let (other_start, other_end) = spatial_hash.get_pointers(x + 1, y);
                        for j in other_start..other_end {
                            let first = unsafe { &mut *(data_ptr.add(i)) };
                            let second = unsafe { &mut *(data_ptr.add(j)) };
                            apply_distance_constraint(first, second, dt);
                        }
                    }
                    if y < height - 1 {
                        let (other_start, other_end) = spatial_hash.get_pointers_range(
                            if x == 0 { 0 } else { x - 1 },
                            if x == width - 1 { x } else { x + 1 },
                            y + 1,
                        );
                        for j in other_start..other_end {
                            let first = unsafe { &mut *(data_ptr.add(i)) };
                            let second = unsafe { &mut *(data_ptr.add(j)) };
                            apply_distance_constraint(first, second, dt);
                        }
                    }
                }

                // let indicies = spatial_hash.get_indexes_by_cell(uvec2(x, y));
                // let other_indicies = []
                //     .into_iter()
                //     .chain(if x < width - 1 {
                //         Some((x + 1, y))
                //     } else {
                //         None
                //     })
                //     .chain(if y < height - 1 {
                //         Some((x, y + 1))
                //     } else {
                //         None
                //     })
                //     .chain(if x < width - 1 && y < height - 1 {
                //         Some((x + 1, y + 1))
                //     } else {
                //         None
                //     })
                //     .chain(if x > 0 && y < height - 1 {
                //         Some((x - 1, y + 1))
                //     } else {
                //         None
                //     })
                //     .flat_map(|(xx, yy)| spatial_hash.get_indexes_by_cell(uvec2(xx, yy)));
                //
                // Self::run_cell_collisions(indicies, other_indicies, data_ptr, dt);
            }
        }
    }

    fn run_collision(
        y: std::iter::StepBy<std::ops::Range<u32>>,
        spatial_hash: &PointerHash<FixedSizeGrid>,
        data_ptr: *mut Particle,
        dt: f32,
    ) {
        let UVec2 {
            x: width,
            y: height,
        } = spatial_hash.grid().size();
        for y in y {
            for x in 0..width {
                let indicies = spatial_hash.get_indexes_by_cell(uvec2(x, y));
                let other_indicies = []
                    .into_iter()
                    .chain(if x < width - 1 {
                        Some((x + 1, y))
                    } else {
                        None
                    })
                    .chain(if x > 0 && y < height - 1 {
                        Some((x - 1, y + 1))
                    } else {
                        None
                    })
                    .chain(if y < height - 1 {
                        Some((x, y + 1))
                    } else {
                        None
                    })
                    .chain(if x < width - 1 && y < height - 1 {
                        Some((x + 1, y + 1))
                    } else {
                        None
                    })
                    .flat_map(|(xx, yy)| spatial_hash.get_indexes_by_cell(uvec2(xx, yy)));

                Self::run_cell_collisions(indicies, other_indicies, data_ptr, dt);
            }
        }
    }

    fn run_cell_collisions<'a, I: Iterator<Item = &'a usize> + Clone>(
        indicies: &'a [usize],
        other_indicies: I,
        data_ptr: *mut Particle,
        dt: f32,
    ) {
        for (i, first_index) in indicies.iter().enumerate() {
            for second_index in indicies.iter().skip(i + 1).chain(other_indicies.clone()) {
                let first = unsafe { &mut *(data_ptr.add(*first_index)) };
                let second = unsafe { &mut *(data_ptr.add(*second_index)) };
                apply_distance_constraint(first, second, dt);
            }
        }
    }
}

fn apply_distance_constraint(first: &mut Particle, second: &mut Particle, dt: f32) {
    // let v = first.position - second.position;
    // let dist = v.length();
    // let min_dist = first.radius + second.radius;
    // if dist < min_dist {
    //     let n = v / dist;
    //     let delta = (min_dist - dist);
    //     first.position += n * 0.5 * delta;
    //     second.position -= n * 0.5 * delta;
    // }
    // let alpha = 0.000004337 * 2.0;
    // let alpha = 1e-4;
    let alpha = 0.0;
    let vector = second.position - first.position;
    let direction = vector.normalize_or(vec2(1.0, 0.0));
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
        self.gen_range(1.0..=1.0) * MAX_PARTICLE_RADIUS
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

// fn get_two_mut<T>(vec: &mut Vec<T>, first: usize, second: usize) -> (&mut T, &mut T) {
//     if first > second {
//         let (head, tail) = vec.split_at_mut(first);
//         (&mut head[second], &mut tail[0])
//     } else {
//         let (head, tail) = vec.split_at_mut(second);
//         (&mut head[first], &mut tail[0])
//     }
// }
//
impl Positioned for Particle {
    fn position(&self) -> Vec2 {
        self.position
    }
}
