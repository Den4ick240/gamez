use glam::{uvec2, vec2, UVec2, Vec2};

use crate::newapp::simulation::box_constraint::BoxConstraint;

use super::SpatialGrid;

pub struct FixedSizeGrid {
    origin: Vec2,
    size: UVec2,
    cell_size: Vec2,
}

impl FixedSizeGrid {
    pub fn new(min_cell_size: f32, bounds: BoxConstraint) -> Self {
        let origin = vec2(bounds.left, bounds.bottom);
        let bounds_size = vec2(bounds.right - bounds.left, bounds.top - bounds.bottom);
        let size: UVec2 = (bounds_size / min_cell_size).as_uvec2();
        let cell_size = bounds_size / size.as_vec2();
        Self {
            origin,
            size,
            cell_size,
        }
    }
}

impl SpatialGrid for FixedSizeGrid {
    fn size(&self) -> UVec2 {
        self.size
    }

    fn get_cell_coords(&self, position: Vec2) -> UVec2 {
        ((position - self.origin) / self.cell_size)
            .as_uvec2()
            .max(UVec2::ZERO)
            .min(self.size - 1)
    }

    fn get_cell_index(&self, coord: UVec2) -> usize {
        coord.x as usize + coord.y as usize * self.size.x as usize
    }

    fn number_of_cells(&self) -> usize {
        self.size.x as usize * self.size.y as usize
    }
}
