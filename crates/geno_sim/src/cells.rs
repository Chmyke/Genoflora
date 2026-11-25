//! Cell storage, as a struct of arrays (the layout the GPU engine uses too).

/// Index of auxin in a cell's hormones.
pub const AUXIN: usize = 0;
/// Index of cytokinin in a cell's hormones.
pub const CYTOKININ: usize = 1;

/// One cell, as the rules see it.
///
/// A cell is an ellipsoid: half-axis `r k^2` along its long axis and `r / k` across it, so its
/// volume is always that of a sphere of radius `r`, whatever its stretch `k` (1 = a sphere).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cell {
    /// Position xyz + radius `r` of the sphere of the same volume.
    pub pr: [f32; 4],
    /// Long axis xyz (unit vector) + stretch `k`.
    pub shape: [f32; 4],
    /// Hormone amounts (auxin, cytokinin).
    pub h: [f32; 2],
}

/// A cell's index is its stable identifier (cells never die yet). Vectors are `capacity` long;
/// only the first `count` entries are alive.
#[derive(Clone, Debug, PartialEq)]
pub struct Cells {
    pub count: usize,
    pub pr: Vec<[f32; 4]>,
    pub shape: Vec<[f32; 4]>,
    pub h: Vec<[f32; 2]>,
}

impl Cells {
    /// Room for `capacity` cells, none alive.
    pub fn empty(capacity: usize) -> Self {
        Self {
            count: 0,
            pr: vec![[0.0; 4]; capacity],
            shape: vec![[1.0, 0.0, 0.0, 1.0]; capacity],
            h: vec![[0.0; 2]; capacity],
        }
    }

    /// A single round cell of radius `p.r_div` at the origin: the starting point of a plant.
    /// Its long axis is drawn from the run's seed, so it is aligned with nothing in particular.
    pub fn seed(p: &crate::Params, capacity: usize, hormone: f32) -> Self {
        let mut cells = Self::empty(capacity.max(1));
        cells.count = 1;
        let a = crate::hash::axis(p.seed, 0, u32::MAX);
        cells.set(
            0,
            Cell {
                pr: [0.0, 0.0, 0.0, p.r_div],
                shape: [a[0], a[1], a[2], 1.0],
                h: [hormone, 0.0],
            },
        );
        cells
    }

    pub fn capacity(&self) -> usize {
        self.pr.len()
    }

    pub fn get(&self, i: usize) -> Cell {
        Cell {
            pr: self.pr[i],
            shape: self.shape[i],
            h: self.h[i],
        }
    }

    pub fn set(&mut self, i: usize, cell: Cell) {
        self.pr[i] = cell.pr;
        self.shape[i] = cell.shape;
        self.h[i] = cell.h;
    }

    /// The living cells: positions with radius.
    pub fn alive(&self) -> &[[f32; 4]] {
        &self.pr[..self.count]
    }

    /// Long axes and stretches of the living cells.
    pub fn shapes(&self) -> &[[f32; 4]] {
        &self.shape[..self.count]
    }

    /// Hormone amounts of the living cells.
    pub fn hormone(&self) -> &[[f32; 2]] {
        &self.h[..self.count]
    }
}
