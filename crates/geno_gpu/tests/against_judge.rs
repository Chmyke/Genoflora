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

fn engine(p: &Params, n: usize, capacity: usize, adapter: Adapter) -> Result<GpuSim, EngineError> {
    GpuSim::new(p, &lattice(n, capacity, p), adapter)
}

#[test]
fn matches_the_judge_cell_by_cell_on_a_short_run() -> Result<(), EngineError> {
    on_each_adapter(|adapter| {
        let p = Params::default();
        let start = lattice(500, 4096, &p);
        let gap = gap_to_judge(&mut GpuSim::new(&p, &start, adapter)?, &p, &start, 200)?;
        assert_eq!(gap.counts.0, gap.counts.1, "{adapter:?} {gap:?}");
        assert!(gap.position <= 1e-3 && gap.mass <= 1e-4, "{adapter:?} {gap:?}");
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
        assert!(gap.position <= 1e-3 && gap.mass <= 1e-4, "{adapter:?} {gap:?}");
        Ok(())
    })
}

#[test]
fn keeps_the_shape_of_the_judge_on_a_long_run() -> Result<(), EngineError> {
    on_each_adapter(|adapter| {
        let p = Params::default();
        let start = lattice(300, 4096, &p);
        let gap = gap_to_judge(&mut GpuSim::new(&p, &start, adapter)?, &p, &start, 600)?;
        assert_eq!(gap.counts.0, gap.counts.1, "{adapter:?} {gap:?}");
        assert!(gap.shape <= 1e-3 && gap.mass <= 1e-4, "{adapter:?} {gap:?}");
        Ok(())
    })
}

#[test]
fn diffusion_conserves_the_hormone() -> Result<(), EngineError> {
    on_each_adapter(|adapter| {
        let p = Params {
            production: 0.0,
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
        let d = division(&mut engine(&p, 1000, 2048, adapter)?, p.divide_every)?;
        assert!(d.cells_after > d.cells_before, "{adapter:?} {d:?}");
        assert!(d.volume <= 1e-5 && d.mass <= 1e-5, "{adapter:?} {d:?}");
        Ok(())
    })
}

#[test]
fn cells_do_not_pile_up() -> Result<(), EngineError> {
    on_each_adapter(|adapter| {
        let p = Params::default();
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
        let p = Params {
            half_sat: 10.0,
            polar: 0.1,
            ck_production: 0.0001,
            ..Params::default()
        };
        let start = geno_sim::Cells::seed(&p, 256, 1.0);
        let gap = gap_to_judge(&mut GpuSim::new(&p, &start, adapter)?, &p, &start, 4000)?;
        assert_eq!(gap.counts.0, gap.counts.1, "{adapter:?} {gap:?}");
        assert!(gap.shape <= 1e-2 && gap.mass <= 1e-3, "{adapter:?} {gap:?}");
        Ok(())
    })
}

/// A genome whose cells stretch and turn, so the shaders are compared on the cell shape too.
/// Stretching adds work per step, so rounding gaps grow faster: the exact run is shorter.
fn stretching() -> Params {
    Params {
        half_sat: 10.0,
        polar: 0.1,
        elong: 0.002,
        e_half: 0.2,
        e_block: 1.0,
        stretch_max: 1.5,
        align: 0.02,
        ..Params::default()
    }
}

#[test]
fn stretching_cells_match_the_judge_cell_by_cell_on_a_short_run() -> Result<(), EngineError> {
    on_each_adapter(|adapter| {
        let p = stretching();
        let start = lattice(500, 4096, &p);
        let mut gpu = GpuSim::new(&p, &start, adapter)?;
        let gap = gap_to_judge(&mut gpu, &p, &start, 120)?;
        assert_eq!(gap.counts.0, gap.counts.1, "{adapter:?} {gap:?}");
        assert!(gap.position <= 1e-3 && gap.mass <= 1e-4, "{adapter:?} {gap:?}");
        let stretched = gpu.snapshot()?.shapes().iter().filter(|s| s[3] > 1.05).count();
        assert!(stretched > 50, "{adapter:?} only {stretched} stretched cells");
        Ok(())
    })
}
