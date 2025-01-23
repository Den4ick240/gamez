use std::ops::Range;

use glam::{vec3, Vec3};
use rand::{distributions::uniform::SampleRange, rngs::StdRng, Rng, SeedableRng};

pub struct MyRng {
    rng: StdRng,
}

impl MyRng {
    pub fn new() -> Self {
        const SEED: [u8; 32] = [
            1u8, 2u8, 3u8, 4u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0,
        ];
        Self {
            rng: SeedableRng::from_seed(SEED),
        }
    }

    pub fn get_random_size<R>(&mut self, range: R) -> f32
    where
        R: SampleRange<f32>,
    {
        self.rng.gen_range(range)
    }

    pub fn get_random_color(&mut self) -> Vec3 {
        const COLORS: [Vec3; 12] = [
            vec3(0.945, 0.769, 0.058), // Vibrant Yellow
            vec3(0.204, 0.596, 0.859), // Sky Blue
            vec3(0.608, 0.349, 0.714), // Soft Purple
            vec3(0.231, 0.764, 0.392), // Fresh Green
            vec3(0.937, 0.325, 0.314), // Coral Red
            vec3(0.180, 0.800, 0.443), // Mint Green
            vec3(0.996, 0.780, 0.345), // Soft Orange
            vec3(0.556, 0.267, 0.678), // Deep Violet
            vec3(0.870, 0.490, 0.847), // Lavender Pink
            vec3(0.529, 0.808, 0.922), // Light Blue
            vec3(0.996, 0.921, 0.545), // Pa.s.tel Yellow
            vec3(0.835, 0.625, 0.459), // Warm Beige
        ];

        COLORS[self.rng.gen_range(0..COLORS.len())]
    }
}
