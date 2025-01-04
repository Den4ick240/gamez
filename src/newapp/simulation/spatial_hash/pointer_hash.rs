use glam::{UVec2, Vec2};

use super::SpatialGrid;

pub struct PointerHash<Grid: SpatialGrid> {
    grid: Grid,
    indexes: Vec<usize>,
    pointers: Vec<usize>,
}

impl<Grid: SpatialGrid> PointerHash<Grid> {
    pub fn new(grid: Grid) -> Self {
        let n_cells = grid.number_of_cells();
        let pointers = vec![0; n_cells + 1];
        let indexes = vec![];
        Self {
            grid,
            pointers,
            indexes,
        }
    }

    pub fn grid(&self) -> &Grid {
        &self.grid
    }

    pub fn build<'a, I>(&mut self, positions: I)
    where
        I: ExactSizeIterator<Item = &'a Vec2> + Clone,
    {
        let n_positions = positions.len();
        let grid = &self.grid;
        self.indexes.resize(n_positions, 0);
        self.pointers.fill(0);
        for &position in positions.clone() {
            let cell_index = grid.get_position_cell_index(position);
            self.pointers[cell_index] += 1;
        }
        let mut sum = 0;
        for pointer in &mut self.pointers {
            sum += *pointer;
            *pointer = sum;
        }
        for (index, &position) in positions.enumerate() {
            let cell_index = grid.get_position_cell_index(position);
            let pointer = &mut self.pointers[cell_index];
            *pointer -= 1;
            self.indexes[*pointer] = index;
        }
    }

    pub fn get_indexes_by_cell(&self, cell: UVec2) -> &[usize] {
        let grid = &self.grid;
        let cell_index = grid.get_cell_index(cell);
        let start = self.pointers[cell_index];
        let end = self.pointers[cell_index + 1];
        &self.indexes[start..end]
    }
}
