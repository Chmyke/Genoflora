//! The local rules of a cell. Every rule is written here first, then mirrored in the GPU shaders.

use crate::cells::Cells;
use crate::hash::axis;
use crate::params::{INV_CBRT2, Params};

/// Contribution of neighbour `j` to cell `i`: displacement and hormone exchange (gather form).
#[inline]
pub fn interact(
    pi: &[f32; 4],
    hi: f32,
    pj: &[f32; 4],
    hj: f32,
    p: &Params,
    disp: &mut [f32; 3],
    hs: &mut f32,
) {
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
        disp[0] += coef * dv[0];
        disp[1] += coef * dv[1];
        disp[2] += coef * dv[2];
        *hs += hj - hi;
    }
}

/// New state of cell `i` once every neighbour has been gathered.
#[inline]
pub fn finish(pi: &[f32; 4], hi: f32, disp: &[f32; 3], hs: f32, p: &Params) -> ([f32; 4], f32) {
    let source = if pi[0] > 0.0 { p.production } else { 0.0 };
    let h = hi + p.diffusion * hs + source - p.decay * hi;
    let r = (pi[3] + p.growth).min(p.r_div);
    ([pi[0] + disp[0], pi[1] + disp[1], pi[2] + disp[2], r], h)
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
            let mut disp = [0.0; 3];
            let mut hs = 0.0;
            interact(
                &[d, 0.0, 0.0, 1.0],
                0.0,
                &[0.0, 0.0, 0.0, 1.0],
                0.0,
                &p,
                &mut disp,
                &mut hs,
            );
            disp[0]
        };
        // Contact is at d = 2, the reach at d = 2.4: no jump on either side.
        assert!((pull(2.0 - 1e-4) - pull(2.0 + 1e-4)).abs() < 1e-4);
        assert!(pull(2.4 - 1e-4).abs() < 1e-4);
        assert_eq!(pull(2.4 + 1e-4), 0.0);
        assert!(pull(1.5) > 0.0 && pull(2.2) < 0.0);
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
