//! Uniform grid used by the shaders to find neighbours.

use geno_sim::Params;

/// Covers the ball that `capacity` cells can form. Out-of-range cells are clamped into the
/// border cells, so the neighbour search stays correct anywhere.
#[derive(Clone, Copy, Debug)]
pub struct Grid {
    /// Grid cells per axis.
    pub g1: u32,
    pub inv_cell: f32,
    /// Half the grid width: positions are shifted by it to become positive.
    pub half: f32,
}

impl Grid {
    pub fn for_capacity(capacity: usize, p: &Params) -> Self {
        // A grid cell must be wider than the longest interaction: reach times two half-lengths,
        // and a cell divides once its half-length reaches r_div * max_half_length (it may overshoot
        // by one step of growth, hence the margin).
        let longest = p.r_div * p.max_half_length.max(1.0);
        let cell = (2.5 * p.r_div).max(p.reach * 2.0 * longest * 1.05);
        let radius = 1.6 * (capacity as f32).cbrt() * longest;
        let g1 = (2.0 * radius / cell).ceil() as u32 + 2;
        Self {
            g1,
            inv_cell: 1.0 / cell,
            half: g1 as f32 * cell * 0.5,
        }
    }

    pub fn cells(&self) -> usize {
        (self.g1 as usize).pow(3)
    }
}
