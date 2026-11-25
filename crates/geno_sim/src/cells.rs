//! Cell storage, as a struct of arrays (the layout the GPU engine uses too).

/// Index of auxin in a cell's hormones.
pub const AUXIN: usize = 0;
/// Index of cytokinin in a cell's hormones.
pub const CYTOKININ: usize = 1;
/// Index of the restraint signal in a cell's hormones.
pub const RESTRAINT: usize = 2;

use crate::shape::{Shape, sphere};

/// One cell, as the rules see it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cell {
    /// Position xyz + radius `r` of the sphere of the same volume.
    pub pr: [f32; 4],
    /// Ellipsoid and growth direction, see `shape`.
    pub shape: Shape,
    /// Hormone amounts (auxin, cytokinin, restraint signal).
    pub h: [f32; 3],
    /// Polarity of the auxin pumps (PIN): xyz is the side of the cell where they sit, its length
    /// (at most 1) how strongly they gather there. The fourth slot is not about the pumps: it
    /// keeps the light the cell received at its last step, for display only (no rule reads it).
    pub pin: [f32; 4],
    /// How young the cell is, from 1 (it can divide) to 0 (mature: it only grows).
    pub youth: f32,
}

/// A cell's index is its stable identifier (cells never die yet). Vectors are `capacity` long;
/// only the first `count` entries are alive.
#[derive(Clone, Debug, PartialEq)]
pub struct Cells {
    pub count: usize,
    pub pr: Vec<[f32; 4]>,
    pub shape: Vec<Shape>,
    pub h: Vec<[f32; 3]>,
    pub pin: Vec<[f32; 4]>,
    pub youth: Vec<f32>,
}

impl Cells {
    /// Room for `capacity` cells, none alive.
    pub fn empty(capacity: usize) -> Self {
        Self {
            count: 0,
            pr: vec![[0.0; 4]; capacity],
            shape: vec![sphere(1.0, [1.0, 0.0, 0.0]); capacity],
            h: vec![[0.0; 3]; capacity],
            pin: vec![[0.0; 4]; capacity],
            youth: vec![1.0; capacity],
        }
    }

    /// A single round cell of radius `p.r_div` at the origin: the starting point of a plant.
    /// Its growth direction is drawn from the run's seed, so it is aligned with nothing in
    /// particular, its auxin pumps are spread evenly (no polarity), and it is young.
    pub fn seed(p: &crate::Params, capacity: usize, hormone: f32) -> Self {
        let mut cells = Self::empty(capacity.max(1));
        cells.count = 1;
        let a = crate::hash::axis(p.seed, 0, u32::MAX);
        cells.set(
            0,
            Cell {
                pr: [0.0, 0.0, 0.0, p.r_div],
                shape: sphere(p.r_div, a),
                h: [hormone, 0.0, 0.0],
                pin: [0.0; 4],
                youth: 1.0,
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
            pin: self.pin[i],
            youth: self.youth[i],
        }
    }

    pub fn set(&mut self, i: usize, cell: Cell) {
        self.pr[i] = cell.pr;
        self.shape[i] = cell.shape;
        self.h[i] = cell.h;
        self.pin[i] = cell.pin;
        self.youth[i] = cell.youth;
    }

    /// The living cells: positions with radius.
    pub fn alive(&self) -> &[[f32; 4]] {
        &self.pr[..self.count]
    }

    /// Shapes of the living cells.
    pub fn shapes(&self) -> &[Shape] {
        &self.shape[..self.count]
    }

    /// Hormone amounts of the living cells.
    pub fn hormone(&self) -> &[[f32; 3]] {
        &self.h[..self.count]
    }

    /// Youth of the living cells.
    pub fn youths(&self) -> &[f32] {
        &self.youth[..self.count]
    }

    /// Pump polarities of the living cells.
    pub fn polarity(&self) -> &[[f32; 4]] {
        &self.pin[..self.count]
    }
}
