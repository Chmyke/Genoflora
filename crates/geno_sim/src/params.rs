//! Parameters of the cell rules. Lengths are in units of `r_div`; rates are per step.

/// Radius factor of a daughter cell: volume is split evenly, so `r / cbrt(2)`.
pub const INV_CBRT2: f32 = 0.793_700_5;

/// Placeholder until the genome provides them: one fixed set of rates shared by every cell.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Params {
    /// Seed of every random draw (division axes, test tissues).
    pub seed: u32,
    /// Radius at which a cell divides.
    pub r_div: f32,
    /// Radius gained per step.
    pub growth: f32,
    /// Mobility under compression (overlapping cells push each other apart).
    pub mob_rep: f32,
    /// Mobility under tension (close cells pull each other, tent profile, zero at the reach).
    pub mob_adh: f32,
    /// Adhesion and diffusion range, as a factor of the sum of the two radii.
    pub reach: f32,
    /// Hormone exchanged per step with each neighbour, per unit of difference.
    pub diffusion: f32,
    /// Hormone produced per step where x > 0 (placeholder source until genes decide).
    pub production: f32,
    /// Fraction of hormone lost per step.
    pub decay: f32,
    /// Divisions are evaluated every `divide_every` steps.
    pub divide_every: u32,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            seed: 20_261_124,
            r_div: 1.0,
            growth: 0.001,
            mob_rep: 0.1,
            mob_adh: 0.02,
            reach: 1.2,
            diffusion: 0.02,
            production: 0.001,
            decay: 0.001,
            divide_every: 20,
        }
    }
}
