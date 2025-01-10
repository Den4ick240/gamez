use glam::{UVec2, Vec2};
pub mod fixed_size_grid;
pub mod pointer_hash;
pub mod sorting_hash;

pub trait SpatialGrid {
    fn size(&self) -> UVec2;
    fn number_of_cells(&self) -> usize;
    fn get_cell_coords(&self, position: Vec2) -> UVec2;
    fn get_cell_index(&self, coord: UVec2) -> usize;
    fn get_position_cell_index(&self, position: Vec2) -> usize {
        self.get_cell_index(self.get_cell_coords(position))
    }
}
