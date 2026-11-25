//! The local rules of a cell. Every rule is written here first, then mirrored in the GPU shaders.

use crate::cells::{Cell, Cells};
use crate::hash::axis;
use crate::params::{INV_CBRT2, LENGTH_MARGIN, Params};
use crate::shape::{HALF, dir, extent, grow_along, grow_even, stretched, with_dir};

/// What a cell gathers from its neighbours during one step.
#[derive(Clone, Copy, Debug, Default)]
pub struct Gathered {
    /// Displacement from pushes and pulls.
    pub disp: [f32; 3],
    /// Sum over the neighbours of the hormone each would send by passive diffusion, per unit of
    /// diffusion rate.
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

/// Contribution of neighbour `other` to cell `me` (gather form: each cell only writes itself).
#[inline]
pub fn interact(me: &Cell, other: &Cell, p: &Params, g: &mut Gathered) {
    let (pi, pj) = (&me.pr, &other.pr);
    let dv = [pi[0] - pj[0], pi[1] - pj[1], pi[2] - pj[2]];
    let d2 = dot(&dv, &dv);
    // Cheap bound first: no cell reaches further than its half-length.
    let far = p.reach * (me.shape[HALF] + other.shape[HALF]);
    if d2 >= far * far || d2 <= 1e-12 {
        return;
    }
    let d = d2.sqrt();
    let u = dv.map(|v| v / d);
    let rs = extent(&me.shape, &u) + extent(&other.shape, &u);
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
    // Diffusion follows concentrations: a big cell does not leak into a small one as rich as it.
    // Moving (cj - ci) vi vj / (vi + vj) would level the pair exactly; `diffusion` is the share of
    // that move made per step (twice it, so that equal cells swap diffusion * amount difference).
    let (hi, hj) = (me.h, other.h);
    let (vi, vj) = (pi[3] * pi[3] * pi[3], pj[3] * pj[3] * pj[3]);
    let level = 2.0 * vi * vj / (vi + vj);
    let gap = [
        (hj[0] / vj - hi[0] / vi) * level,
        (hj[1] / vj - hi[1] / vi) * level,
    ];
    g.spread[0] += gap[0];
    g.spread[1] += gap[1];
    // How much `me` sits above `other`, from -1 to 1. Both cells compute exact opposites, so what
    // one hands over is exactly what the other receives.
    let above = dot(&dv, &p.gravity) / -d;
    let moved = hj[0] * (-above).max(0.0) - hi[0] * above.max(0.0);
    g.fall += moved;
    let sent = p.diffusion * gap[0] + p.polar * moved;
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
    // Even growth, up to the dividing size: holding a lot of auxin, or holding cytokinin while
    // auxin is scarce.
    let (ca, cc) = (hi[0] / volume, hi[1] / volume);
    let drive = ca / (p.half_sat + ca) + (cc / (p.ck_half + cc)) * (p.ck_block / (p.ck_block + ca));
    let even = (pi[3] + p.growth * drive.min(1.0)).min(p.r_div).max(pi[3]);
    // The growth direction (how the wall lets the cell grow) turns toward the line of the auxin
    // flow, the faster the stronger the flow. The cell itself never turns.
    let old = dir(s);
    let turn = p.align * dot(&old, &g.flux);
    let a = [
        old[0] + turn * g.flux[0],
        old[1] + turn * g.flux[1],
        old[2] + turn * g.flux[2],
    ];
    let inv = 1.0 / dot(&a, &a).max(1e-12).sqrt();
    let a = a.map(|v| v * inv);
    // Growth along that direction adds volume, like water taken up: some auxin helps, too much
    // holds it back, and it fades as the cell nears its longest.
    let room = (1.0 - s[HALF] / (p.r_div * p.max_half_length)).max(0.0);
    let eps = p.elong * (ca / (p.e_half + ca)) * (p.e_block / (p.e_block + ca)) * room;
    let shape = with_dir(&grow_along(&grow_even(s, even / pi[3]), &a, eps), a);
    Cell {
        pr: [
            pi[0] + g.disp[0],
            pi[1] + g.disp[1],
            pi[2] + g.disp[2],
            even * (1.0 + eps).powf(1.0 / 3.0),
        ],
        shape,
        h: [auxin, cytokinin],
    }
}

/// One division round, in index order. A cell divides when it reaches the dividing size or its
/// longest half-length. The division direction is random, leaning toward the long axis as much as
/// the cell is long; the cell is cut in two halves across it, which share volume and hormones.
/// The daughter of the n-th dividing cell gets index `count + n`; nothing divides beyond the
/// capacity.
pub fn divide(cells: &mut Cells, p: &Params, epoch: u32) {
    let count = cells.count;
    let capacity = cells.capacity();
    let longest = p.r_div * p.max_half_length * LENGTH_MARGIN;
    let mut rank = 0;
    for i in 0..count {
        let (m, s) = (cells.pr[i], cells.shape[i]);
        if m[3] >= p.r_div || s[HALF] >= longest {
            let j = count + rank;
            rank += 1;
            if j >= capacity {
                continue;
            }
            let v = stretched(&s, &axis(p.seed, i as u32, epoch));
            let inv = 1.0 / dot(&v, &v).max(1e-24).sqrt();
            let d = v.map(|c| c * inv);
            let o = d.map(|c| c * (0.5 * extent(&s, &d)));
            let shape = grow_along(&s, &d, -0.5);
            let r = m[3] * INV_CBRT2;
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
    use crate::shape::{radius, sphere};

    fn cell(at: [f32; 3], r: f32, h: [f32; 2]) -> Cell {
        Cell {
            pr: [at[0], at[1], at[2], r],
            shape: sphere(r, [1.0, 0.0, 0.0]),
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
    fn equally_rich_cells_exchange_nothing_whatever_their_size() {
        let p = Params {
            polar: 0.0,
            ..Params::default()
        };
        // Same concentration (amount over r^3), different sizes.
        let (big, small) = (
            cell([0.0; 3], 1.0, [1.0, 0.0]),
            cell([1.6, 0.0, 0.0], 0.6, [0.216, 0.0]),
        );
        let mut g = Gathered::default();
        interact(&big, &small, &p, &mut g);
        assert!(g.spread[0].abs() < 1e-6, "{g:?}");
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
    fn some_auxin_lengthens_a_cell_and_too_much_holds_it_back() {
        let p = Params {
            growth: 0.0,
            elong: 0.01,
            e_half: 0.5,
            e_block: 0.5,
            max_half_length: 3.0,
            ..Params::default()
        };
        let after = |auxin: f32| {
            finish(
                &cell([0.0; 3], 0.8, [auxin * 0.512, 0.0]),
                &Gathered::default(),
                &p,
            )
        };
        let length = |c: Cell| extent(&c.shape, &[1.0, 0.0, 0.0]);
        assert_eq!(length(after(0.0)), 0.8);
        assert!(length(after(0.5)) > length(after(0.05)) && length(after(0.5)) > length(after(50.0)));
        // Lengthening is growth: the volume goes up, the width does not change.
        let c = after(0.5);
        assert!(
            c.pr[3] > 0.8 && (c.pr[3] - radius(&c.shape)).abs() < 1e-5,
            "{c:?}"
        );
        assert!((extent(&c.shape, &[0.0, 1.0, 0.0]) - 0.8).abs() < 1e-5);
    }

    #[test]
    fn the_growth_direction_turns_with_the_flow_not_the_cell() {
        let p = Params {
            align: 1.0,
            ..Params::default()
        };
        let tilted = Cell {
            shape: sphere(1.0, [0.6, -0.8, 0.0]),
            ..cell([0.0; 3], 1.0, [1.0, 0.0])
        };
        let turned = |flow: f32| {
            let g = Gathered {
                flux: [0.0, flow, 0.0],
                ..Gathered::default()
            };
            dir(&finish(&tilted, &g, &p).shape)
        };
        // Either way along the flow, and the stronger the flow the further it turns.
        assert!(turned(1.0)[1].abs() > 0.9 && turned(-1.0)[1].abs() > 0.9);
        assert!(turned(0.01)[1].abs() < turned(0.1)[1].abs() && turned(0.1)[1].abs() < turned(1.0)[1].abs());
        // A long cell keeps its length where it was: nothing rotates it.
        let long = Cell {
            shape: grow_along(&tilted.shape, &[0.6, -0.8, 0.0], 1.0),
            ..tilted
        };
        let g = Gathered {
            flux: [0.0, 1.0, 0.0],
            ..Gathered::default()
        };
        assert!((extent(&finish(&long, &g, &p).shape, &[0.6, -0.8, 0.0]) - 2.0).abs() < 1e-4);
    }

    #[test]
    fn a_long_cell_divides_across_its_length() {
        let p = Params {
            max_half_length: 1.8,
            ..Params::default()
        };
        let mut cells = Cells::seed(&p, 2, 1.0);
        cells.shape[0] = grow_along(&sphere(0.7, [0.0, 0.0, 1.0]), &[0.0, 0.0, 1.0], 1.6);
        cells.pr[0][3] = radius(&cells.shape[0]);
        divide(&mut cells, &p, 0);
        assert_eq!(cells.count, 2);
        let gap = [0, 1, 2].map(|c| cells.pr[1][c] - cells.pr[0][c]);
        assert!(
            gap[2].abs() / dot(&gap, &gap).sqrt() > 0.9,
            "daughters along {gap:?}"
        );
        assert!((cells.pr[0][3] - radius(&cells.shape[0])).abs() < 1e-5);
        assert!((extent(&cells.shape[0], &[0.0, 0.0, 1.0]) - 0.91).abs() < 0.05);
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
