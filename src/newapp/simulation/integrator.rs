// use glam::Vec2;
//
// pub struct Integrator {
//     previous_positions: Vec<Vec2>,
// }
//
// impl Integrator {
//     pub fn new() -> Self {
//         Self {
//             previous_positions: vec![],
//         }
//     }
//
//     pub fn integrate(&mut self, &mut particle: Particle, acceleration: Vec2) {
//
//         for (i, particle) in particles.iter_mut().enumerate() {
//             if (self.previous_positions.len() as usize) <= i {
//                 self.previous_positions.push(particle.position);
//             } else {
//                 self.previous_positions[i] = particle.position;
//             }
//             // particle.position += particle.velocity * dt + gravity * dt * dt;
//             particle.velocity += gravity * dt;
//             particle.position += particle.velocity * dt;
//         }
//     }
// }
