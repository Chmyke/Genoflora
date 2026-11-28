//! The local rules of a cell. Every rule is written here first, then mirrored in the GPU shaders.

use crate::cells::{Cell, Cells};
use crate::hash::axis;
use crate::params::{INV_CBRT2, LENGTH_MARGIN, Params, YOUNG};
use crate::shape::{HALF, dir, extent, grow_along, grow_even, stretched, with_dir};
use crate::soil;

/// What a cell gathers from its neighbours during one step.
#[derive(Clone, Copy, Debug, Default)]
pub struct Gathered {
    /// Displacement from pushes and pulls.
    pub disp: [f32; 3],
    /// Sum over the neighbours of the hormone each would send by passive diffusion, per unit of
    /// diffusion rate.
    pub spread: [f32; 4],
    /// Auxin pumped in by the neighbours minus auxin pumped out to them (PIN transport).
    pub pumped: f32,
    /// Net auxin flow through the cell, as a vector (sum of what each neighbour sends, along the
    /// line from that neighbour to the cell).
    pub flux: [f32; 3],
    /// Sum over the neighbours of their auxin concentration times the unit vector from the cell
    /// to them...
    pub richer: [f32; 3],
    /// ...and the sum of those concentrations.
    pub richness: f32,
    /// Sum of the unit vectors from the neighbours to the cell...
    pub around: [f32; 3],
    /// ...and how many neighbours there are.
    pub neighbours: f32,
    /// How much the neighbours cover the cell from above: each adds how squarely it sits above.
    pub cover: f32,
    /// How hard the neighbours press on the cell: the sum of how deep each one overlaps it.
    pub squeeze: f32,
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
    g.around[0] += u[0];
    g.around[1] += u[1];
    g.around[2] += u[2];
    g.neighbours += 1.0;
    g.cover += dot(&u, &p.gravity).max(0.0);
    let s = rs - d;
    g.squeeze += s.max(0.0);
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
    let gap = [0, 1, 2, 3].map(|k| (hj[k] / vj - hi[k] / vi) * level);
    for (sum, part) in g.spread.iter_mut().zip(gap) {
        *sum += part;
    }
    // Active transport: each cell pumps auxin out on the side its pumps sit, in proportion to how
    // squarely the neighbour faces that side. `u` points from `other` to `me`. Both cells compute
    // exact opposites, so what one pumps out is exactly what the other receives.
    let (qi, qj) = (&me.pin, &other.pin);
    let moved = hj[0] * dot(qj, &u).max(0.0) - hi[0] * (-dot(qi, &u)).max(0.0);
    g.pumped += moved;
    let cj = hj[0] / vj;
    g.richer[0] -= cj * u[0];
    g.richer[1] -= cj * u[1];
    g.richer[2] -= cj * u[2];
    g.richness += cj;
    let sent = p.diffusion * gap[0] + p.polar * moved;
    g.flux[0] += sent * u[0];
    g.flux[1] += sent * u[1];
    g.flux[2] += sent * u[2];
}

/// How strongly cytokinin calls in a cell, from 0 to 1: it holds cytokinin while auxin is scarce.
/// A cell has brakes on its answer to cytokinin, and the relay it holds lifts them (WUS switches
/// off the type-A ARR genes, Leibfried et al. 2005): a cell that holds relay hears cytokinin
/// better, so the relay sustains its own making.
#[inline]
pub fn ck_call(p: &Params, auxin: f32, cytokinin: f32, relay: f32) -> f32 {
    let brakes = p.relay_frees / (p.relay_frees + relay);
    let half = p.ck_half * (1.0 + p.ck_brake * brakes);
    (cytokinin / (half + cytokinin)) * (p.ck_block / (p.ck_block + auxin))
}

/// Share of the cytokinin call a cell still hears through the restraint signal it holds.
#[inline]
pub fn heard(p: &Params, restraint: f32) -> f32 {
    p.restraint_half / (p.restraint_half + restraint)
}

/// Share of the walls of a cell that face no other cell, from 0 to 1: how far its neighbours lie
/// on one side of it. It is what the cell touches of the soil around it. A cell surrounded evenly
/// reads little (0.06 to 0.18 measured inside a stem), a flat side one half, a lone cell one.
#[inline]
pub fn exposed(g: &Gathered) -> f32 {
    if g.neighbours > 0.0 {
        dot(&g.around, &g.around).sqrt() / g.neighbours
    } else {
        1.0
    }
}

/// Whether a cell belongs to the skin, from 0 to 1: how much the directions of its neighbours
/// agree beyond what chance gives. It is the mean cosine between two of them (1 when all lie the
/// same way, 0 when they lie at random, below 0 when they are spread evenly), as a length.
///
/// WHY: not `exposed`. Being skin is an identity, not a share: measured 2026-11-26, the little
/// that `exposed` reads inside, times the many inner cells, made two thirds of the restraint
/// signal the skin alone was meant to make.
#[inline]
pub fn skin(g: &Gathered) -> f32 {
    let n = g.neighbours;
    if n < 2.0 {
        return 1.0;
    }
    ((dot(&g.around, &g.around) - n) / (n * n - n)).max(0.0).sqrt()
}

/// New state of a cell once every neighbour has been gathered.
#[inline]
pub fn finish(me: &Cell, g: &Gathered, p: &Params) -> Cell {
    let (pi, s, hi) = (&me.pr, &me.shape, &me.h);
    let volume = pi[3] * pi[3] * pi[3];
    // Auxin is made everywhere, more where light reaches the cell.
    let lit = soil::shaded(p, soil::light(p, pi), g.cover);
    let made = (p.production + p.light_production * lit) * volume;
    // Auxin switches on its own breakdown: a cell full of it loses it faster.
    let dose = hi[0] / volume;
    let burnt = p.burn * hi[0] * dose / (p.burn_half + dose);
    let auxin = hi[0] + p.diffusion * g.spread[0] + p.polar * g.pumped + made - p.decay * hi[0] - burnt;
    // Cytokinin is made where auxin is and where light reaches the cell, then wanders with no
    // preferred direction.
    let cytokinin =
        hi[1] + p.ck_diffusion * g.spread[1] + p.ck_production * hi[0] + p.ck_light * lit * volume
            - p.ck_decay * hi[1];
    // Even growth, up to the dividing size: holding a lot of auxin, or holding cytokinin while
    // auxin is scarce.
    let (ca, cc, cw) = (hi[0] / volume, hi[1] / volume, hi[3] / volume);
    let ck_push = ck_call(p, ca, cc, cw);
    let drive = ca / (p.half_sat + ca) + ck_push;
    // The soil holds the cell and pushes back on it, but only through the walls that press on soil
    // rather than on other cells: a cell surrounded evenly touches no soil, a lone cell or the tip
    // of a column does. Held everywhere, the cells inside a root could not make room for their
    // growth and piled up (measured 2026-11-25: 40 neighbours, overlaps deeper than a radius).
    let soil = soil::share(p, pi);
    let exposed = exposed(g);
    // Cytokinin does not keep a cell young by itself: where it calls, the cell makes a relay that
    // passes from cell to cell, and the relay does. The restraint signal dulls that call.
    let reading = ck_push * heard(p, hi[2] / volume);
    let relay =
        hi[3] + p.relay_diffusion * g.spread[3] + p.relay_made * reading * volume - p.relay_decay * hi[3];
    // A cell of the skin that holds relay makes the restraint signal, which goes back to where
    // the relay is made.
    let told = cw / (p.relay_half + cw);
    let restraint = hi[2] + p.restraint_diffusion * g.spread[2] + p.restraint_made * told * skin(g) * volume
        - p.restraint_decay * hi[2];
    let push = dot(&g.disp, &g.disp).sqrt();
    let held = (1.0 - p.soil_grip * soil * exposed / push.max(1e-12)).max(0.0);
    // A cell grows by pushing its wall out with its own pressure: what presses back on the wall
    // holds the growth, and stops it once it matches that pressure (Lockhart 1965).
    let free = (1.0 - p.squeeze_hold * g.squeeze / pi[3]).max(0.0);
    let slowed = free / (1.0 + p.soil_hardness * soil * exposed);
    let even = (pi[3] + p.growth * drive.min(1.0) * slowed)
        .min(p.r_div)
        .max(pi[3]);
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
    let eps = p.elong * (ca / (p.e_half + ca)) * (p.e_block / (p.e_block + ca)) * room * slowed;
    let shape = with_dir(&grow_along(&grow_even(s, even / pi[3]), &a, eps), a);
    let mut pin = polarise(me, g, p, ca);
    pin[3] = lit;
    // A cell stays young where auxin gathers (root tip), or where cytokinin gathers while auxin
    // is scarce (shoot tip); it matures elsewhere, and turns young again if they come back.
    let calls = (ca / (p.youth_half + ca) + told).min(1.0);
    let youth = me.youth + p.youth_rate * (calls - me.youth);
    Cell {
        pr: [
            pi[0] + held * g.disp[0],
            pi[1] + held * g.disp[1],
            pi[2] + held * g.disp[2],
            even * (1.0 + eps).powf(1.0 / 3.0),
        ],
        shape,
        h: [auxin, cytokinin, restraint, relay],
        pin,
        youth,
    }
}

/// The pumps move toward a target made of two ways of placing them, both at work in every cell
/// (Bayer et al. 2009), plus the pull of gravity:
/// - toward the neighbours that hold the most auxin (up the gradient, Jönsson et al. 2006),
///   which wins while the cell holds little auxin;
/// - along the auxin already flowing through the cell (with the flux, canalisation, Mitchison
///   1980), which wins once it holds a lot: the more auxin flows one way, the more is pumped
///   that way;
/// - toward the lower side, where the heavy grains of the cell settle (statoliths).
///
/// The polarity never gets longer than one (all the pumps on one side).
fn polarise(me: &Cell, g: &Gathered, p: &Params, auxin: f32) -> [f32; 4] {
    let follow = auxin / (p.pin_switch + auxin);
    let uphill = (1.0 - follow) / g.richness.max(1e-12);
    let strength = dot(&g.flux, &g.flux).sqrt();
    let along = follow / (p.flux_half + strength);
    let q = &me.pin;
    // What the neighbours hold on average, taken out of the sum as far as the cell reads fairly:
    // what is left is how they differ, which even neighbours do not.
    let even = p.pin_fair * g.richness / g.neighbours.max(1.0);
    // The skin holds no heavy grains: as far as a cell is skin, it senses gravity less.
    let down = p.gravisense * (1.0 - p.grav_skin * skin(g)).max(0.0);
    let t = [0, 1, 2]
        .map(|k| uphill * (g.richer[k] + even * g.around[k]) + along * g.flux[k] + down * p.gravity[k]);
    let n = [0, 1, 2].map(|k| q[k] + p.pin_rate * (t[k] - q[k]));
    let len = dot(&n, &n).sqrt();
    let keep = if len > 1.0 { 1.0 / len } else { 1.0 };
    [n[0] * keep, n[1] * keep, n[2] * keep, 0.0]
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
        if cells.youth[i] > YOUNG && (m[3] >= p.r_div || s[HALF] >= longest) {
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
            let pin = cells.pin[i];
            let youth = cells.youth[i];
            cells.set(
                i,
                Cell {
                    pr: [m[0] - o[0], m[1] - o[1], m[2] - o[2], r],
                    shape,
                    h,
                    pin,
                    youth,
                },
            );
            cells.set(
                j,
                Cell {
                    pr: [m[0] + o[0], m[1] + o[1], m[2] + o[2], r],
                    shape,
                    h,
                    pin,
                    youth,
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
            h: [h[0], h[1], 0.0, 0.0],
            pin: [0.0; 4],
            youth: 1.0,
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
    fn pumps_hand_auxin_to_the_side_they_sit_on_and_none_is_lost() {
        let p = Params {
            diffusion: 0.0,
            production: 0.0,
            light_production: 0.0,
            decay: 0.0,
            burn: 0.0,
            ..Params::default()
        };
        // Both cells pump down: the top one hands auxin to the bottom one, which faces no one.
        let down = |c: Cell| Cell {
            pin: [0.0, -1.0, 0.0, 0.0],
            ..c
        };
        let (top, bottom) = (
            down(cell([0.0, 1.0, 0.0], 1.0, [1.0, 0.0])),
            down(cell([0.0, -1.0, 0.0], 1.0, [1.0, 0.0])),
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
    fn cells_without_polarity_pump_nothing() {
        let p = Params {
            diffusion: 0.0,
            ..Params::default()
        };
        let mut g = Gathered::default();
        interact(
            &cell([0.0, 1.0, 0.0], 1.0, [1.0, 0.0]),
            &cell([0.0, -1.0, 0.0], 1.0, [5.0, 0.0]),
            &p,
            &mut g,
        );
        assert_eq!(g.pumped, 0.0);
    }

    /// Where the pumps of a lone cell head after `steps` steps, given what it gathered.
    fn pumps_after(p: &Params, auxin: f32, g: &Gathered, steps: u32) -> [f32; 4] {
        let mut c = cell([0.0; 3], 1.0, [auxin, 0.0]);
        for _ in 0..steps {
            c.pin = finish(&c, g, p).pin;
        }
        c.pin
    }

    #[test]
    fn a_cell_poor_in_auxin_turns_its_pumps_toward_its_richest_neighbours() {
        let p = Params {
            gravisense: 0.0,
            ..Params::default()
        };
        // A rich neighbour on +x, a poor one on -x, flow along +z.
        let g = Gathered {
            richer: [3.0 - 1.0, 0.0, 0.0],
            richness: 4.0,
            flux: [0.0, 0.0, 1.0],
            ..Gathered::default()
        };
        let q = pumps_after(&p, 0.01, &g, 2000);
        assert!(q[0] > 0.4 && q[0] > 10.0 * q[2].abs(), "{q:?}");
        // Rich in auxin, the same cell follows the flow instead.
        let q = pumps_after(&p, 100.0, &g, 2000);
        assert!(q[2] > 0.9 && q[2] > 10.0 * q[0].abs(), "{q:?}");
    }

    #[test]
    fn a_skin_cell_reading_fairly_does_not_pump_toward_neighbours_that_hold_the_same() {
        // Three neighbours on the -x side only, each holding 2: nothing to tell them apart.
        let g = Gathered {
            richer: [-6.0, 0.0, 0.0],
            richness: 6.0,
            around: [3.0, 0.0, 0.0],
            neighbours: 3.0,
            ..Gathered::default()
        };
        let p = Params {
            gravisense: 0.0,
            ..Params::default()
        };
        let q = pumps_after(&p, 0.01, &g, 2000);
        assert!(q[0] < -0.9, "adding up, the pumps go inward: {q:?}");
        let fair = Params { pin_fair: 1.0, ..p };
        let q = pumps_after(&fair, 0.01, &g, 2000);
        assert!(q[0].abs() < 1e-4, "reading fairly, they go nowhere: {q:?}");
        // A richer neighbour still pulls them: one of the three holds 4 instead of 2.
        let uneven = Gathered {
            richer: [-6.0, 2.0, 0.0],
            richness: 8.0,
            ..g
        };
        let q = pumps_after(&fair, 0.01, &uneven, 2000);
        assert!(q[1] > 0.1, "{q:?}");
    }

    #[test]
    fn a_skin_cell_that_holds_no_heavy_grains_is_not_pulled_down() {
        // Four neighbours on one side: fully skin. Reading them fairly, they pull the pumps nowhere.
        let skin_cell = Gathered {
            around: [4.0, 0.0, 0.0],
            neighbours: 4.0,
            ..Gathered::default()
        };
        assert!((skin(&skin_cell) - 1.0).abs() < 1e-6);
        let p = Params {
            pin_fair: 1.0,
            ..Params::default()
        };
        let q = pumps_after(&p, 1.0, &skin_cell, 1000);
        assert!(
            (q[1] + p.gravisense).abs() < 0.01,
            "every cell senses gravity: {q:?}"
        );
        let numb = Params { grav_skin: 5.0, ..p };
        let q = pumps_after(&numb, 1.0, &skin_cell, 1000);
        assert!(q[1].abs() < 1e-6, "the skin does not: {q:?}");
        // An inner cell, with neighbours all around it, still does.
        let inner = Gathered {
            neighbours: 6.0,
            ..Gathered::default()
        };
        let q = pumps_after(&numb, 1.0, &inner, 1000);
        assert!((q[1] + p.gravisense).abs() < 0.01, "{q:?}");
    }

    #[test]
    fn a_cell_full_of_auxin_breaks_it_down_faster() {
        let p = Params {
            burn: 0.01,
            burn_half: 10.0,
            ..Params::default()
        };
        let plain = Params { burn: 0.0, ..p };
        // What the breakdown takes in one step from a cell of volume one holding `auxin`.
        let burnt = |auxin: f32| {
            let c = cell([0.0; 3], 1.0, [auxin, 0.0]);
            let g = Gathered::default();
            finish(&c, &g, &plain).h[0] - finish(&c, &g, &p).h[0]
        };
        // Half the full pace at `burn_half`, next to nothing in a cell that holds little.
        assert!((burnt(10.0) - 0.5 * 0.01 * 10.0).abs() < 1e-5, "{}", burnt(10.0));
        assert!(burnt(0.1) < 0.02 * 0.01 * 0.1, "{}", burnt(0.1));
        assert!(burnt(1000.0) > 0.98 * 0.01 * 1000.0, "{}", burnt(1000.0));
    }

    #[test]
    fn gravity_only_pulls_the_pumps_of_a_cell_that_senses_it() {
        let g = Gathered::default();
        let numb = Params {
            gravisense: 0.0,
            ..Params::default()
        };
        assert_eq!(pumps_after(&numb, 1.0, &g, 500)[..3], [0.0; 3]);
        let p = Params::default();
        let q = pumps_after(&p, 1.0, &g, 1000);
        assert!((q[1] + p.gravisense).abs() < 0.01, "{q:?}");
        // The pumps move over time, not at once: the cell remembers where they were.
        let q = pumps_after(&p, 1.0, &g, 10);
        assert!(q[1] < -0.01 * p.gravisense && q[1] > -0.5 * p.gravisense, "{q:?}");
    }

    #[test]
    fn the_polarity_never_gets_longer_than_one() {
        let p = Params {
            gravisense: 5.0,
            ..Params::default()
        };
        let g = Gathered {
            flux: [1.0, 0.0, 0.0],
            ..Gathered::default()
        };
        let q = pumps_after(&p, 100.0, &g, 5000);
        assert!(dot(&q, &q) <= 1.0 + 1e-6, "{q:?}");
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
    fn a_lit_cell_makes_more_auxin_than_one_in_the_dark() {
        let p = Params {
            light_production: 0.01,
            ..Params::default()
        };
        let made =
            |height: f32| finish(&cell([0.0, height, 0.0], 1.0, [0.0; 2]), &Gathered::default(), &p).h[0];
        let (lit, dark) = (made(p.surface + 3.0), made(p.surface - 10.0));
        assert!((lit - p.production - p.light_production).abs() < 1e-6, "{lit}");
        assert!((dark - p.production).abs() < 1e-5, "{dark}");
    }

    #[test]
    fn a_cell_matures_without_auxin_and_turns_young_again_with_it() {
        let p = Params {
            youth_rate: 0.01,
            youth_half: 1.0,
            ck_block: 0.1,
            relay_made: 0.1,
            ..Params::default()
        };
        let after = |youth: f32, auxin: f32, steps: u32| {
            let mut c = Cell {
                youth,
                ..cell([0.0; 3], 1.0, [auxin, 0.0])
            };
            for _ in 0..steps {
                c.youth = finish(&c, &Gathered::default(), &p).youth;
            }
            c.youth
        };
        assert!(after(1.0, 0.0, 1000) < 0.01);
        // Cytokinin keeps a cell young too, through the relay it makes.
        let mut c = Cell {
            youth: 0.0,
            ..cell([0.0; 3], 1.0, [0.0, 50.0])
        };
        for _ in 0..2000 {
            let next = finish(&c, &Gathered::default(), &p);
            (c.youth, c.h[3]) = (next.youth, next.h[3]);
        }
        assert!(c.youth > 0.85, "{}", c.youth);
        assert!(after(0.0, 50.0, 1000) > 0.9);
        // Slowly: the cell remembers what it was.
        assert!(after(1.0, 0.0, 10) > 0.85);
    }

    #[test]
    fn cytokinin_makes_the_relay_and_the_restraint_signal_holds_it_back() {
        let p = Params {
            relay_made: 0.01,
            restraint_half: 1.0,
            ..Params::default()
        };
        // Relay made in one step by a cell holding these hormones (auxin, cytokinin, restraint).
        let made = |h: [f32; 3]| {
            let c = Cell {
                h: [h[0], h[1], h[2], 0.0],
                ..cell([0.0; 3], 1.0, [0.0; 2])
            };
            finish(&c, &Gathered::default(), &p).h[3]
        };
        assert_eq!(made([0.0, 0.0, 0.0]), 0.0);
        assert!(made([0.0, 50.0, 0.0]) > 0.009);
        // Auxin opposes cytokinin, and the restraint signal halves the call at `restraint_half`.
        assert!(made([5.0, 50.0, 0.0]) < 0.3 * made([0.0, 50.0, 0.0]));
        assert!((made([0.0, 50.0, 1.0]) - 0.5 * made([0.0, 50.0, 0.0])).abs() < 1e-6);
    }

    #[test]
    fn a_cell_that_holds_relay_hears_cytokinin_better() {
        let p = Params {
            ck_brake: 9.0,
            ..Params::default()
        };
        // Brakes full on: the half-answer is ten times further. Half lifted at `relay_frees`.
        let deaf = ck_call(&p, 0.0, p.ck_half, 0.0);
        let freed = ck_call(&p, 0.0, p.ck_half, p.relay_frees);
        assert!((deaf - 1.0 / 11.0).abs() < 1e-6, "{deaf}");
        assert!((freed - 1.0 / 6.5).abs() < 1e-6, "{freed}");
        assert!(ck_call(&p, 0.0, p.ck_half, 1.0e6) > 0.499);
        // Without brakes the relay changes nothing.
        let q = Params::default();
        assert_eq!(ck_call(&q, 0.0, 2.0, 0.0), ck_call(&q, 0.0, 2.0, 5.0));
        // And so the relay is made faster by a cell that already holds some.
        let made = |relay: f32| {
            let c = Cell {
                h: [0.0, p.ck_half, 0.0, relay],
                ..cell([0.0; 3], 1.0, [0.0; 2])
            };
            finish(&c, &Gathered::default(), &p).h[3] - (1.0 - p.relay_decay) * relay
        };
        assert!(made(1.0) > 1.5 * made(0.0), "{} {}", made(1.0), made(0.0));
    }

    #[test]
    fn the_relay_keeps_a_cell_young_and_fades() {
        let p = Params::default();
        let with = |relay: f32| Cell {
            h: [0.0, 0.0, 0.0, relay],
            youth: 0.0,
            ..cell([0.0; 3], 1.0, [0.0; 2])
        };
        let next = |relay: f32| finish(&with(relay), &Gathered::default(), &p);
        assert_eq!(next(0.0).youth, 0.0);
        assert!(next(9.0).youth > next(1.0).youth && next(1.0).youth > 0.0);
        assert!((next(1.0).youth - 0.5 * p.youth_rate).abs() < 1e-6);
        assert!((next(1.0).h[3] - (1.0 - p.relay_decay)).abs() < 1e-6);
    }

    #[test]
    fn only_a_skin_cell_holding_relay_makes_the_restraint_signal() {
        let p = Params {
            restraint_made: 0.01,
            ..Params::default()
        };
        let me = |relay: f32| Cell {
            h: [0.0, 0.0, 0.0, relay],
            ..cell([0.0; 3], 1.0, [0.0; 2])
        };
        // Neighbours on one side only (skin), or on both sides (inside the tissue).
        let made = |relay: f32, sides: &[f32]| {
            let mut g = Gathered::default();
            for &x in sides {
                interact(&me(relay), &cell([x, 0.0, 0.0], 1.0, [0.0; 2]), &p, &mut g);
            }
            finish(&me(relay), &g, &p).h[2]
        };
        assert!((made(1.0, &[2.0]) - 0.005).abs() < 1e-6);
        assert!(made(9.0, &[2.0]) > made(1.0, &[2.0]));
        assert_eq!(made(0.0, &[2.0]), 0.0);
        assert!(made(9.0, &[2.0, -2.0]) < 1e-9);
    }

    #[test]
    fn a_cell_is_skin_only_as_far_as_its_neighbours_lie_on_one_side() {
        let read = |dirs: &[[f32; 3]]| {
            let mut g = Gathered::default();
            for d in dirs {
                let len = dot(d, d).sqrt();
                for (sum, part) in g.around.iter_mut().zip(d) {
                    *sum += part / len;
                }
                g.neighbours += 1.0;
            }
            (skin(&g), exposed(&g))
        };
        let six = [
            [1.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, -1.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, -1.0],
        ];
        assert_eq!(read(&[]), (1.0, 1.0));
        assert_eq!(read(&six[..1]), (1.0, 1.0));
        // Surrounded unevenly (one neighbour pushed aside): it touches a little of what lies
        // around, yet it is no skin at all.
        let mut uneven = six;
        uneven[0] = [1.0, 0.6, 0.0];
        let (inside, touching) = read(&uneven);
        assert_eq!(inside, 0.0);
        assert!(touching > 0.05 && touching < 0.15, "{touching}");
        // Neighbours filling one half of the space: skin. All of them close to one direction
        // (the tip of a column): more so.
        let half = [
            [-1.0, 0.0, 0.0],
            [-1.0, 1.0, 0.0],
            [-1.0, -1.0, 0.0],
            [-1.0, 0.0, 1.0],
            [-1.0, 0.0, -1.0],
            [0.0, 1.0, 0.0],
            [0.0, -1.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, -1.0],
        ];
        let (flat, _) = read(&half);
        let (tip, _) = read(&half[..5]);
        assert!(flat > 0.2 && tip > 2.0 * flat && tip < 1.0, "{flat} {tip}");
    }

    #[test]
    fn a_mature_cell_does_not_divide_and_daughters_inherit_youth() {
        let p = Params::default();
        let mut cells = Cells::seed(&p, 2, 1.0);
        cells.youth[0] = 0.2;
        cells.pr[0][3] = p.r_div;
        divide(&mut cells, &p, 0);
        assert_eq!(cells.count, 1);
        cells.youth[0] = 0.8;
        divide(&mut cells, &p, 1);
        assert_eq!(cells.count, 2);
        assert_eq!(cells.youth[1], 0.8);
    }

    #[test]
    fn a_cell_under_another_gets_less_light() {
        let p = Params {
            ck_light: 0.01,
            shade: 1.0,
            diffusion: 0.0,
            ck_diffusion: 0.0,
            ..Params::default()
        };
        let h = p.surface + 5.0;
        let (top, under, beside) = (
            cell([0.0, h + 2.0, 0.0], 1.0, [0.0; 2]),
            cell([0.0, h, 0.0], 1.0, [0.0; 2]),
            cell([2.0, h, 0.0], 1.0, [0.0; 2]),
        );
        let made = |me: &Cell, other: &Cell| {
            let mut g = Gathered::default();
            interact(me, other, &p, &mut g);
            finish(me, &g, &p).h[1]
        };
        // The one on top is in full light, the one under it is dimmed by e, a neighbour at the
        // same height casts no shadow.
        assert!((made(&top, &under) - 0.01).abs() < 1e-6);
        assert!((made(&under, &top) - 0.01 * (-1.0f32).exp()).abs() < 1e-6);
        assert!((made(&under, &beside) - 0.01).abs() < 1e-6);
    }

    #[test]
    fn a_lit_cell_makes_cytokinin() {
        let p = Params {
            ck_light: 0.01,
            ..Params::default()
        };
        let made =
            |height: f32| finish(&cell([0.0, height, 0.0], 1.0, [0.0; 2]), &Gathered::default(), &p).h[1];
        assert!((made(p.surface + 3.0) - 0.01).abs() < 1e-6);
        assert!(made(p.surface - 10.0) < 1e-6);
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
        let p = Params {
            ck_block: 0.1,
            ..Params::default()
        };
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
    fn a_squeezed_cell_grows_less_and_stops_when_pressed_hard_enough() {
        let p = Params {
            squeeze_hold: 2.0,
            ..Params::default()
        };
        // A neighbour pressing in from the side, `depth` deep.
        let grown = |depth: f32| {
            let me = cell([0.0; 3], 0.9, [50.0, 0.0]);
            let mut g = Gathered::default();
            interact(&me, &cell([1.8 - depth, 0.0, 0.0], 0.9, [50.0, 0.0]), &p, &mut g);
            finish(&me, &g, &p).pr[3] - 0.9
        };
        // A neighbour that only touches, or pulls from a gap, holds nothing back.
        assert_eq!(grown(0.0), grown(-0.1));
        assert!(grown(0.0) > 0.0);
        assert!(grown(0.2) < grown(0.0) && grown(0.2) > 0.0);
        // Growth stops once the overlap reaches 1 / squeeze_hold of the radius.
        assert_eq!(grown(0.45), 0.0);
        assert_eq!(grown(0.6), 0.0);
        // A cell that does not sense the pressure grows whatever presses on it.
        let numb = Params {
            squeeze_hold: 0.0,
            ..Params::default()
        };
        let me = cell([0.0; 3], 0.9, [50.0, 0.0]);
        let mut g = Gathered::default();
        interact(&me, &cell([1.2, 0.0, 0.0], 0.9, [50.0, 0.0]), &numb, &mut g);
        assert!(finish(&me, &g, &numb).pr[3] > 0.9);
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
