//! Prototype, judge only: cells that keep their neighbours. Plant cells are glued by the wall they
//! share and never slide past each other; two cells are neighbours because one division made
//! them so, not because they happen to touch. A bond is born at a division (between the two
//! daughters) and is never remade: when a bonded cell divides, each of its bonds goes to the
//! daughter that lies against that neighbour (to both when both do).
//!
//! A bond is a wall, not a rubber band: it remembers how far apart the two cells were when it was
//! laid (its rest gap), pulls only when they are drawn further apart than that, and, stretched
//! beyond what it holds, gives way for good (a wall grows by yielding under tension, Lockhart
//! 1965). Measured 2026-11-26: bonds that pulled their cells back to contact crushed the tissue.

use crate::cells::{Cell, Cells};
use crate::shape::{dir, extent};

/// Bonds a cell can hold. A cell packed among equals touches about fourteen.
pub const SLOTS: usize = 24;
const NONE: u32 = u32::MAX;

/// How the walls between bonded cells behave.
#[derive(Clone, Copy, Debug)]
pub struct Glue {
    /// How hard a stretched wall pulls its two cells together, per unit of stretch.
    pub strength: f32,
    /// How close a neighbour must lie to a daughter to stay bonded to it, as a factor of the sum
    /// of their extents (spheres leave gaps where real cells fill their mother's place exactly).
    /// Zero: by the cut instead, a neighbour stays bonded to both daughters when the new wall
    /// crosses the patch of the mother's wall it took.
    pub touch: f32,
    /// Stretch a wall holds without yielding, as a share of the sum of the two cells' extents.
    pub hold: f32,
    /// Share of the stretch beyond that which the wall gives up for good at every step.
    pub creep: f32,
    /// How much of that a wall keeps when it is pulled across the growth direction of its two
    /// cells, from 1 (it yields the same every way) to 0 (it yields only along them: the fibres of
    /// a wall lie across the direction it lets the cell grow in, and hold against widening).
    pub across: f32,
}

#[derive(Clone, Debug)]
pub struct Bonds {
    with: Vec<[u32; SLOTS]>,
    /// Rest gap of every bond: the distance between the two walls at which it does not pull.
    rest: Vec<[f32; SLOTS]>,
    /// Bonds lost because a cell had no free slot: the measure is only honest while this is small.
    pub dropped: usize,
    glue: Glue,
}

impl Bonds {
    /// No bond yet, for `capacity` cells.
    pub fn new(capacity: usize, glue: Glue) -> Self {
        Self {
            with: vec![[NONE; SLOTS]; capacity],
            rest: vec![[0.0; SLOTS]; capacity],
            dropped: 0,
            glue,
        }
    }

    /// The cells bonded to cell `i`.
    pub fn of(&self, i: usize) -> impl Iterator<Item = usize> + '_ {
        self.with[i].iter().filter(|&&k| k != NONE).map(|&k| k as usize)
    }

    /// The cells bonded to cell `i`, each with the rest gap of the bond.
    pub fn walls(&self, i: usize) -> impl Iterator<Item = (usize, f32)> + '_ {
        let pairs = self.with[i].iter().zip(&self.rest[i]);
        pairs.filter(|(k, _)| **k != NONE).map(|(&k, &r)| (k as usize, r))
    }

    /// Displacement of cell `i` from its walls: each one stretched beyond its rest gap pulls it
    /// toward the neighbour, the harder the further (zero at rest, so no jump).
    pub fn pull(&self, cells: &Cells, i: usize) -> [f32; 3] {
        let me = cells.get(i);
        let mut sum = [0.0; 3];
        for (k, rest) in self.walls(i) {
            let (gap, _, u) = gap(&me, &cells.get(k));
            let stretch = (gap - rest).max(0.0);
            for (s, v) in sum.iter_mut().zip(u) {
                *s -= self.glue.strength * stretch * v;
            }
        }
        sum
    }

    /// The walls stretched beyond what they hold give way: their rest gap grows, for good.
    pub fn creep(&mut self, cells: &Cells) {
        for i in 0..cells.count {
            for slot in 0..SLOTS {
                let k = self.with[i][slot];
                if k == NONE || (k as usize) < i {
                    continue;
                }
                let (a, b) = (cells.get(i), cells.get(k as usize));
                let (gap, rs, u) = gap(&a, &b);
                let excess = gap - self.rest[i][slot] - self.glue.hold * rs;
                if excess > 0.0 {
                    // How squarely the pull lies along the growth direction of the two cells.
                    let along = |c: &Cell| (0..3).map(|x| dir(&c.shape)[x] * u[x]).sum::<f32>().abs();
                    let way = 0.5 * (along(&a) + along(&b));
                    let give = self.glue.across + (1.0 - self.glue.across) * way;
                    let rest = self.rest[i][slot] + self.glue.creep * give * excess;
                    self.rest[i][slot] = rest;
                    if let Some(back) = self.with[k as usize].iter().position(|&x| x as usize == i) {
                        self.rest[k as usize][back] = rest;
                    }
                }
            }
        }
    }

    fn free(&self, i: usize) -> Option<usize> {
        self.with[i].iter().position(|&k| k == NONE)
    }

    /// Bonds `a` and `b` with a wall at rest at `rest`, both ways or not at all (false when
    /// either has no free slot).
    fn tie(&mut self, a: usize, b: usize, rest: f32) -> bool {
        if self.with[a].contains(&(b as u32)) {
            return true;
        }
        match (self.free(a), self.free(b)) {
            (Some(sa), Some(sb)) => {
                self.with[a][sa] = b as u32;
                self.with[b][sb] = a as u32;
                self.rest[a][sa] = rest;
                self.rest[b][sb] = rest;
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
            // The wall between the two daughters is laid where they stand.
            let (apart, _, _) = gap(&after.get(i), &after.get(j));
            if !self.tie(i, j, apart.max(0.0)) {
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
                // How far apart the walls of the neighbour and of each daughter are.
                let (gi, ri, _) = gap(&after.get(i), &after.get(k));
                let (gj, rj, _) = gap(&after.get(j), &after.get(k));
                let (to_i, to_j) = if self.glue.touch > 0.0 {
                    // Close enough to count as touching?
                    let reach = self.glue.touch - 1.0;
                    let (ti, tj) = (gi < reach * ri, gj < reach * rj);
                    (ti || (!tj && gi <= gj), tj || (!ti && gj < gi))
                } else {
                    let q = after.pr[k];
                    let out = [0, 1, 2].map(|x| q[x] - 0.5 * (a[x] + b[x]));
                    let len = |v: &[f32; 3]| v.iter().map(|c| c * c).sum::<f32>().sqrt().max(1e-6);
                    let side = (0..3).map(|x| out[x] * across[x]).sum::<f32>() / (len(&out) * len(&across));
                    (side < patch, side > -patch)
                };
                // The wall it shared with the mother is the one it now shares with the daughter:
                // at rest where they stand.
                for (c, wanted, at) in [(i, to_i, gi), (j, to_j, gj)] {
                    if wanted && !self.tie(c, k, at.max(0.0)) {
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
/// overlap), the sum of their extents along it, and the unit vector from `b` to `a`.
fn gap(a: &Cell, b: &Cell) -> (f32, f32, [f32; 3]) {
    let dv = [0, 1, 2].map(|k| a.pr[k] - b.pr[k]);
    let d = dv.iter().map(|v| v * v).sum::<f32>().sqrt().max(1e-6);
    let u = dv.map(|v| v / d);
    let rs = extent(&a.shape, &u) + extent(&b.shape, &u);
    (d - rs, rs, u)
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

    const GLUE: Glue = Glue {
        strength: 0.1,
        touch: 1.2,
        hold: 0.1,
        creep: 0.5,
        across: 1.0,
    };

    fn pair(d: f32) -> Cells {
        let mut c = Cells::empty(2);
        c.count = 2;
        c.set(0, ball([0.0; 3], 1.0));
        c.set(1, ball([d, 0.0, 0.0], 1.0));
        c
    }

    #[test]
    fn a_wall_pulls_only_when_stretched_beyond_its_rest_and_harder_the_further() {
        let mut bonds = Bonds::new(2, GLUE);
        assert!(bonds.tie(0, 1, 0.5));
        let at = |d: f32| bonds.pull(&pair(d), 1)[0];
        // The walls are 0.5 apart at d = 2.5: no pull up to there, none when pressed together.
        assert_eq!((at(1.5), at(2.4)), (0.0, 0.0));
        assert!(at(2.5 + 1e-4).abs() < 1e-4);
        assert!(at(3.0) < 0.0 && at(4.0) < at(3.0));
        assert_eq!(bonds.pull(&pair(3.0), 0)[0], -at(3.0));
    }

    #[test]
    fn a_wall_stretched_beyond_what_it_holds_gives_way_for_good() {
        let mut bonds = Bonds::new(2, GLUE);
        assert!(bonds.tie(0, 1, 0.0));
        let rest = |b: &Bonds| (b.walls(0).next().map(|w| w.1), b.walls(1).next().map(|w| w.1));
        // It holds a stretch of a tenth of the two radii (0.2): nothing yields at 0.15.
        bonds.creep(&pair(2.15));
        assert_eq!(rest(&bonds), (Some(0.0), Some(0.0)));
        // At 1.0 it gives up half of the 0.8 beyond, on both sides, and keeps it once released.
        bonds.creep(&pair(3.0));
        bonds.creep(&pair(2.0));
        let (a, b) = rest(&bonds);
        assert_eq!(a, b);
        assert!(a.is_some_and(|r| (r - 0.4).abs() < 1e-5), "{a:?}");
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
        let mut bonds = Bonds::new(8, GLUE);
        for k in 1..4 {
            assert!(bonds.tie(0, k, 0.0));
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
