//! Cell storage, as a struct of arrays (the layout the GPU engine uses too).

/// A cell's index is its stable identifier (cells never die yet).
/// `pr[i]` = position xyz + radius, `h[i]` = hormone amount. Vectors are `capacity` long;
/// only the first `count` entries are alive.
#[derive(Clone, Debug, PartialEq)]
pub struct Cells {
    pub count: usize,
    pub pr: Vec<[f32; 4]>,
    pub h: Vec<f32>,
}

impl Cells {
    /// Room for `capacity` cells, none alive.
    pub fn empty(capacity: usize) -> Self {
        Self {
            count: 0,
            pr: vec![[0.0; 4]; capacity],
            h: vec![0.0; capacity],
        }
    }

    /// A single cell at the origin: the starting point of a plant.
    pub fn seed(capacity: usize, radius: f32, hormone: f32) -> Self {
        let mut cells = Self::empty(capacity.max(1));
        cells.count = 1;
        cells.pr[0] = [0.0, 0.0, 0.0, radius];
        cells.h[0] = hormone;
        cells
    }

    pub fn capacity(&self) -> usize {
        self.pr.len()
    }

    /// The living cells: positions with radius.
    pub fn alive(&self) -> &[[f32; 4]] {
        &self.pr[..self.count]
    }

    /// Hormone amounts of the living cells.
    pub fn hormone(&self) -> &[f32] {
        &self.h[..self.count]
    }
}
