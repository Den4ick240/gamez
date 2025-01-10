use glam::{uvec2, Vec2};

use super::SpatialGrid;

pub struct SortingHash<Grid: SpatialGrid> {
    grid: Grid,
    pointers: Vec<u32>,
    indexes: Vec<u32>,
}

impl<Grid: SpatialGrid> SortingHash<Grid> {
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

    pub fn build<I: Positioned>(&mut self, items: &mut [I]) {
        let n_items = items.len();
        let grid = &self.grid;
        self.pointers.fill(0);
        self.indexes.clear();

        let item_indexes = items
            .iter()
            .map(|it| grid.get_position_cell_index(it.position()) as u32);
        self.indexes.extend(item_indexes);

        for &index in &self.indexes {
            self.pointers[index as usize] += 1;
        }

        let mut sum = 0;
        for pointer in &mut self.pointers {
            // todo try to add & to pointer
            sum += *pointer;
            *pointer = sum;
        }

        let mut i = 0usize;
        while i < n_items {
            let cell_index: usize = self.indexes[i] as usize;
            let pointer = &mut self.pointers[cell_index];
            if *pointer <= i as u32 {
                i += 1;
            } else {
                *pointer -= 1;
                items.swap(*pointer as usize, i);
                self.indexes.swap(*pointer as usize, i);
            }
        }
    }

    pub fn get_pointers(&self, x: u32, y: u32) -> (usize, usize) {
        let cell_index = self.grid.get_cell_index(uvec2(x, y));
        let start = self.pointers[cell_index];
        let end = self.pointers[cell_index + 1];
        (start as usize, end as usize)
    }

    pub fn get_pointers_range(&self, x_start: u32, x_end: u32, y: u32) -> (usize, usize) {
        let cell_index = self.grid.get_cell_index(uvec2(x_start, y));
        let start = self.pointers[cell_index];
        let end = self.pointers[cell_index + 1 + (x_end - x_start) as usize];
        (start as usize, end as usize)
    }
}

pub trait Positioned {
    fn position(&self) -> Vec2;
}
