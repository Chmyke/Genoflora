//! Prototype, judge only: cells that keep their neighbours. Plant cells are glued by the wall they
//! share and never slide past each other; two cells are neighbours because one division made
//! them so, not because they happen to touch. A bond is born at a division (between the two
//! daughters) and is never remade: when a bonded cell divides, each of its bonds goes to the
//! daughter that lies against that neighbour (to both when both do).

use crate::cells::{Cell, Cells};
use crate::shape::extent;

/// Bonds a cell can hold. A cell packed among equals touches about fourteen.
pub const SLOTS: usize = 24;
const NONE: u32 = u32::MAX;

#[derive(Clone, Debug)]
pub struct Bonds {
    with: Vec<[u32; SLOTS]>,
    /// Bonds lost because a cell had no free slot: the measure is only honest while this is small.
    pub dropped: usize,
    /// How close a neighbour must lie to a daughter to stay bonded to it, as a factor of the sum
    /// of their extents (spheres leave gaps where real cells fill their mother's place exactly).
    /// Zero: by the cut instead, a neighbour stays bonded to both daughters when the new wall
    /// crosses the patch of the mother's wall it took.
    touch: f32,
}

impl Bonds {
    /// No bond yet, for `capacity` cells.
    pub fn new(capacity: usize, touch: f32) -> Self {
        Self {
            with: vec![[NONE; SLOTS]; capacity],
            dropped: 0,
            touch,
        }
    }

    /// The cells bonded to cell `i`.
    pub fn of(&self, i: usize) -> impl Iterator<Item = usize> + '_ {
        self.with[i].iter().filter(|&&k| k != NONE).map(|&k| k as usize)
    }

    fn free(&self, i: usize) -> Option<usize> {
        self.with[i].iter().position(|&k| k == NONE)
    }

    /// Bonds `a` and `b`, both ways or not at all (false when either has no free slot).
    fn tie(&mut self, a: usize, b: usize) -> bool {
        if self.with[a].contains(&(b as u32)) {
            return true;
        }
        match (self.free(a), self.free(b)) {
            (Some(sa), Some(sb)) => {
                self.with[a][sa] = b as u32;
                self.with[b][sb] = a as u32;
                true
            }
            _ => false,
        }
    }

    fn untie(&mut self, a: usize, b: usize) {
        for (x, y) in [(a, b), (b, a)] {
            for slot in self.with[x].iter_mut().filter(|k| **k == y as u32) {
                *slot = NONE;
            }
        }
    }

    /// Hands the bonds on after a division round that turned `before` into `after`. A cell that
    /// divided is one whose record changed; the daughter of the n-th one sits at `count + n`
    /// (the order `rules::divide` uses).
    pub fn inherit(&mut self, before: &Cells, after: &Cells) {
        let mut daughter = before.count;
        for i in 0..before.count {
            if after.pr[i] == before.pr[i] {
                continue;
            }
            let j = daughter;
            daughter += 1;
            let old: Vec<usize> = self.of(i).collect();
            for &k in &old {
                self.untie(i, k);
            }
            if !self.tie(i, j) {
                self.dropped += 1;
            }
            // The cut: its middle, the direction across it, and how wide a share of the mother's
            // wall each neighbour took if they shared it evenly (the sine of the angle its patch
            // spans): the new wall lands on the neighbours whose patch it crosses.
            let (a, b) = (after.pr[i], after.pr[j]);
            let across = [0, 1, 2].map(|x| b[x] - a[x]);
            let n = old.len() as f32;
            let patch = if n > 1.0 { 2.0 * (n - 1.0).sqrt() / n } else { 0.0 };
            for k in old {
                let (to_i, to_j) = if self.touch > 0.0 {
                    // How far apart the walls of the neighbour and of each daughter are, and
                    // whether they are close enough to count as touching.
                    let (gi, ti) = gap(&after.get(i), &after.get(k), self.touch);
                    let (gj, tj) = gap(&after.get(j), &after.get(k), self.touch);
                    (ti || (!tj && gi <= gj), tj || (!ti && gj < gi))
                } else {
                    let q = after.pr[k];
                    let out = [0, 1, 2].map(|x| q[x] - 0.5 * (a[x] + b[x]));
                    let len = |v: &[f32; 3]| v.iter().map(|c| c * c).sum::<f32>().sqrt().max(1e-6);
                    let side = (0..3).map(|x| out[x] * across[x]).sum::<f32>() / (len(&out) * len(&across));
                    (side < patch, side > -patch)
                };
                for (c, wanted) in [(i, to_i), (j, to_j)] {
                    if wanted && !self.tie(c, k) {
                        self.dropped += 1;
                    }
                }
            }
        }
    }

    /// Mean number of bonds of the first `count` cells.
    pub fn mean(&self, count: usize) -> f32 {
        (0..count).map(|i| self.of(i).count()).sum::<usize>() as f32 / count.max(1) as f32
    }
}

/// Distance between the walls of two cells along the line that joins them (negative when they
/// overlap), and whether they lie within `touch` times the sum of their extents.
fn gap(a: &Cell, b: &Cell, touch: f32) -> (f32, bool) {
    let dv = [0, 1, 2].map(|k| a.pr[k] - b.pr[k]);
    let d = dv.iter().map(|v| v * v).sum::<f32>().sqrt().max(1e-6);
    let u = dv.map(|v| v / d);
    let rs = extent(&a.shape, &u) + extent(&b.shape, &u);
    (d - rs, d < touch * rs)
}

/// Displacement of `me` from its bond with `other`: none while they touch, then a pull that grows
/// with the gap between their walls and never lets go (zero at contact, so no jump).
pub fn pull(me: &Cell, other: &Cell, strength: f32) -> [f32; 3] {
    let dv = [0, 1, 2].map(|k| me.pr[k] - other.pr[k]);
    let d = dv.iter().map(|v| v * v).sum::<f32>().sqrt().max(1e-6);
    let u = dv.map(|v| v / d);
    let apart = (d - extent(&me.shape, &u) - extent(&other.shape, &u)).max(0.0);
    u.map(|v| -strength * apart * v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shape::sphere;

    fn ball(at: [f32; 3], r: f32) -> Cell {
        Cell {
            pr: [at[0], at[1], at[2], r],
            shape: sphere(r, [1.0, 0.0, 0.0]),
            h: [0.0; 4],
            pin: [0.0; 4],
            youth: 1.0,
        }
    }

    #[test]
    fn a_bond_pulls_only_across_a_gap_and_harder_the_wider_it_is() {
        let at = |d: f32| pull(&ball([d, 0.0, 0.0], 1.0), &ball([0.0; 3], 1.0), 0.1)[0];
        assert_eq!(at(1.5), 0.0);
        assert!(at(2.0 + 1e-4).abs() < 1e-4);
        assert!(at(3.0) < 0.0 && at(6.0) < at(3.0));
    }

    #[test]
    fn a_dividing_cell_hands_each_neighbour_to_the_daughter_lying_against_it() {
        // Cell 0 between a neighbour on its left (1), one on its right (2) and one above (3).
        let mut before = Cells::empty(8);
        before.count = 4;
        before.set(0, ball([0.0; 3], 1.0));
        before.set(1, ball([-2.0, 0.0, 0.0], 1.0));
        before.set(2, ball([2.0, 0.0, 0.0], 1.0));
        before.set(3, ball([0.0, 1.6, 0.0], 1.0));
        let mut bonds = Bonds::new(8, 1.2);
        for k in 1..4 {
            assert!(bonds.tie(0, k));
        }
        // It divides across x: daughters at -0.5 (index 0) and +0.5 (index 4).
        let mut after = before.clone();
        after.count = 5;
        after.set(0, ball([-0.5, 0.0, 0.0], 0.8));
        after.set(4, ball([0.5, 0.0, 0.0], 0.8));
        bonds.inherit(&before, &after);
        let of = |i: usize| {
            let mut v: Vec<usize> = bonds.of(i).collect();
            v.sort_unstable();
            v
        };
        assert_eq!(of(0), [1, 3, 4]);
        assert_eq!(of(4), [0, 2, 3]);
        assert_eq!((of(1), of(2), of(3)), (vec![0], vec![4], vec![0, 4]));
        assert_eq!(bonds.dropped, 0);
    }
}
