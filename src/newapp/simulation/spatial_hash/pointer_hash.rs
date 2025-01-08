use std::{
    ptr::{slice_from_raw_parts, slice_from_raw_parts_mut, NonNull},
    slice,
};

use glam::{uvec2, UVec2, Vec2};

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
        let cell_index = self.grid.get_cell_index(cell);
        let start = self.pointers[cell_index];
        let end = self.pointers[cell_index + 1];
        &self.indexes[start..end]
    }

    pub fn reference<'a, I>(&'a self, items: &'a mut [I]) -> HashReference<'a, I, Grid> {
        HashReference::<'a, I, Grid>::new(items, &self.grid, &self.pointers, &self.indexes)
    }
}

pub struct HashReference<'a, Item, Grid: SpatialGrid> {
    data: NonNull<Item>,
    indexes: &'a [usize],
    pointers: &'a [usize],
    grid: &'a Grid,
    start_cell_index: usize,
    end_cell_index: usize,
}

pub struct RowReference<'a, Item, Grid: SpatialGrid> {
    data: NonNull<Item>,
    indexes: &'a [usize],
    pointers: &'a [usize],
    grid: &'a Grid,
    start_cell_index: usize,
}
//
// impl<'a, Item, Grid: SpatialGrid> HashReference<'a, Item, Grid> {}

impl<'a, Item, Grid: SpatialGrid> HashReference<'a, Item, Grid> {
    pub(self) fn new(
        data: &'a [Item],
        grid: &'a Grid,
        pointers: &'a [usize],
        indexes: &'a [usize],
    ) -> Self {
        Self {
            data: NonNull::from(data).cast(),
            pointers,
            indexes,
            grid,
            start_cell_index: 0,
            end_cell_index: grid.number_of_cells(),
        }
    }

    pub fn get(
        &'a mut self,
        coord: UVec2,
    ) -> std::iter::Map<slice::Iter<'a, usize>, impl FnMut(&'a usize) -> &'a mut Item> {
        let cell_index = self.grid.get_cell_index(coord);
        let start = self.pointers[cell_index];
        let end = self.pointers[cell_index + 1];
        self.indexes[start..end]
            .iter()
            .map(|i| unsafe { &mut *self.data.as_ptr().add(*i) })
    }

    pub fn split_at_row(self, row: u32) -> (Self, Self) {
        self.split(uvec2(0, row))
    }

    pub fn split(self, coord: UVec2) -> (Self, Self) {
        let Self {
            data,
            pointers,
            indexes,
            grid,
            start_cell_index,
            end_cell_index,
        } = self;

        let second_start_cell_index = grid.get_cell_index(coord);

        assert!(second_start_cell_index in start_cell_index..end_cell_index);

        let first = Self {
            // data: unsafe {
            //     // obtain another mutable reference to the data
            //     &mut *slice_from_raw_parts_mut((data.as_ptr() as usize) as *mut Item, data.len())
            // },
            data,
            pointers: &pointers[0..second_start_cell_index + 1],
            indexes,
            grid,
            start_cell_index,
            end_cell_index: second_start_cell_index,
        };

        let second = Self {
            data,
            pointers: &pointers[second_start_cell_index..],
            indexes,
            grid,
            start_cell_index: second_start_cell_index,
            end_cell_index,
        };
        (first, second)
    }
}
