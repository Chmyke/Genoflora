//! The GPU engine against the CPU judge, on the software adapter (always there on Windows) and
//! on the graphics card when the machine has one.

use geno_gpu::{Adapter, GpuSim};
use geno_sim::checks::{division, gap_to_judge, lattice, mass_drift, run, worst_overlap};
use geno_sim::{Engine, EngineError, Params};

/// Runs `check` on every adapter. The software adapter is mandatory; a missing graphics card
/// only skips the hardware run.
///
/// One test at a time: tests run in parallel threads, and twice (2026-11-25) a parallel run spun
/// forever on one core, outside any GPU wait (the 2-minute wait limit never fired). Creating
/// several devices at once is the likely trigger; serial runs never hung.
fn on_each_adapter(check: impl Fn(Adapter) -> Result<(), EngineError>) -> Result<(), EngineError> {
    static ONE_AT_A_TIME: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _turn = ONE_AT_A_TIME
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    check(Adapter::Software)?;
    let p = Params::default();
    match GpuSim::new(&p, &lattice(1, 1, &p), Adapter::Hardware) {
        Ok(_) => check(Adapter::Hardware),
        Err(e) => {
            eprintln!("hardware adapter skipped: {e}");
            Ok(())
        }
    }
}

/// Cell-by-cell comparison only holds on a short run: cells divide as soon as they are ready, and
/// a rounding difference that tips one division moves every later cell (the tissue is chaotic).
/// The graphics card rounds square roots and divisions less finely than WARP: since the soil
/// slows growth through both, a division tipped one step apart at step 34 on the card (measured
/// 2026-11-25; equal for 60 steps without soil), never on WARP.
fn exact_steps(adapter: Adapter) -> u32 {
    match adapter {
        Adapter::Software => 60,
        Adapter::Hardware => 30,
    }
}

/// Beyond that, the two tissues must agree in count (within 1 %), shape and hormone mass.
fn assert_alike(adapter: Adapter, gap: &geno_sim::checks::Gap) {
    let counts = (gap.counts.0 as f64 - gap.counts.1 as f64).abs() / gap.counts.1 as f64;
    eprintln!("{adapter:?} {gap:?}");
    assert!(
        counts <= 0.01 && gap.shape <= 1e-2 && gap.mass <= 1e-2,
        "{adapter:?} {gap:?}"
    );
}

fn engine(p: &Params, n: usize, capacity: usize, adapter: Adapter) -> Result<GpuSim, EngineError> {
    GpuSim::new(p, &lattice(n, capacity, p), adapter)
}

#[test]
fn matches_the_judge_cell_by_cell_on_a_short_run() -> Result<(), EngineError> {
    on_each_adapter(|adapter| {
        // The test tissue straddles the soil surface: part of it in the dark, part in the light.
        // Its cells mature fast and answer cytokinin weakly (the test tissue starts full of it), so
        // that some stop dividing within the run.
        let p = Params {
            youth_rate: 0.05,
            ck_light: 0.001,
            ck_half: 20.0,
            ..Params::default()
        };
        let start = lattice(500, 4096, &p);
        let mut gpu = GpuSim::new(&p, &start, adapter)?;
        let gap = gap_to_judge(&mut gpu, &p, &start, exact_steps(adapter))?;
        assert_eq!(gap.counts.0, gap.counts.1, "{adapter:?} {gap:?}");
        assert!(gap.position <= 1e-3 && gap.mass <= 1e-4, "{adapter:?} {gap:?}");
        let cells = gpu.snapshot()?;
        let mature = cells.youths().iter().filter(|&&y| y <= 0.5).count();
        assert!(mature > 20, "{adapter:?} only {mature} mature cells");
        Ok(())
    })
}

#[test]
fn stops_dividing_at_capacity_like_the_judge() -> Result<(), EngineError> {
    on_each_adapter(|adapter| {
        let p = Params::default();
        let start = lattice(500, 600, &p);
        let gap = gap_to_judge(&mut GpuSim::new(&p, &start, adapter)?, &p, &start, 300)?;
        assert_eq!(gap.counts, (600, 600), "{adapter:?} {gap:?}");
        assert_alike(adapter, &gap);
        Ok(())
    })
}

#[test]
fn keeps_the_shape_of_the_judge_on_a_long_run() -> Result<(), EngineError> {
    on_each_adapter(|adapter| {
        let p = Params::default();
        let start = lattice(300, 4096, &p);
        let gap = gap_to_judge(&mut GpuSim::new(&p, &start, adapter)?, &p, &start, 600)?;
        assert_alike(adapter, &gap);
        Ok(())
    })
}

#[test]
fn diffusion_conserves_the_hormone() -> Result<(), EngineError> {
    on_each_adapter(|adapter| {
        let p = Params {
            production: 0.0,
            light_production: 0.0,
            decay: 0.0,
            ck_production: 0.0,
            ck_decay: 0.0,
            ..Params::default()
        };
        let drift = mass_drift(&mut engine(&p, 1000, 1000, adapter)?, 1000)?;
        assert!(drift <= 1e-5, "{adapter:?} hormone drift {drift:.2e}");
        Ok(())
    })
}

#[test]
fn division_conserves_volume_and_hormone() -> Result<(), EngineError> {
    on_each_adapter(|adapter| {
        let p = Params::default();
        let d = division(&mut engine(&p, 1000, 2048, adapter)?, 20)?;
        assert!(d.cells_after > d.cells_before, "{adapter:?} {d:?}");
        assert!(d.volume <= 1e-5 && d.mass <= 1e-5, "{adapter:?} {d:?}");
        Ok(())
    })
}

#[test]
fn cells_do_not_pile_up() -> Result<(), EngineError> {
    on_each_adapter(|adapter| {
        // Without soil: in the soil, cells may press into each other by what it withstands.
        let p = Params {
            soil_grip: 0.0,
            soil_hardness: 0.0,
            ..Params::default()
        };
        let mut e = engine(&p, 1000, 1000, adapter)?;
        run(&mut e, 300, None);
        let overlap = worst_overlap(&e.snapshot()?);
        assert!(
            overlap <= 0.2,
            "{adapter:?} worst overlap {:.1} % of radius",
            overlap * 100.0
        );
        Ok(())
    })
}

#[test]
fn same_inputs_same_bits_on_the_same_machine() -> Result<(), EngineError> {
    on_each_adapter(|adapter| {
        let p = Params::default();
        let grow = || {
            let mut e = engine(&p, 1000, 4096, adapter)?;
            run(&mut e, 300, Some(p.divide_every));
            e.snapshot()
        };
        assert_eq!(grow()?, grow()?, "{adapter:?}");
        Ok(())
    })
}

#[test]
fn a_seed_germinates_like_on_the_judge() -> Result<(), EngineError> {
    on_each_adapter(|adapter| {
        // Cytokinin switched on, so the judge and the shaders are compared on both hormones.
        // Light-made auxin stays off: WARP's exponential differs from the CPU's by a few parts in
        // a million, enough to tip a division at step 1300 (measured 2026-11-25). The light rule is
        // compared cell by cell on the short runs.
        let p = Params {
            half_sat: 10.0,
            polar: 0.1,
            ck_production: 0.0001,
            light_production: 0.0,
            ..Params::default()
        };
        // Exact on WARP only (measured 2026-11-25: equal for 4000 steps, apart by 6000: the cube
        // root of even growth is rounded a little differently, then one division tips and the
        // tissues part ways). The graphics card rounds divisions and square roots less finely
        // still; a cell then reaches its dividing size one step apart within 2500 steps. The card
        // stays covered by the short exact runs and the long statistical ones.
        if adapter == Adapter::Hardware {
            return Ok(());
        }
        let start = geno_sim::Cells::seed(&p, 1024, 1.0);
        let gap = gap_to_judge(&mut GpuSim::new(&p, &start, adapter)?, &p, &start, 4000)?;
        assert_eq!(gap.counts.0, gap.counts.1, "{adapter:?} {gap:?}");
        assert!(gap.position <= 1e-3 && gap.mass <= 1e-4, "{adapter:?} {gap:?}");
        Ok(())
    })
}

/// A genome whose cells stretch and turn, so the shaders are compared on the cell shape too.
fn stretching() -> Params {
    Params {
        half_sat: 10.0,
        polar: 0.1,
        elong: 0.002,
        e_half: 0.2,
        e_block: 1.0,
        max_half_length: 2.25,
        align: 10.0,
        ..Params::default()
    }
}

#[test]
fn stretching_cells_match_the_judge_cell_by_cell_on_a_short_run() -> Result<(), EngineError> {
    on_each_adapter(|adapter| {
        let p = stretching();
        let start = lattice(500, 4096, &p);
        let mut gpu = GpuSim::new(&p, &start, adapter)?;
        let gap = gap_to_judge(&mut gpu, &p, &start, exact_steps(adapter))?;
        assert_eq!(gap.counts.0, gap.counts.1, "{adapter:?} {gap:?}");
        assert!(gap.position <= 1e-3 && gap.mass <= 1e-4, "{adapter:?} {gap:?}");
        let stretched = gpu
            .snapshot()?
            .shapes()
            .iter()
            .filter(|s| geno_sim::checks::length_over_width(s) > 1.05)
            .count();
        assert!(stretched > 20, "{adapter:?} only {stretched} stretched cells");
        Ok(())
    })
}
