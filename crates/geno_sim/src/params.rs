//! Parameters of the cell rules. Lengths are in units of `r_div`; rates are per step.

/// A cell divides by length once its half-length passes the longest by this share. For a round
/// cell the longest equals the dividing radius: without the margin, the last rounding digit of
/// the half-length would decide, differently on the judge and on the GPU.
pub const LENGTH_MARGIN: f32 = 1.001;

/// A cell divides only while its youth is above this.
pub const YOUNG: f32 = 0.5;

/// Radius factor of a daughter cell: volume is split evenly, so `r / cbrt(2)`.
pub const INV_CBRT2: f32 = 0.793_700_5;

/// Placeholder until the genome provides them: one fixed set of rates shared by every cell.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Params {
    /// Seed of every random draw (division axes, test tissues).
    pub seed: u32,
    /// Radius at which a cell divides.
    pub r_div: f32,
    /// Radius gained per step by a cell saturated with hormone.
    pub growth: f32,
    /// Hormone concentration at which a cell grows at half that speed.
    pub half_sat: f32,
    /// Mobility under compression (overlapping cells push each other apart).
    pub mob_rep: f32,
    /// Mobility under tension (close cells pull each other, tent profile, zero at the reach).
    pub mob_adh: f32,
    /// Adhesion and diffusion range, as a factor of the sum of the two radii.
    pub reach: f32,
    /// Share of the auxin gap with each neighbour closed per step (the gap is in concentration).
    /// Stable while it stays well below one over the number of neighbours.
    pub diffusion: f32,
    /// Share of its auxin a fully polarised cell pumps, per step, to a neighbour lying straight
    /// along its polarity (active transport by the PIN pumps).
    pub polar: f32,
    /// Share of the way the pump polarity moves toward its target per step (how fast the pumps
    /// are moved from one side of the cell to another).
    pub pin_rate: f32,
    /// Auxin concentration at which a cell weighs both ways of placing its pumps equally: below,
    /// mostly toward the richer neighbours; above, mostly along the auxin flowing through it.
    pub pin_switch: f32,
    /// Auxin flow through a cell at which its pumps follow the flow half as much as they can.
    pub flux_half: f32,
    /// How far the heavy grains settled on the lower side of a cell pull its pumps down (0: the
    /// cell does not sense gravity).
    pub gravisense: f32,
    /// Auxin every cell makes per step, per unit of its volume, in the dark.
    pub production: f32,
    /// Auxin a cell makes on top of that, per step, per unit of its volume, in full light (light
    /// feeds the sugars from which auxin is made).
    pub light_production: f32,
    /// Fraction of hormone lost per step.
    pub decay: f32,
    /// Cytokinin a cell makes per step, per unit of auxin it holds.
    pub ck_production: f32,
    /// Cytokinin a cell makes per step, per unit of its volume, in full light (light wakes the
    /// shoot tip through cytokinin, Pfeiffer et al. 2016).
    pub ck_light: f32,
    /// Same as `diffusion`, for cytokinin.
    /// It has no preferred direction.
    pub ck_diffusion: f32,
    /// Fraction of cytokinin lost per step.
    pub ck_decay: f32,
    /// Cytokinin concentration at which its push on growth is half of its maximum.
    pub ck_half: f32,
    /// Auxin concentration that halves the push of cytokinin (the two hormones oppose each
    /// other: what counts is their ratio).
    pub ck_block: f32,
    /// Relative growth per step along the growth direction, for a cell holding just the right
    /// amount of auxin (this growth adds volume, like water taken up).
    pub elong: f32,
    /// Auxin concentration at which that growth reaches half its pace...
    pub e_half: f32,
    /// ...and the one above which it is held back by half.
    pub e_block: f32,
    /// How much the pressure of its neighbours holds back the growth of a cell: growth stops once
    /// the sum of their overlaps reaches `1 / squeeze_hold` of its radius (0: it grows whatever
    /// presses on it).
    pub squeeze_hold: f32,
    /// Share of the way a cell's youth moves per step toward what its auxin calls for (how fast
    /// cells mature, or turn young again).
    pub youth_rate: f32,
    /// Auxin concentration that keeps a cell half young: more keeps it young, less lets it mature.
    /// Cytokinin keeps a cell young too where auxin is scarce (`ck_half`, `ck_block`): the two
    /// hormones swap roles between the root tip and the shoot tip.
    pub youth_half: f32,
    /// Longest half-length of a cell, in units of `r_div`: a cell that reaches it divides.
    pub max_half_length: f32,
    /// How fast the growth direction of a cell turns toward the line of the auxin flow through it,
    /// per unit of flow (a weak flow turns it little).
    pub align: f32,
    /// Divisions are evaluated every `divide_every` steps (1: each cell divides as soon as it is
    /// ready, which is what real cells do).
    pub divide_every: u32,
    /// Direction in which heavy things fall (unit vector). Part of the environment, not of the cell.
    pub gravity: [f32; 3],
    /// Height of the soil surface above the origin, where the seed starts (environment).
    pub surface: f32,
    /// Push the soil withstands without giving way, as a move per step: in the soil a cell only
    /// moves by what its push exceeds this (solid friction, environment).
    pub soil_grip: f32,
    /// How hard the soil is: a cell whose walls press on the soil rather than on other cells grows
    /// `1 + soil_hardness` times slower (environment).
    pub soil_hardness: f32,
    /// Depth over which the soil dims the light by a factor e (environment).
    pub light_depth: f32,
    /// How much a cell lying straight above another dims its light: the light is divided by e for
    /// each `1 / shade` of such cover (environment; 0: cells cast no shadow).
    pub shade: f32,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            seed: 20_261_124,
            r_div: 1.0,
            growth: 0.001,
            half_sat: 10.0,
            mob_rep: 0.1,
            mob_adh: 0.02,
            reach: 1.2,
            diffusion: 0.02,
            polar: 0.1,
            pin_rate: 0.01,
            pin_switch: 1.0,
            flux_half: 0.01,
            gravisense: 0.3,
            production: 0.001,
            light_production: 0.03,
            decay: 0.001,
            // Off in the default genome: measured on 2026-11-25, cytokinin that only diffuses
            // makes no second pole (see docs/contexte/Feuille de route.md).
            ck_production: 0.0,
            ck_light: 0.03,
            ck_diffusion: 0.05,
            ck_decay: 0.001,
            ck_half: 1.0,
            ck_block: 1.0,
            // Off in the default genome: cells stay round.
            elong: 0.0,
            e_half: 1.0,
            e_block: 1.0e6,
            squeeze_hold: 1.0,
            youth_rate: 0.01,
            youth_half: 3.0,
            max_half_length: 1.0,
            align: 0.0,
            divide_every: 1,
            gravity: [0.0, -1.0, 0.0],
            // The seed lies just under the surface: one dividing radius of soil above it.
            surface: 2.0,
            soil_grip: 0.02,
            soil_hardness: 0.3,
            light_depth: 1.0,
            shade: 3.0,
        }
    }
}
