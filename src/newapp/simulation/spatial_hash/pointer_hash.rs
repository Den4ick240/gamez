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

    pub fn reference<'a, I>(&'a self, items: &'a mut [I]) -> HashReference<'a, I, Grid> {
        let ptr = items.as_mut_ptr() as usize;
        let cell_iterator = self.pointers.windows(2).map(|cell| {
            self.indexes[cell[0]..cell[1]].iter().map(|&index| {
                unsafe { &mut *(ptr as *mut I).add(index) };
            })
        });
        HashReference::new(items, &self.grid, &self.pointers, &self.indexes)
    }
}

pub struct HashReference<'a, I, Grid: SpatialGrid> {
    data: &'a mut [I],
    indexes: &'a [usize],
    pointers: &'a [usize],
    grid: &'a Grid,
    start_cell_index: usize,
}

impl<'a, I, Grid: SpatialGrid> HashReference<'a, I, Grid> {
    pub(self) fn new(
        data: &'a mut [I],
        grid: &'a Grid,
        pointers: &'a [usize],
        indexes: &'a [usize],
    ) -> Self {
        Self {
            data,
            pointers,
            indexes,
            grid,
            start_cell_index: 0,
        }
    }

    pub fn split_at_mut(self, coord: UVec2) -> (Self, Self) {
        let Self {
            data,
            pointers,
            indexes,
            grid,
            start_cell_index,
        } = self;

        let second_start_cell_index = grid.get_cell_index(coord);

        // let (first_pointers, second_pointers) = pointers.split_at(mid);
        // let (first_indexes, second_indexes) = indexes.split_at(second_star );

        let first = Self {
            data,
            grid,
            start_cell_coord,
        };

        let second = Self {
            data,
            grid,
            start_cell_coord: coord,
        };
        (first, second)
    }
}
