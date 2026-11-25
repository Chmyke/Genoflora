//! What emerges on a big tissue, measured on the graphics card. The judge is too slow for sixteen
//! thousand cells, and some effects only show at that size. Skipped on a machine without a card
//! (the software adapter would take hours).

use geno_gpu::{Adapter, GpuSim};
use geno_sim::checks::{foci, lie, on_the_edge, packing, run};
use geno_sim::{Cells, Engine, EngineError, Params};

/// Grows a seed until the tissue is nearly full, or `None` without a graphics card.
fn grow(p: &Params) -> Result<Option<Cells>, EngineError> {
    grow_watched(p, 2000, |_| ())
}

/// Same, showing the tissue to `watch` every `every` steps on the way.
fn grow_watched(p: &Params, every: u32, mut watch: impl FnMut(&Cells)) -> Result<Option<Cells>, EngineError> {
    const CAPACITY: usize = 16_384;
    // PITFALL: tests run in parallel threads, and creating several devices at once has left a
    // run spinning forever on one core (seen again 2026-11-25); the plants then grow in parallel.
    static ONE_AT_A_TIME: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let created = {
        let _turn = ONE_AT_A_TIME
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        GpuSim::new(p, &Cells::seed(p, CAPACITY, 1.0), Adapter::Hardware)
    };
    let mut sim = match created {
        Ok(sim) => sim,
        Err(e) => {
            eprintln!("hardware adapter skipped: {e}");
            return Ok(None);
        }
    };
    for _ in 0..80_000 / every {
        run(&mut sim, every, Some(p.divide_every));
        let cells = sim.snapshot()?;
        watch(&cells);
        if cells.count >= CAPACITY * 97 / 100 {
            return Ok(Some(cells));
        }
    }
    Err(EngineError("the tissue never filled up".into()))
}

/// A genome whose cells stretch where auxin is moderate and turn toward the auxin flow. Its cells
/// stay young, make no cytokinin and cast no shadow unless a test says otherwise.
fn stretching() -> Params {
    Params {
        youth_rate: 0.0,
        ck_light: 0.0,
        ck_block: 0.1,
        shade: 0.0,
        squeeze_hold: 0.0,
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
// Remeasured 2026-11-25 once the soil only held the cells that touch it: the threshold moved from
// 0.01 to 0.03 (9700 of 16000 cells in the air, 35 high for 17 across).
#[test]
fn light_made_cytokinin_wakes_the_top_into_a_ball_not_a_shoot() -> Result<(), EngineError> {
    let p = Params {
        youth_rate: 0.01,
        youth_half: 3.0,
        ck_light: 0.03,
        ck_block: 1.0,
        ..stretching()
    };
    let Some(cells) = grow(&p)? else {
        return Ok(());
    };
    let (air, height, across) = aerial(&cells, &p);
    eprintln!("aerial part: {air} cells, height {height:.1}, spread across {across:.1}");
    assert!(air * 3 >= cells.count, "{air} of {} in the air", cells.count);
    // As wide as it is tall.
    assert!(height <= 3.0 * across, "height {height}, spread {across}");
    Ok(())
}

/// The part of the plant above the soil: how many cells, how high it reaches above the surface,
/// and its spread across (root mean square distance to its vertical axis).
fn aerial(cells: &Cells, p: &Params) -> (usize, f32, f32) {
    let air: Vec<_> = cells
        .alive()
        .iter()
        .filter(|q| geno_sim::soil::share(p, &q[..3]) < 0.5)
        .collect();
    let n = air.len().max(1) as f32;
    let mean = [0, 2].map(|k| air.iter().map(|q| q[k]).sum::<f32>() / n);
    let across = (air
        .iter()
        .map(|q| (q[0] - mean[0]).powi(2) + (q[2] - mean[1]).powi(2))
        .sum::<f32>()
        / n)
        .sqrt();
    let height = air.iter().map(|q| q[1]).fold(p.surface, f32::max) - p.surface;
    (air.len(), height, across)
}

// Nothing says "grow up" or "make a stem". Cells cast a shadow on those below them, so only the
// cells on top stay lit, make cytokinin and stay young; those they leave behind mature. The young
// top keeps climbing on what it has laid down: a stem, while the root goes down.
// Measured 2026-11-25 on three seeds at 16000 cells: 375 to 1800 cells in the air, 24 to 52 high
// for 6 to 10 across, root 85 to 95 deep.
#[test]
fn shaded_cells_mature_and_a_stem_rises_from_the_lit_top() -> Result<(), EngineError> {
    for seed in 1..=3 {
        let p = Params {
            seed,
            youth_rate: 0.01,
            youth_half: 3.0,
            ck_light: 0.03,
            ck_block: 1.0,
            shade: 3.0,
            ..stretching()
        };
        let Some(cells) = grow(&p)? else {
            return Ok(());
        };
        let (air, height, across) = aerial(&cells, &p);
        let depth = lie(&cells, p.gravity).reach.0;
        eprintln!(
            "seed {seed}: {air} cells in the air, height {height:.1}, across {across:.1}, root {depth:.0}"
        );
        assert!(
            air >= 200 && height >= 2.5 * across,
            "seed {seed}: {air} cells, height {height}, across {across}"
        );
        assert!(depth >= 30.0, "seed {seed}: root only {depth} deep");
    }
    Ok(())
}

// The plant must hold together as it grows: cells that keep their place have a dozen neighbours,
// and no hormone ever goes below zero. Measured 2026-11-25, before the soil only held the cells
// that touch it: the cells inside the root, held too, piled up as they grew (40 neighbours,
// overlaps deeper than a radius); cytokinin diffusion then overshot, went negative and blew up,
// every cell turned young, and the card gave up.
#[test]
fn the_reference_plant_does_not_pile_up() -> Result<(), EngineError> {
    // The reference plant, and the same without shade: every lit cell stays young and the top
    // divides everywhere (a ball), which is where cells used to be crushed into each other.
    let reference = Params::default();
    let ball = Params {
        shade: 0.0,
        ..reference
    };
    for p in [reference, ball] {
        let Some(cells) = grow(&p)? else {
            return Ok(());
        };
        let packed = packing(&cells, &p);
        eprintln!("shade {}: {packed:?}", p.shade);
        assert!(packed.lowest >= 0.0, "{packed:?}");
        assert!(
            packed.neighbours * p.ck_diffusion.max(p.diffusion).max(p.restraint_diffusion) < 1.5,
            "{packed:?}"
        );
    }
    Ok(())
}

// What the lit top of the reference plant is made of, measured 2026-11-25 on three seeds. A focus
// of young cells does stay at the very top and climbs steadily: a tip. But nothing sets its size:
// it swells and shrinks between about ten and more than a hundred cells, and pieces of it stay
// behind on the lit shoulders, where they live on as tips of their own (side branches, not
// leaves: same hormones as the tip, never flat, no regular spacing). This test records the
// wandering size; it is expected to break the day the tip holds its size (in real plants, the
// stem cells hold back their neighbours: WUS/CLV3).
// Why it wanders, measured 2026-11-25: no cell of the stem is clearly young or clearly mature.
// Cytokinin is about as rich all over the lit top, and what it calls for sits right at the
// threshold under which a cell stops dividing: every lit cell has a youth between 0.4 and 0.6,
// so a focus is whichever cells happen to stand a hair above one half.
#[test]
fn the_tip_of_the_stem_has_no_steady_size_yet() -> Result<(), EngineError> {
    for seed in 1..=3 {
        let p = Params {
            seed,
            ..Params::default()
        };
        let mut sizes = Vec::new();
        let mut branches = 0;
        let grown = grow_watched(&p, 1000, |cells| {
            let aerial = foci(cells, &p, |q| geno_sim::soil::share(&p, &q[..3]) < 0.5);
            // Once the stem is out: a tip of at least ten cells has been seen.
            let biggest = aerial.first().map_or(0, |f| f.cells);
            if biggest >= 10 || !sizes.is_empty() {
                sizes.push(biggest);
            }
            branches = aerial.iter().filter(|f| f.cells >= 6).count();
        })?;
        let Some(cells) = grown else {
            return Ok(());
        };
        let (low, high) = (
            sizes.iter().copied().min().unwrap_or(0),
            sizes.iter().copied().max().unwrap_or(0),
        );
        let lit = |i: usize| cells.pin[i][3] >= 0.5 && geno_sim::soil::share(&p, &cells.pr[i][..3]) < 0.5;
        let undecided = on_the_edge(&cells, 0.1, lit);
        eprintln!(
            "seed {seed}: biggest focus in the air {sizes:?}, {branches} foci at the end, {undecided:.2} of the lit cells on the edge"
        );
        assert!(
            sizes.len() >= 5,
            "seed {seed}: the stem came out too late, {sizes:?}"
        );
        assert!(high >= 4 * low.max(1), "seed {seed}: {sizes:?}");
        assert!(undecided >= 0.9, "seed {seed}: {undecided}");
    }
    Ok(())
}

// A restraint signal made by the cells that can divide, which dulls what cytokinin calls for (the
// CLV3 half of the WUS/CLV3 loop, alone). Measured 2026-11-25 on sixty settings and three seeds:
// it does not hold the size of the tip. Weak, nothing changes; three times stronger than the best
// setting, no stem comes out at all; in between, the wandering narrows on one seed (9 to 35 cells)
// and not on the next (6 to 140). The signal has nothing to tell apart: the cells of the stem all
// sit next to the threshold (see above). This test records it on the best setting found; it is
// expected to break the day a tip holds its size.
#[test]
fn a_restraint_signal_alone_does_not_hold_the_tip() -> Result<(), EngineError> {
    let mut widest = 0;
    for seed in 1..=3 {
        let p = Params {
            seed,
            restraint_made: 0.05,
            restraint_decay: 0.1,
            ..Params::default()
        };
        let mut sizes = Vec::new();
        let grown = grow_watched(&p, 1000, |cells| {
            let aerial = foci(cells, &p, |q| geno_sim::soil::share(&p, &q[..3]) < 0.5);
            let biggest = aerial.first().map_or(0, |f| f.cells);
            if biggest >= 10 || !sizes.is_empty() {
                sizes.push(biggest);
            }
        })?;
        if grown.is_none() {
            return Ok(());
        }
        let (low, high) = (
            sizes.iter().copied().min().unwrap_or(0),
            sizes.iter().copied().max().unwrap_or(0),
        );
        eprintln!("seed {seed}: biggest focus in the air {sizes:?}");
        assert!(
            sizes.len() >= 5,
            "seed {seed}: the stem came out too late, {sizes:?}"
        );
        widest = widest.max(high / low.max(1));
    }
    assert!(widest >= 8, "the tip wandered by a factor {widest} at most");
    Ok(())
}
