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
        cells.pr[i] = [
            q[0] as f32 * SPACING + jitter(0),
            q[1] as f32 * SPACING + jitter(1),
            q[2] as f32 * SPACING + jitter(2),
            p.r_div * (0.8 + 0.2 * unit(hash(p.seed, id, 3))),
        ];
        cells.h[i] = unit(hash(p.seed, id, 4));
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

/// Total hormone.
pub fn mass(c: &Cells) -> f64 {
    c.hormone().iter().map(|&h| f64::from(h)).sum()
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

#[cfg(test)]
mod tests {
    use super::*;

    fn judge(p: &Params, n: usize, capacity: usize) -> Reference {
        Reference::new(p, &lattice(n, capacity, p))
    }

    #[test]
    fn diffusion_conserves_the_hormone() -> Result<(), EngineError> {
        let p = Params {
            production: 0.0,
            decay: 0.0,
            ..Params::default()
        };
        let drift = mass_drift(&mut judge(&p, 300, 300), 500)?;
        assert!(drift <= 1e-5, "hormone drift {drift:.2e}");
        Ok(())
    }

    #[test]
    fn division_conserves_volume_and_hormone() -> Result<(), EngineError> {
        let p = Params::default();
        let d = division(&mut judge(&p, 300, 1024), p.divide_every)?;
        assert!(d.cells_after > d.cells_before, "{d:?}");
        assert!(d.volume <= 1e-5 && d.mass <= 1e-5, "{d:?}");
        Ok(())
    }

    #[test]
    fn cells_do_not_pile_up() -> Result<(), EngineError> {
        let p = Params::default();
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

    #[test]
    fn a_seed_becomes_a_tissue() -> Result<(), EngineError> {
        let p = Params::default();
        let mut e = Reference::new(&p, &Cells::seed(64, p.r_div, 1.0));
        run(&mut e, 800, Some(p.divide_every));
        let cells = e.snapshot()?;
        assert!(cells.count >= 8, "only {} cells", cells.count);
        Ok(())
    }
}
