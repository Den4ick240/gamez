use super::Particle;

#[derive(Debug, Clone, Copy)]
pub struct BoxConstraint {
    pub top: f32,
    pub bottom: f32,
    pub right: f32,
    pub left: f32,
}

impl BoxConstraint {
    pub fn apply(&self, particle: &mut Particle, _dt: f32) {
        let r = particle.radius;
        let top = self.top - r;
        let bottom = self.bottom + r;
        let left = self.left + r;
        let right = self.right - r;
        let p = &mut particle.position;

        if p.y < bottom {
            p.y = bottom
        }
        if p.y > top {
            p.y = top
        }
        if p.x < left {
            p.x = left
        }
        if p.x > right {
            p.x = right
        }
    }

    pub fn around_center(radius: f32) -> Self {
        Self {
            top: radius,
            right: radius,
            bottom: -radius,
            left: -radius,
        }
    }
}
