//! What emerges on a big tissue, measured on the graphics card. The judge is too slow for sixteen
//! thousand cells, and some effects only show at that size. Skipped on a machine without a card
//! (the software adapter would take hours).

use geno_gpu::{Adapter, GpuSim};
use geno_sim::checks::{lie, run};
use geno_sim::{Cells, Engine, EngineError, Params};

/// Grows a seed until the tissue is nearly full, or `None` without a graphics card.
fn grow(p: &Params) -> Result<Option<Cells>, EngineError> {
    const CAPACITY: usize = 16_384;
    let mut sim = match GpuSim::new(p, &Cells::seed(p, CAPACITY, 1.0), Adapter::Hardware) {
        Ok(sim) => sim,
        Err(e) => {
            eprintln!("hardware adapter skipped: {e}");
            return Ok(None);
        }
    };
    for _ in 0..40 {
        run(&mut sim, 2000, Some(p.divide_every));
        let cells = sim.snapshot()?;
        if cells.count >= CAPACITY * 97 / 100 {
            return Ok(Some(cells));
        }
    }
    Err(EngineError("the tissue never filled up".into()))
}

/// A genome whose cells stretch where auxin is moderate and turn toward the auxin flow.
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

// Nothing says "line up vertically": each cell only turns its growth direction toward the auxin
// that flows through it. The control genome is identical except that growth directions never
// turn. Several seeds and a tilted gravity show the cells follow the flow, not an axis.
#[test]
fn stretched_cells_line_up_with_the_auxin_flow() -> Result<(), EngineError> {
    let runs = [
        (1, [0.0, -1.0, 0.0]),
        (2, [0.0, -1.0, 0.0]),
        (3, [0.0, -1.0, 0.0]),
        (1, [0.6, 0.0, 0.8]),
    ];
    let (mut turning, mut control) = (0.0, 0.0);
    for (seed, gravity) in runs {
        let p = Params {
            seed,
            gravity,
            ..stretching()
        };
        let (Some(a), Some(b)) = (grow(&p)?, grow(&Params { align: 0.0, ..p })?) else {
            return Ok(());
        };
        let (l, c) = (lie(&a, p.gravity), lie(&b, p.gravity));
        eprintln!(
            "seed {seed} {gravity:?}: turning {:.2}, control {:.2}, {l:?}",
            l.alignment, c.alignment
        );
        assert!(l.stretch >= 2.0 && l.alignment >= 0.65, "{l:?}");
        turning += l.alignment / runs.len() as f64;
        control += c.alignment / runs.len() as f64;
    }
    // Measured 2026-11-25 with the pumps: 0.74 to 0.80 against 0.32 to 0.53.
    assert!(
        turning >= control + 0.2,
        "turning {turning:.2}, control {control:.2}"
    );
    Ok(())
}

// Nothing says "grow at the tip": cells only stay young where auxin gathers and mature where it
// is scarce, and only young cells divide. Measured 2026-11-25: at 16000 cells the root reaches
// 109 deep against 44 without maturing; at 30000 cells the root reached
// 137 deep against 50 without maturing, and the old tissue near the seed stayed narrower than the
// tip (it stopped thickening once mature).
#[test]
fn maturing_cells_make_the_root_grow_from_its_tip() -> Result<(), EngineError> {
    let p = stretching();
    let maturing = Params {
        youth_rate: 0.01,
        youth_half: 3.0,
        ..p
    };
    let (Some(a), Some(b)) = (grow(&maturing)?, grow(&p)?) else {
        return Ok(());
    };
    let (l, c) = (lie(&a, p.gravity), lie(&b, p.gravity));
    eprintln!("root depth: {:.0} maturing, {:.0} not", l.reach.0, c.reach.0);
    assert!(l.reach.0 >= 1.8 * c.reach.0, "{l:?} against {c:?}");
    Ok(())
}

// Light-made cytokinin keeps the cells young where auxin is scarce. Measured 2026-11-25: it is all
// or nothing. Weak, the top still matures and stays buried; strong, every lit cell stays young and
// the top bursts into a ball (a callus, as in Skoog and Miller's dishes), not a shoot: nothing
// confines the young cells to a tip. This test records the ball; it is expected to break the day
// a tip holds itself (in real plants, the stem cells hold back their neighbours: WUS/CLV3).
#[test]
fn light_made_cytokinin_wakes_the_top_into_a_ball_not_a_shoot() -> Result<(), EngineError> {
    let p = Params {
        youth_rate: 0.01,
        youth_half: 3.0,
        ck_light: 0.01,
        ck_block: 1.0,
        ..stretching()
    };
    let Some(cells) = grow(&p)? else {
        return Ok(());
    };
    let air: Vec<_> = cells
        .alive()
        .iter()
        .filter(|q| geno_sim::soil::share(&p, &q[..3]) < 0.5)
        .collect();
    assert!(
        air.len() * 3 >= cells.count,
        "{} of {} in the air",
        air.len(),
        cells.count
    );
    // As wide as it is tall: twice the spread across against the height.
    let n = air.len() as f32;
    let mean = [0, 2].map(|k| air.iter().map(|q| q[k]).sum::<f32>() / n);
    let across = (air
        .iter()
        .map(|q| (q[0] - mean[0]).powi(2) + (q[2] - mean[1]).powi(2))
        .sum::<f32>()
        / n)
        .sqrt();
    let height = air.iter().map(|q| q[1]).fold(f32::MIN, f32::max) - p.surface;
    eprintln!(
        "aerial part: {} cells, height {height:.1}, spread across {across:.1}",
        air.len()
    );
    assert!(height <= 3.0 * across, "height {height}, spread {across}");
    Ok(())
}
