//! The local rules of a cell. Every rule is written here first, then mirrored in the GPU shaders.

use crate::cells::Cells;
use crate::hash::axis;
use crate::params::{INV_CBRT2, Params};

/// What a cell gathers from its neighbours during one step.
#[derive(Clone, Copy, Debug, Default)]
pub struct Gathered {
    /// Displacement from pushes and pulls.
    pub disp: [f32; 3],
    /// Sum of hormone differences with the neighbours (passive diffusion).
    pub spread: f32,
    /// Hormone received from the neighbours above minus hormone handed to those below.
    pub fall: f32,
}

/// Contribution of neighbour `j` to cell `i` (gather form: each cell only writes itself).
#[inline]
pub fn interact(pi: &[f32; 4], hi: f32, pj: &[f32; 4], hj: f32, p: &Params, g: &mut Gathered) {
    let dv = [pi[0] - pj[0], pi[1] - pj[1], pi[2] - pj[2]];
    let d2 = dv[0] * dv[0] + dv[1] * dv[1] + dv[2] * dv[2];
    let rs = pi[3] + pj[3];
    let reach = p.reach * rs;
    if d2 < reach * reach && d2 > 1e-12 {
        let d = d2.sqrt();
        // Overlap: linear repulsion. Gap: adhesion shaped as a tent that returns to zero at the
        // reach, so the force has no jump (a jump would amplify rounding differences).
        let s = rs - d;
        let pull = (-s).min(reach - d);
        let coef = (if s > 0.0 { p.mob_rep * s } else { -p.mob_adh * pull }) / d;
        g.disp[0] += coef * dv[0];
        g.disp[1] += coef * dv[1];
        g.disp[2] += coef * dv[2];
        g.spread += hj - hi;
        // How much `i` sits above `j`, from -1 to 1. Both cells compute exact opposites, so what
        // one hands over is exactly what the other receives.
        let above = (dv[0] * p.gravity[0] + dv[1] * p.gravity[1] + dv[2] * p.gravity[2]) / -d;
        g.fall += hj * (-above).max(0.0) - hi * above.max(0.0);
    }
}

/// New state of cell `i` once every neighbour has been gathered.
#[inline]
pub fn finish(pi: &[f32; 4], hi: f32, g: &Gathered, p: &Params) -> ([f32; 4], f32) {
    let volume = pi[3] * pi[3] * pi[3];
    let h = hi + p.diffusion * g.spread + p.polar * g.fall + p.production * volume - p.decay * hi;
    // The more hormone a cell holds for its size, the faster it grows, up to a ceiling.
    let conc = hi / volume;
    let r = (pi[3] + p.growth * (conc / (p.half_sat + conc))).min(p.r_div);
    ([pi[0] + g.disp[0], pi[1] + g.disp[1], pi[2] + g.disp[2], r], h)
}

/// One division round, in index order. Volume and hormone are split evenly between the two
/// daughters. The daughter of the n-th dividing cell gets index `count + n`; nothing divides
/// beyond the capacity.
pub fn divide(cells: &mut Cells, p: &Params, epoch: u32) {
    let count = cells.count;
    let capacity = cells.capacity();
    let mut rank = 0;
    for i in 0..count {
        if cells.pr[i][3] >= p.r_div {
            let j = count + rank;
            rank += 1;
            if j >= capacity {
                continue;
            }
            let a = axis(p.seed, i as u32, epoch);
            let m = cells.pr[i];
            let r = m[3] * INV_CBRT2;
            let o = a.map(|c| c * (0.5 * r));
            let h = cells.h[i] * 0.5;
            cells.pr[i] = [m[0] - o[0], m[1] - o[1], m[2] - o[2], r];
            cells.pr[j] = [m[0] + o[0], m[1] + o[1], m[2] + o[2], r];
            cells.h[i] = h;
            cells.h[j] = h;
        }
    }
    cells.count = (count + rank).min(capacity);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn force_is_continuous_at_contact_and_at_the_reach() {
        let p = Params::default();
        let pull = |d: f32| {
            let mut g = Gathered::default();
            interact(&[d, 0.0, 0.0, 1.0], 0.0, &[0.0, 0.0, 0.0, 1.0], 0.0, &p, &mut g);
            g.disp[0]
        };
        // Contact is at d = 2, the reach at d = 2.4: no jump on either side.
        assert!((pull(2.0 - 1e-4) - pull(2.0 + 1e-4)).abs() < 1e-4);
        assert!(pull(2.4 - 1e-4).abs() < 1e-4);
        assert_eq!(pull(2.4 + 1e-4), 0.0);
        assert!(pull(1.5) > 0.0 && pull(2.2) < 0.0);
    }

    #[test]
    fn hormone_falls_to_the_cell_below_and_none_is_lost() {
        let p = Params {
            diffusion: 0.0,
            production: 0.0,
            decay: 0.0,
            ..Params::default()
        };
        let (top, bottom) = ([0.0, 1.0, 0.0, 1.0], [0.0, -1.0, 0.0, 1.0]);
        let (mut a, mut b) = (Gathered::default(), Gathered::default());
        interact(&top, 1.0, &bottom, 1.0, &p, &mut a);
        interact(&bottom, 1.0, &top, 1.0, &p, &mut b);
        let (ha, hb): (f32, f32) = (finish(&top, 1.0, &a, &p).1, finish(&bottom, 1.0, &b, &p).1);
        assert!(ha < 1.0 && hb > 1.0, "top {ha}, bottom {hb}");
        assert!((ha + hb - 2.0).abs() < 1e-6);
    }

    #[test]
    fn a_cell_without_hormone_does_not_grow() {
        let p = Params::default();
        let cell = [0.0, 0.0, 0.0, 0.9];
        assert_eq!(finish(&cell, 0.0, &Gathered::default(), &p).0[3], 0.9);
        assert!(finish(&cell, 1.0, &Gathered::default(), &p).0[3] > 0.9);
    }

    #[test]
    fn a_full_cell_stops_dividing_at_capacity() {
        let p = Params::default();
        let mut cells = Cells::seed(2, p.r_div, 1.0);
        divide(&mut cells, &p, 0);
        assert_eq!(cells.count, 2);
        cells.pr[0][3] = p.r_div;
        let before = cells.clone();
        divide(&mut cells, &p, 1);
        assert_eq!(cells, before);
    }
}
