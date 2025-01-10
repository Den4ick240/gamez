use std::{
    ptr::{slice_from_raw_parts, slice_from_raw_parts_mut, NonNull},
    slice,
};

use glam::{uvec2, UVec2, Vec2};
use wgpu::naga::Range;

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
    start_cell: UVec2,
    end_cell: UVec2,
}

unsafe impl<'a, Item, Grid: SpatialGrid> Send for HashReference<'a, Item, Grid> {}

pub struct RowReference<'a, Item, Grid: SpatialGrid> {
    data: NonNull<Item>,
    indexes: &'a [usize],
    pointers: &'a [usize],
    grid: &'a Grid,
    start_cell_index: usize,
}

pub struct RowIter<'a, Item, Grid: SpatialGrid, const START: u32, const END: u32> {
    reference: &'a HashReference<'a, Item, Grid>,
    rr: Vec<&'a mut Item>,
    start_cell_index: u32,
    curr: u32,
    width: u32,
}

// impl<'a, Item, Grid: SpatialGrid, const START: u32, const END: u32>
//     RowIter<'a, Item, Grid, START, END>
// {
//     fn next(&mut self) -> Option<(&[&'a mut Item], &[&'a mut Item])> {
//         if self.curr < self.width {
//             let start = self.curr.max(START) - START;
//             let end = self.curr + END;
//             // (start..curr).chain((curr + 1)..=end).flat_map()
//         } else {
//             None
//         }
//     }
// }

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
            start_cell: uvec2(0, 0),
            end_cell: grid.size(),
        }
    }

    pub fn get<'b>(
        &'b mut self,
        coord: UVec2,
    ) -> std::iter::Map<slice::Iter<'b, usize>, impl FnMut(&'b usize) -> &'b mut Item> {
        let cell_index = self.grid.get_cell_index(coord);
        let start = self.pointers[cell_index];
        let end = self.pointers[cell_index + 1];
        self.indexes[start..end]
            .iter()
            .map(|i| unsafe { &mut *self.data.as_ptr().add(*i) })
    }

    pub fn get_vec<'b>(&'b mut self, coord: UVec2) -> Vec<&'b mut Item> {
        // let cell_index = self.grid.get_cell_index(coord);
        // let start = self.pointers[cell_index];
        // let end = self.pointers[cell_index + 1];
        // self.indexes[start..end]
        //     .iter()
        //     .map(|i| unsafe { &mut *self.data.as_ptr().add(*i) })
        self.get(coord).collect()
    }

    // pub(self) fn get_range<'b>(
    //     &'b mut self,
    //     range: Range<u32>,
    // ) -> std::iter::Map<slice::Iter<'b, usize>, impl FnMut(&'b usize) -> &'b mut Item> {
    //     self.indexes[start..end]
    //         .iter()
    //         .map(|i| unsafe { &mut *self.data.as_ptr().add(*i) })
    // }

    pub fn split_at_row(self, row: u32) -> (Self, Self) {
        self.split(uvec2(0, row))
    }

    // pub fn iter<const START: u32, const END: u32>(
    //     &mut self,
    // ) -> RowIter<'_, Item, Grid, START, END> {
    //     RowIter {
    //         reference: self,
    //         row: self.start_cell.y,
    //         curr: 0,
    //     }
    // }

    // pub fn iterate_row(self, range: Range<u32>) ->

    fn split(self, coord: UVec2) -> (Self, Self) {
        let Self {
            data,
            pointers,
            indexes,
            grid,
            start_cell,
            end_cell,
        } = self;

        // let second_start_cell_index = grid.get_cell_index(coord);
        // assert!(second_start_cell_index in start_cell_index..end_cell_index);

        let first = Self {
            data,
            pointers,
            indexes,
            grid,
            start_cell,
            end_cell: coord,
        };

        let second = Self {
            data,
            pointers,
            indexes,
            grid,
            start_cell: coord,
            end_cell,
        };
        (first, second)
    }
}
