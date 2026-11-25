//! The local rules of a cell. Every rule is written here first, then mirrored in the GPU shaders.

use crate::cells::{Cell, Cells};
use crate::hash::axis;
use crate::params::{INV_CBRT2, Params};

/// What a cell gathers from its neighbours during one step.
#[derive(Clone, Copy, Debug, Default)]
pub struct Gathered {
    /// Displacement from pushes and pulls.
    pub disp: [f32; 3],
    /// Sum of hormone differences with the neighbours (passive diffusion), per hormone.
    pub spread: [f32; 2],
    /// Auxin received from the neighbours above minus auxin handed to those below.
    pub fall: f32,
    /// Net auxin flow through the cell, as a vector (sum of what each neighbour sends, along the
    /// line from that neighbour to the cell).
    pub flux: [f32; 3],
}

fn dot(a: &[f32], b: &[f32]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Distance from the centre of a cell to its surface along the unit vector `u`.
#[inline]
pub fn extent(r: f32, shape: &[f32; 4], u: &[f32; 3]) -> f32 {
    let c = dot(shape, u);
    let k2 = shape[3] * shape[3];
    r / (c * c / (k2 * k2) + (1.0 - c * c) * k2).sqrt()
}

/// Contribution of neighbour `other` to cell `me` (gather form: each cell only writes itself).
#[inline]
pub fn interact(me: &Cell, other: &Cell, p: &Params, g: &mut Gathered) {
    let (pi, pj) = (&me.pr, &other.pr);
    let dv = [pi[0] - pj[0], pi[1] - pj[1], pi[2] - pj[2]];
    let d2 = dot(&dv, &dv);
    // Cheap bound first: no cell reaches further than its half-length r k^2.
    let far = p.reach * (pi[3] * me.shape[3] * me.shape[3] + pj[3] * other.shape[3] * other.shape[3]);
    if d2 >= far * far || d2 <= 1e-12 {
        return;
    }
    let d = d2.sqrt();
    let u = dv.map(|v| v / d);
    let rs = extent(pi[3], &me.shape, &u) + extent(pj[3], &other.shape, &u);
    let reach = p.reach * rs;
    if d >= reach {
        return;
    }
    // Overlap: linear repulsion. Gap: adhesion shaped as a tent that returns to zero at the reach,
    // so the force has no jump (a jump would amplify rounding differences).
    let s = rs - d;
    let pull = (-s).min(reach - d);
    let coef = (if s > 0.0 { p.mob_rep * s } else { -p.mob_adh * pull }) / d;
    g.disp[0] += coef * dv[0];
    g.disp[1] += coef * dv[1];
    g.disp[2] += coef * dv[2];
    let (hi, hj) = (me.h, other.h);
    g.spread[0] += hj[0] - hi[0];
    g.spread[1] += hj[1] - hi[1];
    // How much `me` sits above `other`, from -1 to 1. Both cells compute exact opposites, so what
    // one hands over is exactly what the other receives.
    let above = dot(&dv, &p.gravity) / -d;
    let moved = hj[0] * (-above).max(0.0) - hi[0] * above.max(0.0);
    g.fall += moved;
    let sent = p.diffusion * (hj[0] - hi[0]) + p.polar * moved;
    g.flux[0] += sent * u[0];
    g.flux[1] += sent * u[1];
    g.flux[2] += sent * u[2];
}

/// New state of a cell once every neighbour has been gathered.
#[inline]
pub fn finish(me: &Cell, g: &Gathered, p: &Params) -> Cell {
    let (pi, s, hi) = (&me.pr, &me.shape, &me.h);
    let volume = pi[3] * pi[3] * pi[3];
    let auxin =
        hi[0] + p.diffusion * g.spread[0] + p.polar * g.fall + p.production * volume - p.decay * hi[0];
    // Cytokinin is made where auxin is, then wanders with no preferred direction.
    let cytokinin = hi[1] + p.ck_diffusion * g.spread[1] + p.ck_production * hi[0] - p.ck_decay * hi[1];
    // Two ways to gain volume: holding a lot of auxin, or holding cytokinin while auxin is scarce.
    let (ca, cc) = (hi[0] / volume, hi[1] / volume);
    let drive = ca / (p.half_sat + ca) + (cc / (p.ck_half + cc)) * (p.ck_block / (p.ck_block + ca));
    let r = (pi[3] + p.growth * drive.min(1.0)).min(p.r_div);
    // Stretching, at constant volume: some auxin helps, too much holds it back.
    let k = (s[3] + p.elong * (ca / (p.e_half + ca)) * (p.e_block / (p.e_block + ca))).min(p.stretch_max);
    // The long axis turns toward the line along which auxin flows through the cell (either way).
    let inv = 1.0 / (dot(&g.flux, &g.flux) + 1e-12).sqrt();
    let f = g.flux.map(|v| v * inv);
    let turn = p.align * dot(s, &f);
    let a = [s[0] + turn * f[0], s[1] + turn * f[1], s[2] + turn * f[2]];
    let inv = 1.0 / dot(&a, &a).max(1e-12).sqrt();
    Cell {
        pr: [pi[0] + g.disp[0], pi[1] + g.disp[1], pi[2] + g.disp[2], r],
        shape: [a[0] * inv, a[1] * inv, a[2] * inv, k],
        h: [auxin, cytokinin],
    }
}

/// One division round, in index order. Volume and hormones are split evenly between the two
/// daughters. The division direction is random, stretched along the long axis of the cell as much
/// as the cell itself is stretched, and the daughters are that much shorter. The daughter of the
/// n-th dividing cell gets index `count + n`; nothing divides beyond the capacity.
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
            let (m, s) = (cells.pr[i], cells.shape[i]);
            let rnd = axis(p.seed, i as u32, epoch);
            let w = (s[3] * s[3] * s[3] - 1.0) * dot(&rnd, &s);
            let dir = [rnd[0] + w * s[0], rnd[1] + w * s[1], rnd[2] + w * s[2]];
            let inv = 1.0 / dot(&dir, &dir).max(1e-12).sqrt();
            let r = m[3] * INV_CBRT2;
            let o = dir.map(|c| c * inv * (0.5 * r));
            let shape = [s[0], s[1], s[2], (s[3] * INV_CBRT2).max(1.0)];
            let h = cells.h[i].map(|v| v * 0.5);
            cells.set(
                i,
                Cell {
                    pr: [m[0] - o[0], m[1] - o[1], m[2] - o[2], r],
                    shape,
                    h,
                },
            );
            cells.set(
                j,
                Cell {
                    pr: [m[0] + o[0], m[1] + o[1], m[2] + o[2], r],
                    shape,
                    h,
                },
            );
        }
    }
    cells.count = (count + rank).min(capacity);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(at: [f32; 3], r: f32, h: [f32; 2]) -> Cell {
        Cell {
            pr: [at[0], at[1], at[2], r],
            shape: [1.0, 0.0, 0.0, 1.0],
            h,
        }
    }

    #[test]
    fn force_is_continuous_at_contact_and_at_the_reach() {
        let p = Params::default();
        let pull = |d: f32| {
            let mut g = Gathered::default();
            interact(
                &cell([d, 0.0, 0.0], 1.0, [0.0; 2]),
                &cell([0.0; 3], 1.0, [0.0; 2]),
                &p,
                &mut g,
            );
            g.disp[0]
        };
        // Contact is at d = 2, the reach at d = 2.4: no jump on either side.
        assert!((pull(2.0 - 1e-4) - pull(2.0 + 1e-4)).abs() < 1e-4);
        assert!(pull(2.4 - 1e-4).abs() < 1e-4);
        assert_eq!(pull(2.4 + 1e-4), 0.0);
        assert!(pull(1.5) > 0.0 && pull(2.2) < 0.0);
    }

    #[test]
    fn a_stretched_cell_is_long_along_its_axis_and_thin_across() {
        let s = [0.0, 1.0, 0.0, 1.5];
        assert!((extent(1.0, &[1.0, 0.0, 0.0, 1.0], &[0.6, 0.8, 0.0]) - 1.0).abs() < 1e-6);
        assert!((extent(1.0, &s, &[0.0, 1.0, 0.0]) - 2.25).abs() < 1e-5);
        assert!((extent(1.0, &s, &[1.0, 0.0, 0.0]) - 1.0 / 1.5).abs() < 1e-6);
    }

    #[test]
    fn auxin_falls_to_the_cell_below_and_none_is_lost() {
        let p = Params {
            diffusion: 0.0,
            production: 0.0,
            decay: 0.0,
            ..Params::default()
        };
        let (top, bottom) = (
            cell([0.0, 1.0, 0.0], 1.0, [1.0, 0.0]),
            cell([0.0, -1.0, 0.0], 1.0, [1.0, 0.0]),
        );
        let (mut a, mut b) = (Gathered::default(), Gathered::default());
        interact(&top, &bottom, &p, &mut a);
        interact(&bottom, &top, &p, &mut b);
        let (ha, hb) = (finish(&top, &a, &p).h[0], finish(&bottom, &b, &p).h[0]);
        assert!(ha < 1.0 && hb > 1.0, "top {ha}, bottom {hb}");
        assert!((ha + hb - 2.0).abs() < 1e-6);
        // The flow through both cells points down.
        assert!(a.flux[1] < 0.0 && b.flux[1] < 0.0, "{a:?} {b:?}");
    }

    #[test]
    fn a_cell_without_hormone_does_not_grow() {
        let p = Params::default();
        let grown = |h: [f32; 2]| finish(&cell([0.0; 3], 0.9, h), &Gathered::default(), &p).pr[3];
        assert_eq!(grown([0.0, 0.0]), 0.9);
        assert!(grown([1.0, 0.0]) > 0.9);
    }

    #[test]
    fn cytokinin_makes_a_cell_grow_only_where_auxin_is_scarce() {
        let p = Params::default();
        let grown = |h: [f32; 2]| finish(&cell([0.0; 3], 0.9, h), &Gathered::default(), &p).pr[3] - 0.9;
        assert!(grown([0.0, 1.0]) > 0.0);
        // With plenty of auxin, the same cytokinin adds almost nothing.
        assert!(grown([5.0, 1.0]) - grown([5.0, 0.0]) < 0.1 * grown([0.0, 1.0]));
    }

    #[test]
    fn some_auxin_stretches_a_cell_and_too_much_holds_it_back() {
        let p = Params {
            elong: 0.01,
            e_half: 0.5,
            e_block: 0.5,
            stretch_max: 2.0,
            ..Params::default()
        };
        let k = |auxin: f32| finish(&cell([0.0; 3], 1.0, [auxin, 0.0]), &Gathered::default(), &p).shape[3];
        assert_eq!(k(0.0), 1.0);
        assert!(
            k(0.5) > k(0.05) && k(0.5) > k(50.0),
            "{} {} {}",
            k(0.05),
            k(0.5),
            k(50.0)
        );
    }

    #[test]
    fn the_long_axis_turns_toward_the_auxin_flow_either_way() {
        let p = Params {
            align: 0.1,
            ..Params::default()
        };
        let tilted = Cell {
            shape: [0.6, -0.8, 0.0, 1.0],
            ..cell([0.0; 3], 1.0, [1.0, 0.0])
        };
        for flow in [-1.0, 1.0] {
            let g = Gathered {
                flux: [0.0, flow, 0.0],
                ..Gathered::default()
            };
            let a = finish(&tilted, &g, &p).shape;
            assert!(a[1].abs() > 0.8, "{a:?}");
            assert!((dot(&a, &a) - 1.0).abs() < 1e-6);
        }
    }

    #[test]
    fn a_stretched_cell_divides_across_its_length() {
        let p = Params::default();
        let mut cells = Cells::seed(&p, 2, 1.0);
        cells.shape[0] = [0.0, 0.0, 1.0, 1.5];
        divide(&mut cells, &p, 0);
        let gap = [0, 1, 2].map(|c| cells.pr[1][c] - cells.pr[0][c]);
        let along = gap[2].abs() / dot(&gap, &gap).sqrt();
        assert!(along > 0.9, "daughters along {gap:?}");
        assert!((cells.shape[0][3] - 1.5 * INV_CBRT2).abs() < 1e-6);
    }

    #[test]
    fn a_full_cell_stops_dividing_at_capacity() {
        let p = Params::default();
        let mut cells = Cells::seed(&p, 2, 1.0);
        divide(&mut cells, &p, 0);
        assert_eq!(cells.count, 2);
        cells.pr[0][3] = p.r_div;
        let before = cells.clone();
        divide(&mut cells, &p, 1);
        assert_eq!(cells, before);
    }
}
