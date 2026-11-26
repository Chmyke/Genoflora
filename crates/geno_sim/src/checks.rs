//! Measured checks that any engine must pass. They return numbers; the tests hold the thresholds.
//! The same functions run on the CPU judge (below) and on the GPU engine (`geno_gpu` tests).

use crate::cells::Cells;
use crate::engine::{Engine, EngineError};
use crate::hash::{hash, unit};
use crate::params::Params;
use crate::reference::Reference;

/// Test tissue: `n` cells on a noisy cubic lattice, the lattice points closest to the origin.
pub fn lattice(n: usize, capacity: usize, p: &Params) -> Cells {
    const SPACING: f32 = 1.9;
    const NOISE: f32 = 0.1;
    let mut m = 1i32;
    while ((2 * m + 1) as usize).pow(3) < n * 22 / 10 + 27 {
        m += 1;
    }
    let mut pts: Vec<(i32, [i32; 3])> = Vec::with_capacity(((2 * m + 1) as usize).pow(3));
    for z in -m..=m {
        for y in -m..=m {
            for x in -m..=m {
                pts.push((x * x + y * y + z * z, [x, y, z]));
            }
        }
    }
    pts.sort_unstable_by_key(|&(d2, [x, y, z])| (d2, z, y, x));
    let mut cells = Cells::empty(capacity.max(n));
    cells.count = n;
    for (i, &(_, q)) in pts.iter().take(n).enumerate() {
        let id = i as u32;
        let jitter = |c: u32| (unit(hash(p.seed, id, c)) * 2.0 - 1.0) * NOISE;
        let a = crate::hash::axis(p.seed, id, 7);
        cells.pr[i] = [
            q[0] as f32 * SPACING + jitter(0),
            q[1] as f32 * SPACING + jitter(1),
            q[2] as f32 * SPACING + jitter(2),
            p.r_div * (0.8 + 0.2 * unit(hash(p.seed, id, 3))),
        ];
        cells.shape[i] = crate::shape::sphere(cells.pr[i][3], a);
        cells.h[i] = [4, 5, 6, 8].map(|salt| unit(hash(p.seed, id, salt)));
    }
    cells
}

/// Runs `steps` steps, with a division round every `divide_every` steps when given.
pub fn run(e: &mut dyn Engine, steps: u32, divide_every: Option<u32>) {
    for s in 1..=steps {
        e.step();
        if divide_every.is_some_and(|k| k > 0 && s % k == 0) {
            e.divide();
        }
    }
}

/// What cell `i` gathers from all the others. Every pair is tested, like the judge.
fn gathered(c: &Cells, p: &Params, i: usize) -> crate::rules::Gathered {
    let me = c.get(i);
    let mut g = crate::rules::Gathered::default();
    for j in (0..c.count).filter(|&j| j != i) {
        crate::rules::interact(&me, &c.get(j), p, &mut g);
    }
    g
}

/// Light each cell receives: what the soil lets through, dimmed by the cells lying above it.
pub fn light_on(c: &Cells, p: &Params) -> Vec<f32> {
    (0..c.count)
        .map(|i| crate::soil::shaded(p, crate::soil::light(p, &c.pr[i]), gathered(c, p, i).cover))
        .collect()
}

/// How packed a tissue is, and whether its hormones are still sound.
#[derive(Clone, Copy, Debug)]
pub struct Packing {
    /// Most neighbours any cell has. Cells that hold their place have a dozen; many more means
    /// they are piled into each other.
    pub neighbours: f32,
    /// Lowest hormone amount of any cell. Below zero, the diffusion has overshot: it closes
    /// `diffusion` of the gap with each neighbour, so it needs `diffusion * neighbours` under one.
    pub lowest: f32,
}

/// Measures the packing of the tissue.
pub fn packing(c: &Cells, p: &Params) -> Packing {
    Packing {
        neighbours: (0..c.count)
            .map(|i| gathered(c, p, i).neighbours)
            .fold(0.0, f32::max),
        lowest: c
            .hormone()
            .iter()
            .flatten()
            .copied()
            .fold(f32::INFINITY, f32::min),
    }
}

/// A group of young cells in contact: a place where the tissue still divides.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Focus {
    pub cells: usize,
    /// Mean position of its cells.
    pub centre: [f32; 3],
}

/// The foci of the tissue among the cells that `keep` retains, the biggest first. Two young cells
/// belong to the same focus when they are neighbours, or linked by a chain of young neighbours.
pub fn foci(c: &Cells, p: &Params, keep: impl Fn(&[f32; 4]) -> bool) -> Vec<Focus> {
    let young: Vec<usize> = (0..c.count)
        .filter(|&i| c.youth[i] > crate::params::YOUNG && keep(&c.pr[i]))
        .collect();
    // Union-find over the young cells: `group[a]` leads to the representative of `a`.
    let mut group: Vec<usize> = (0..young.len()).collect();
    fn find(group: &mut [usize], mut a: usize) -> usize {
        while group[a] != a {
            group[a] = group[group[a]];
            a = group[a];
        }
        a
    }
    for (a, &first) in young.iter().enumerate() {
        let me = c.get(first);
        for (b, &second) in young.iter().enumerate().skip(a + 1) {
            let mut g = crate::rules::Gathered::default();
            crate::rules::interact(&me, &c.get(second), p, &mut g);
            if g.neighbours > 0.0 {
                let (ra, rb) = (find(&mut group, a), find(&mut group, b));
                group[ra] = rb;
            }
        }
    }
    let mut sums = vec![(0usize, [0.0f64; 3]); young.len()];
    for (a, &cell) in young.iter().enumerate() {
        let root = find(&mut group, a);
        sums[root].0 += 1;
        for k in 0..3 {
            sums[root].1[k] += f64::from(c.pr[cell][k]);
        }
    }
    let mut out: Vec<Focus> = sums
        .iter()
        .filter(|s| s.0 > 0)
        .map(|s| Focus {
            cells: s.0,
            centre: s.1.map(|v| (v / s.0 as f64) as f32),
        })
        .collect();
    out.sort_by(|a, b| b.cells.cmp(&a.cells).then(a.centre[0].total_cmp(&b.centre[0])));
    out
}

/// The lit cells at one distance from a point (the centre of a focus, usually).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Ring {
    pub cells: usize,
    /// Share of them that are young.
    pub young: f32,
    /// Mean concentrations.
    pub auxin: f32,
    pub cytokinin: f32,
    pub restraint: f32,
    pub relay: f32,
}

/// The lit cells around `centre`, in `rings` rings one dividing radius wide. Only the cells that
/// received at least half of the full light count: the skin of the tissue, where a tip lives.
pub fn profile(c: &Cells, centre: [f32; 3], rings: usize) -> Vec<Ring> {
    let mut out = vec![Ring::default(); rings];
    for i in (0..c.count).filter(|&i| c.pin[i][3] >= 0.5) {
        let q = c.pr[i];
        let d2: f32 = (0..3).map(|k| (q[k] - centre[k]).powi(2)).sum();
        let Some(ring) = out.get_mut(d2.sqrt() as usize) else {
            continue;
        };
        let volume = q[3] * q[3] * q[3];
        ring.cells += 1;
        ring.young += f32::from(u8::from(c.youth[i] > crate::params::YOUNG));
        ring.auxin += c.h[i][0] / volume;
        ring.cytokinin += c.h[i][1] / volume;
        ring.restraint += c.h[i][2] / volume;
        ring.relay += c.h[i][3] / volume;
    }
    for ring in &mut out {
        let n = ring.cells.max(1) as f32;
        ring.young /= n;
        ring.auxin /= n;
        ring.cytokinin /= n;
        ring.restraint /= n;
        ring.relay /= n;
    }
    out
}

/// The cells of one layer of a cut through the tissue: means over them.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Layer {
    pub cells: usize,
    pub youth: f32,
    /// Light received at the last step.
    pub light: f32,
    /// How much they belong to the skin (`rules::skin`).
    pub skin: f32,
    /// Concentrations.
    pub auxin: f32,
    pub cytokinin: f32,
    pub restraint: f32,
    pub relay: f32,
    /// Share of the fullest relay making they are at: where the relay is made, as opposed to
    /// where it is found.
    pub making: f32,
}

/// Means over the cells that `layer_of` places in one of `layers` layers.
fn cut(c: &Cells, p: &Params, layers: usize, layer_of: impl Fn(&[f32; 4]) -> Option<usize>) -> Vec<Layer> {
    let mut out = vec![Layer::default(); layers];
    for i in 0..c.count {
        let q = c.pr[i];
        let Some(layer) = layer_of(&q).and_then(|k| out.get_mut(k)) else {
            continue;
        };
        let volume = q[3] * q[3] * q[3];
        let [auxin, cytokinin, restraint, relay] = c.h[i].map(|h| h / volume);
        layer.cells += 1;
        layer.youth += c.youth[i];
        layer.light += c.pin[i][3];
        layer.skin += crate::rules::skin(&gathered(c, p, i));
        layer.auxin += auxin;
        layer.cytokinin += cytokinin;
        layer.restraint += restraint;
        layer.relay += relay;
        layer.making += crate::rules::ck_call(p, auxin, cytokinin) * crate::rules::heard(p, restraint);
    }
    for layer in &mut out {
        let n = layer.cells.max(1) as f32;
        for sum in [
            &mut layer.youth,
            &mut layer.light,
            &mut layer.skin,
            &mut layer.auxin,
            &mut layer.cytokinin,
            &mut layer.restraint,
            &mut layer.relay,
            &mut layer.making,
        ] {
            *sum /= n;
        }
    }
    out
}

/// The tissue straight under `top`, layer by layer going down (along gravity), one dividing
/// radius per layer: the cells within `reach` of the vertical line through `top`.
pub fn under(c: &Cells, p: &Params, top: [f32; 3], reach: f32, layers: usize) -> Vec<Layer> {
    cut(c, p, layers, |q| {
        let d = [q[0] - top[0], q[1] - top[1], q[2] - top[2]];
        let depth: f32 = (0..3).map(|k| d[k] * p.gravity[k]).sum();
        let aside2: f32 = (0..3).map(|k| (d[k] - depth * p.gravity[k]).powi(2)).sum();
        (depth >= 0.0 && aside2 <= reach * reach).then_some((depth / p.r_div) as usize)
    })
}

/// The tissue across the plant at `depth` under `top`, from its own axis outward, one dividing
/// radius per layer: the cells of a slab `thick` thick, sorted by their distance to the centre of
/// the slab. The outermost filled layer is the skin of the flank.
pub fn across(c: &Cells, p: &Params, top: [f32; 3], depth: f32, thick: f32, layers: usize) -> Vec<Layer> {
    let below = |q: &[f32; 4]| -> f32 { (0..3).map(|k| (q[k] - top[k]) * p.gravity[k]).sum() };
    let in_slab = |q: &[f32; 4]| (below(q) - depth).abs() <= 0.5 * thick;
    let slab: Vec<&[f32; 4]> = c.alive().iter().filter(|q| in_slab(q)).collect();
    let n = slab.len().max(1) as f32;
    let centre = [0, 1, 2].map(|k| slab.iter().map(|q| q[k]).sum::<f32>() / n);
    cut(c, p, layers, |q| {
        let d = [q[0] - centre[0], q[1] - centre[1], q[2] - centre[2]];
        let along: f32 = (0..3).map(|k| d[k] * p.gravity[k]).sum();
        let aside2: f32 = (0..3).map(|k| (d[k] - along * p.gravity[k]).powi(2)).sum();
        in_slab(q).then_some((aside2.sqrt() / p.r_div) as usize)
    })
}

/// Total hormone, all kinds together.
pub fn mass(c: &Cells) -> f64 {
    c.hormone().iter().flatten().map(|&h| f64::from(h)).sum()
}

/// Total cell volume, up to the constant 4/3 pi.
pub fn volume(c: &Cells) -> f64 {
    c.alive().iter().map(|q| f64::from(q[3]).powi(3)).sum()
}

fn mean_radius(c: &Cells) -> f64 {
    c.alive().iter().map(|q| f64::from(q[3])).sum::<f64>() / c.count.max(1) as f64
}

/// Radius of gyration of the tissue: a shape measure that does not depend on which cell is where.
pub fn gyration(c: &Cells) -> f64 {
    let n = c.count.max(1) as f64;
    let centre = [0, 1, 2].map(|k| c.alive().iter().map(|q| f64::from(q[k])).sum::<f64>() / n);
    let sum: f64 = c
        .alive()
        .iter()
        .map(|q| (0..3).map(|k| (f64::from(q[k]) - centre[k]).powi(2)).sum::<f64>())
        .sum();
    (sum / n).sqrt()
}

/// Largest overlap between two cells, as a fraction of the smaller radius.
pub fn worst_overlap(c: &Cells) -> f64 {
    let mut worst = 0.0f64;
    for (i, a) in c.alive().iter().enumerate() {
        for b in &c.alive()[i + 1..] {
            let d2: f32 = (0..3).map(|k| (a[k] - b[k]).powi(2)).sum();
            let overlap = (a[3] + b[3] - d2.sqrt()) / a[3].min(b[3]);
            worst = worst.max(f64::from(overlap));
        }
    }
    worst
}

/// How far an engine ends from the judge after the same run.
#[derive(Clone, Copy, Debug)]
pub struct Gap {
    /// Cell counts: (engine, judge).
    pub counts: (usize, usize),
    /// Largest cell-to-cell position gap, in mean radii. Only meaningful on a short horizon:
    /// a dividing tissue amplifies rounding differences exponentially.
    pub position: f64,
    /// Relative gap of the radius of gyration (shape), valid on any horizon.
    pub shape: f64,
    /// Relative gap of the total hormone.
    pub mass: f64,
}

/// Same run (with divisions) on the judge and on `engine`, both started from `start`.
pub fn gap_to_judge(
    engine: &mut dyn Engine,
    p: &Params,
    start: &Cells,
    steps: u32,
) -> Result<Gap, EngineError> {
    let mut judge = Reference::new(p, start);
    run(&mut judge, steps, Some(p.divide_every));
    let want = judge.snapshot()?;
    run(engine, steps, Some(p.divide_every));
    let got = engine.snapshot()?;
    let mut position = 0.0f64;
    for (a, b) in want.alive().iter().zip(got.alive()) {
        let d2: f64 = (0..3).map(|k| f64::from(a[k] - b[k]).powi(2)).sum();
        position = position.max(d2.sqrt());
    }
    Ok(Gap {
        counts: (got.count, want.count),
        position: position / mean_radius(&want),
        shape: ((gyration(&want) - gyration(&got)) / gyration(&want)).abs(),
        mass: ((mass(&want) - mass(&got)) / mass(&want)).abs(),
    })
}

/// Relative hormone drift over `steps` steps without division. With no production and no decay,
/// diffusion alone must conserve the hormone.
pub fn mass_drift(engine: &mut dyn Engine, steps: u32) -> Result<f64, EngineError> {
    let before = mass(&engine.snapshot()?);
    run(engine, steps, None);
    Ok(((mass(&engine.snapshot()?) - before) / before).abs())
}

/// What one division round changes.
#[derive(Clone, Copy, Debug)]
pub struct Division {
    pub cells_before: usize,
    pub cells_after: usize,
    /// Relative volume gap across the round.
    pub volume: f64,
    /// Relative hormone gap across the round.
    pub mass: f64,
}

/// Grows the cells for `steps` steps, then measures one division round.
pub fn division(engine: &mut dyn Engine, steps: u32) -> Result<Division, EngineError> {
    run(engine, steps, None);
    let before = engine.snapshot()?;
    engine.divide();
    let after = engine.snapshot()?;
    Ok(Division {
        cells_before: before.count,
        cells_after: after.count,
        volume: ((volume(&after) - volume(&before)) / volume(&before)).abs(),
        mass: ((mass(&after) - mass(&before)) / mass(&before)).abs(),
    })
}

/// How a tissue lies along a direction (a unit vector, usually gravity).
#[derive(Clone, Copy, Debug)]
pub struct Lie {
    /// Spread of the cells along the direction divided by their spread across it:
    /// 1 for a ball, more for a column.
    pub elongation: f64,
    /// Mean auxin concentration of the tenth of the cells furthest along the direction...
    pub front: f64,
    /// ...and of the tenth furthest against it.
    pub back: f64,
    /// Same two means for cytokinin: (front, back).
    pub cytokinin: (f64, f64),
    /// Mean of |long axis . direction| over the cells: 0.5 for random axes, 1 if all line up.
    /// Round cells (length below 1.05 times the width) count as random.
    pub alignment: f64,
    /// Mean length over width of the cells (1 for spheres).
    pub stretch: f64,
    /// Furthest reach of the tissue along the direction and against it, from the origin
    /// (where a seed starts).
    pub reach: (f64, f64),
}

/// Length over width of a cell.
pub fn length_over_width(s: &crate::shape::Shape) -> f64 {
    let (low, high) = crate::shape::eigen_extremes(s);
    (f64::from(high) / f64::from(low).max(1e-12)).sqrt()
}

/// Unit long axis of a cell, or `None` for a round cell.
pub fn long_axis(s: &crate::shape::Shape) -> Option<[f64; 3]> {
    if length_over_width(s) < 1.05 {
        return None;
    }
    let low = f64::from(crate::shape::eigen_extremes(s).0);
    let m = [
        [f64::from(s[0]) - low, f64::from(s[3]), f64::from(s[4])],
        [f64::from(s[3]), f64::from(s[1]) - low, f64::from(s[5])],
        [f64::from(s[4]), f64::from(s[5]), f64::from(s[2]) - low],
    ];
    // The eigenvector is orthogonal to every row of N - low I: take the best cross product.
    let cross = |a: &[f64; 3], b: &[f64; 3]| {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    };
    let v = [cross(&m[0], &m[1]), cross(&m[0], &m[2]), cross(&m[1], &m[2])]
        .into_iter()
        .max_by(|a, b| {
            a.iter()
                .map(|x| x * x)
                .sum::<f64>()
                .total_cmp(&b.iter().map(|x| x * x).sum::<f64>())
        })?;
    let len = v.iter().map(|x| x * x).sum::<f64>().sqrt();
    (len > 0.0).then(|| v.map(|x| x / len))
}

/// Measures the shape and the hormone of the tissue along `axis`. Needs at least ten cells.
pub fn lie(c: &Cells, axis: [f32; 3]) -> Lie {
    let n = c.count.max(1) as f64;
    let axis = axis.map(f64::from);
    let centre = [0, 1, 2].map(|k| c.alive().iter().map(|q| f64::from(q[k])).sum::<f64>() / n);
    let (mut along2, mut all2) = (0.0f64, 0.0f64);
    let mut heights: Vec<(f64, f64, f64)> = Vec::with_capacity(c.count);
    for (q, h) in c.alive().iter().zip(c.hormone()) {
        let d = [0, 1, 2].map(|k| f64::from(q[k]) - centre[k]);
        let a = d[0] * axis[0] + d[1] * axis[1] + d[2] * axis[2];
        along2 += a * a;
        all2 += d[0] * d[0] + d[1] * d[1] + d[2] * d[2];
        let from_origin = (0..3).map(|k| f64::from(q[k]) * axis[k]).sum::<f64>();
        let volume = f64::from(q[3]).powi(3);
        heights.push((from_origin, f64::from(h[0]) / volume, f64::from(h[1]) / volume));
    }
    heights.sort_by(|a, b| a.0.total_cmp(&b.0));
    let tenth = (c.count / 10).max(1).min(heights.len());
    let mean = |part: &[(f64, f64, f64)], ck: bool| {
        part.iter().map(|v| if ck { v.2 } else { v.1 }).sum::<f64>() / part.len().max(1) as f64
    };
    let (front, back) = (&heights[heights.len() - tenth..], &heights[..tenth]);
    Lie {
        elongation: (along2 / ((all2 - along2) / 2.0).max(1e-12)).sqrt(),
        front: mean(front, false),
        back: mean(back, false),
        cytokinin: (mean(front, true), mean(back, true)),
        alignment: c
            .shapes()
            .iter()
            .map(|s| match long_axis(s) {
                Some(l) => (0..3).map(|k| l[k] * axis[k]).sum::<f64>().abs(),
                None => 0.5,
            })
            .sum::<f64>()
            / n,
        stretch: c.shapes().iter().map(length_over_width).sum::<f64>() / n,
        reach: (
            heights.last().map_or(0.0, |v| v.0),
            -heights.first().map_or(0.0, |v| v.0),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn judge(p: &Params, n: usize, capacity: usize) -> Reference {
        Reference::new(p, &lattice(n, capacity, p))
    }

    #[test]
    fn light_on_a_tissue_is_full_on_top_and_dim_underneath() {
        let p = Params::default();
        let mut cells = Cells::empty(2);
        cells.count = 2;
        let h = p.surface + 5.0;
        cells.pr[0] = [0.0, h + 1.9, 0.0, 1.0];
        cells.pr[1] = [0.0, h, 0.0, 1.0];
        let light = light_on(&cells, &p);
        assert_eq!(light[0], 1.0);
        assert!((light[1] - (-p.shade).exp()).abs() < 1e-6, "{light:?}");
    }

    /// A column of five cells going down from `h`, the n-th holding `n` of relay and a youth of
    /// `n / 10`, and one more cell far aside at the height of the third.
    fn column(h: f32) -> Cells {
        let mut cells = Cells::empty(6);
        cells.count = 6;
        for n in 0..5 {
            cells.pr[n] = [0.0, h - 1.5 * n as f32, 0.0, 1.0];
            cells.h[n][crate::cells::RELAY] = n as f32;
            cells.youth[n] = n as f32 / 10.0;
        }
        cells.pr[5] = [4.5, h - 3.0, 0.0, 1.0];
        cells.h[5][crate::cells::RELAY] = 9.0;
        cells
    }

    #[test]
    fn the_cut_under_a_top_reads_each_depth_and_leaves_out_what_lies_aside() {
        let p = Params::default();
        let cells = column(p.surface + 20.0);
        let layers = under(&cells, &p, [0.0, p.surface + 20.0, 0.0], 2.0, 8);
        // Cells every 1.5 down: depths 0, 1.5, 3, 4.5, 6 fall in layers 0, 1, 3, 4, 6.
        let found: Vec<usize> = layers.iter().map(|l| l.cells).collect();
        assert_eq!(found, [1, 1, 0, 1, 1, 0, 1, 0]);
        assert_eq!(layers[3].relay, 2.0);
        assert!((layers[4].youth - 0.3).abs() < 1e-6);
        // The top of the column has a neighbour on one side only: all skin. The middle has two
        // opposite ones: inside.
        assert_eq!(layers[0].skin, 1.0);
        assert!(layers[3].skin < 1e-6, "{}", layers[3].skin);
    }

    #[test]
    fn the_cut_across_sorts_a_slab_from_its_centre_outward() {
        let p = Params::default();
        let cells = column(p.surface + 20.0);
        let top = [0.0, p.surface + 20.0, 0.0];
        // The slab at depth 3 holds the third cell of the column and the one aside, 4.5 apart:
        // each lies 2.25 from their centre.
        let layers = across(&cells, &p, top, 3.0, 1.0, 4);
        let found: Vec<usize> = layers.iter().map(|l| l.cells).collect();
        assert_eq!(found, [0, 0, 2, 0]);
        assert_eq!(layers[2].relay, 5.5);
    }

    #[test]
    fn the_making_of_relay_is_read_from_the_rules() {
        let p = Params::default();
        let mut cells = column(p.surface + 20.0);
        cells.h[0][crate::cells::CYTOKININ] = p.ck_half;
        cells.h[0][crate::cells::RESTRAINT] = p.restraint_half;
        let top = [0.0, p.surface + 20.0, 0.0];
        let layers = under(&cells, &p, top, 2.0, 2);
        // Half the cytokinin call, half heard through the restraint signal, no auxin.
        assert_eq!(layers[0].making, 0.25);
        assert_eq!(layers[1].making, 0.0);
    }

    #[test]
    fn diffusion_conserves_the_hormone() -> Result<(), EngineError> {
        let p = Params {
            production: 0.0,
            light_production: 0.0,
            decay: 0.0,
            ck_production: 0.0,
            ck_light: 0.0,
            ck_decay: 0.0,
            restraint_made: 0.0,
            restraint_decay: 0.0,
            relay_made: 0.0,
            relay_decay: 0.0,
            ..Params::default()
        };
        let drift = mass_drift(&mut judge(&p, 300, 300), 500)?;
        assert!(drift <= 1e-5, "hormone drift {drift:.2e}");
        Ok(())
    }

    #[test]
    fn division_conserves_volume_and_hormone() -> Result<(), EngineError> {
        let p = Params::default();
        let d = division(&mut judge(&p, 300, 1024), 20)?;
        assert!(d.cells_after > d.cells_before, "{d:?}");
        assert!(d.volume <= 1e-5 && d.mass <= 1e-5, "{d:?}");
        Ok(())
    }

    // In the open air: in the soil, cells may press into each other by up to what the soil
    // withstands (`soil_grip / mob_rep` of a radius).
    #[test]
    fn cells_do_not_pile_up() -> Result<(), EngineError> {
        let p = on_their_own(Params::default());
        let mut e = judge(&p, 300, 300);
        run(&mut e, 300, None);
        let overlap = worst_overlap(&e.snapshot()?);
        assert!(overlap <= 0.2, "worst overlap {:.1} % of radius", overlap * 100.0);
        Ok(())
    }

    #[test]
    fn same_inputs_same_bits() -> Result<(), EngineError> {
        let p = Params::default();
        let grow = || {
            let mut e = judge(&p, 200, 1024);
            run(&mut e, 200, Some(p.divide_every));
            e.snapshot()
        };
        assert_eq!(grow()?, grow()?);
        Ok(())
    }

    /// The default genome before it could mature: cells stay young, make no cytokinin and cast
    /// no shadow. What the root does is measured on it.
    fn frugal() -> Params {
        Params {
            youth_rate: 0.0,
            ck_light: 0.0,
            ck_block: 0.1,
            shade: 0.0,
            squeeze_hold: 0.0,
            ..Params::default()
        }
    }

    /// The same genome with no soil to hold it and no light to feed it: what the cells do on
    /// their own.
    fn on_their_own(p: Params) -> Params {
        Params {
            soil_grip: 0.0,
            soil_hardness: 0.0,
            light_production: 0.0,
            ..p
        }
    }

    fn germinate(p: &Params) -> Result<Cells, EngineError> {
        grow_seed(p, 1024, 24_000)
    }

    fn grow_seed(p: &Params, capacity: usize, steps: u32) -> Result<Cells, EngineError> {
        let mut e = Reference::new(p, &Cells::seed(p, capacity, 1.0));
        run(&mut e, steps, Some(p.divide_every));
        e.snapshot()
    }

    // Nothing below says "grow down" or "make a tip": each cell pumps auxin toward the side its
    // pumps sit on, moves them toward richer neighbours, along the flow and a little down, and
    // grows when it holds enough auxin. The tests measure what comes out.
    #[test]
    fn a_seed_grows_a_column_that_goes_down_with_the_hormone_at_its_tip() -> Result<(), EngineError> {
        let p = on_their_own(frugal());
        let cells = germinate(&p)?;
        let l = lie(&cells, p.gravity);
        assert!(cells.count >= 100, "only {} cells", cells.count);
        assert!(l.elongation >= 2.0, "{l:?}");
        assert!(l.front >= 5.0 * l.back, "{l:?}");
        assert!(l.reach.0 >= 5.0 * l.reach.1, "{l:?}");
        Ok(())
    }

    #[test]
    fn the_column_follows_gravity_wherever_it_points() -> Result<(), EngineError> {
        let p = on_their_own(Params {
            gravity: [0.6, 0.0, 0.8],
            ..frugal()
        });
        let cells = germinate(&p)?;
        let l = lie(&cells, p.gravity);
        assert!(l.elongation >= 2.0 && l.reach.0 >= 5.0 * l.reach.1, "{l:?}");
        Ok(())
    }

    #[test]
    fn without_pumps_the_seed_stays_a_ball() -> Result<(), EngineError> {
        let p = Params {
            polar: 0.0,
            half_sat: 1.0,
            ..Params::default()
        };
        let mut e = Reference::new(&p, &Cells::seed(&p, 1024, 1.0));
        run(&mut e, 4000, Some(p.divide_every));
        let l = lie(&e.snapshot()?, p.gravity);
        assert!((0.7..=1.4).contains(&l.elongation), "{l:?}");
        Ok(())
    }

    // Measured 2026-11-25: pumps that do not sense gravity still gather on one side of each cell
    // (polarity 0.45 to 0.89), but no column comes out, whatever the weight of the flow. Every
    // cell makes auxin at the same pace and none takes it away: there is no source and no sink
    // for a channel to run between. This test records that; it is expected to break the day a
    // source and a sink emerge.
    #[test]
    fn pumps_blind_to_gravity_make_no_column_yet() -> Result<(), EngineError> {
        let p = on_their_own(Params {
            gravisense: 0.0,
            ..frugal()
        });
        let l = lie(&germinate(&p)?, p.gravity);
        assert!(l.elongation <= 1.4 && l.reach.0 <= 2.0 * l.reach.1, "{l:?}");
        Ok(())
    }

    // The soil holds the cells: the seed cell (it keeps index 0 through its divisions) is pushed
    // aside by the growing tissue far less than in the open air.
    #[test]
    fn the_soil_holds_the_seed_in_place() -> Result<(), EngineError> {
        let drift = |p: Params| -> Result<f32, EngineError> {
            let q = grow_seed(&p, 1024, 24_000)?.pr[0];
            Ok((q[0] * q[0] + q[1] * q[1] + q[2] * q[2]).sqrt())
        };
        // Unlit, so that the soil is the only difference.
        let soft = Params {
            soil_hardness: 0.0,
            light_production: 0.0,
            ..frugal()
        };
        let (held, free) = (drift(soft)?, drift(on_their_own(soft))?);
        // Measured 2026-11-25: 1.48 against 2.54.
        eprintln!("seed cell drift: {held:.2} in the soil, {free:.2} in the open air");
        assert!(held <= 0.7 * free, "{held} against {free}");
        Ok(())
    }

    // A hard soil pushes back on the tip of the root, which then reaches less deep.
    #[test]
    fn a_hard_soil_slows_the_root() -> Result<(), EngineError> {
        let depth = |hardness: f32| -> Result<f64, EngineError> {
            let p = Params {
                soil_hardness: hardness,
                ..frugal()
            };
            Ok(lie(&germinate(&p)?, p.gravity).reach.0)
        };
        let (hard, soft) = (depth(3.0)?, depth(0.0)?);
        // Measured 2026-11-25 at a hardness of 3: 20.8 against 24.5. At the default hardness (0.3)
        // the root is barely slowed (24.2): only the cells of its surface press on the soil.
        eprintln!("root depth: {hard:.1} in a hard soil, {soft:.1} in a soft one");
        assert!(hard <= 0.9 * soft, "{hard} against {soft}");
        Ok(())
    }

    // Light-made auxin: the top of the seed, just under the surface, gets some light and makes
    // more auxin, so it grows too, against the root anchored below. Measured 2026-11-25 on three
    // seeds: a third to a half of the cells end above the surface (none without light). What
    // comes out is a mass, not a shoot: see docs/contexte/Feuille de route.md.
    #[test]
    fn light_made_auxin_lifts_part_of_the_plant_out_of_the_soil() -> Result<(), EngineError> {
        let p = Params {
            gravisense: 0.1,
            ..frugal()
        };
        let cells = germinate(&p)?;
        let out = cells
            .alive()
            .iter()
            .filter(|q| crate::soil::share(&p, &q[..3]) < 0.5)
            .count();
        assert!(
            out * 5 >= cells.count,
            "{out} of {} cells above the surface",
            cells.count
        );
        Ok(())
    }

    // Measured 2026-11-25 on three seeds and six soils: at most two cells ever break the surface.
    // The root only grows at its tip, where the auxin gathers; the top of the seed, drained of
    // auxin, does not grow, so nothing pushes up against the hold of the soil. This test records
    // that; it is expected to break the day a shoot emerges (light-made auxin off).
    #[test]
    fn nothing_comes_out_of_the_soil_yet() -> Result<(), EngineError> {
        let p = Params {
            light_production: 0.0,
            ..frugal()
        };
        let cells = germinate(&p)?;
        let out = cells
            .alive()
            .iter()
            .filter(|q| crate::soil::share(&p, &q[..3]) < 0.5)
            .count();
        assert!(
            out * 100 <= cells.count,
            "{out} of {} cells above the surface",
            cells.count
        );
        Ok(())
    }

    /// A genome whose cells stretch where auxin is moderate and turn toward the auxin flow.
    fn stretching() -> Params {
        Params {
            elong: 0.002,
            e_half: 0.2,
            e_block: 1.0,
            max_half_length: 2.25,
            align: 10.0,
            ..frugal()
        }
    }

    // A cheap guard on the judge: the stretching genome does stretch its cells. Whether they line
    // up with the flow only shows on a big tissue: `geno_gpu/tests/emergence.rs` measures it.
    #[test]
    fn the_stretching_genome_stretches_its_cells() -> Result<(), EngineError> {
        let p = on_their_own(stretching());
        let l = lie(&grow_seed(&p, 1024, 12_000)?, p.gravity);
        assert!(l.stretch >= 2.0, "{l:?}");
        Ok(())
    }
}
