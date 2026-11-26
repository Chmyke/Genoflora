//! What emerges on a big tissue, measured on the graphics card. The judge is too slow for sixteen
//! thousand cells, and some effects only show at that size. Skipped on a machine without a card
//! (the software adapter would take hours).

use geno_gpu::{Adapter, GpuSim};
use geno_sim::checks::{across, foci, lie, packing, run};
use geno_sim::{Cells, Engine, EngineError, Params};

/// Grows a seed until the tissue is nearly full, or `None` without a graphics card.
fn grow(p: &Params) -> Result<Option<Cells>, EngineError> {
    grow_watched(p, 2000, |_| ())
}

/// Same, showing the tissue to `watch` every `every` steps on the way.
fn grow_watched(p: &Params, every: u32, mut watch: impl FnMut(&Cells)) -> Result<Option<Cells>, EngineError> {
    const CAPACITY: usize = 16_384;
    // PITFALL: tests run in parallel threads, and several devices alive at once have left a run
    // spinning forever on one core, off the card. Guarding the creation alone was not enough
    // (seen three times on 2026-11-26, seven tests done and one thread spinning for 80 minutes),
    // so one plant grows at a time. It costs nothing: the card is the limit, and the tests take
    // 209 s in a row against 243 s side by side.
    static ONE_AT_A_TIME: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _turn = ONE_AT_A_TIME
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut sim = match GpuSim::new(p, &Cells::seed(p, CAPACITY, 1.0), Adapter::Hardware) {
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
// 0.01 to 0.03 (9700 of 16000 cells in the air, 35 high for 17 across). With the relay and the
// restraint signal, still a ball without shade: 7800 cells in the air, 37 high for 18 across.
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
// for 6 to 10 across, root 85 to 95 deep. With the relay and the restraint signal: 3900 to 4200
// cells in the air, 74 to 76 high for 10 to 12 across, root 81 to 82 deep.
#[test]
fn shaded_cells_mature_and_a_stem_rises_from_the_lit_top() -> Result<(), EngineError> {
    for seed in 1..=3 {
        let p = Params {
            seed,
            youth_rate: 0.01,
            youth_half: 3.0,
            ck_light: 0.3,
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

/// Sizes of the biggest young focus in the air, every thousand steps from the moment one reaches
/// ten cells until the tissue is full. `None` without a graphics card.
fn tip_sizes(p: &Params) -> Result<Option<Vec<usize>>, EngineError> {
    let mut sizes = Vec::new();
    let grown = grow_watched(p, 1000, |cells| {
        let aerial = foci(cells, p, |q| geno_sim::soil::share(p, &q[..3]) < 0.5);
        let biggest = aerial.first().map_or(0, |f| f.cells);
        if biggest >= 10 || !sizes.is_empty() {
            sizes.push(biggest);
        }
    })?;
    Ok(grown.map(|_| sizes))
}

// Nothing says how big the tip of the stem is. Cytokinin, made by the lit skin, fades with depth;
// where it calls, cells make a relay that passes from cell to cell and keeps them young; the skin
// cells that hold relay make a restraint signal that goes back down and dulls the call (in real
// plants: WUS and CLV3). The more young cells, the more restraint: the tip settles on a size.
// Before the relay (measured 2026-11-25), the biggest focus wandered between 3 and 147 cells and
// a restraint signal alone changed nothing: every cell of the stem sat next to the dividing
// threshold. Measured with the loop, three seeds: 180 to 450 cells over 30000 steps on a plant of
// 65000 cells. Here, once the tip is set up (second half of the run), it stays within a factor
// four (3.1 at worst: it halves when a side branch leaves it) and never empties.
#[test]
fn the_tip_of_the_stem_holds_its_size() -> Result<(), EngineError> {
    for seed in 1..=3 {
        let p = Params {
            seed,
            ..Params::default()
        };
        let Some(sizes) = tip_sizes(&p)? else {
            return Ok(());
        };
        eprintln!("seed {seed}: biggest focus in the air {sizes:?}");
        assert!(
            sizes.len() >= 10,
            "seed {seed}: the stem came out too late, {sizes:?}"
        );
        let settled = &sizes[sizes.len() / 2..];
        let (low, high) = (
            settled.iter().copied().min().unwrap_or(0),
            settled.iter().copied().max().unwrap_or(0),
        );
        assert!(low >= 100 && high <= 4 * low, "seed {seed}: {settled:?}");
    }
    Ok(())
}

// The restraint signal is what holds the tip: the same plant without it has a tip that keeps
// swelling (measured 2026-11-25: 1400 cells and still growing on a plant of 65000 cells, against
// 200 to 450 with it; twice the signal, 40 to 130).
#[test]
fn without_the_restraint_signal_the_tip_keeps_swelling() -> Result<(), EngineError> {
    let held = Params::default();
    let free = Params {
        restraint_made: 0.0,
        ..held
    };
    let (Some(held), Some(free)) = (tip_sizes(&held)?, tip_sizes(&free)?) else {
        return Ok(());
    };
    eprintln!("with the signal {held:?}, without {free:?}");
    let last = |sizes: &[usize]| sizes.iter().rev().take(3).sum::<usize>() / 3;
    assert!(last(&free) >= 2 * last(&held), "{held:?} against {free:?}");
    Ok(())
}

/// Mean youth and relay of the cells of a slab across the stem, `depth` under its highest cell.
fn slab(cells: &Cells, p: &Params, depth: f32) -> (f32, f32) {
    let in_air = |q: &&[f32; 4]| geno_sim::soil::share(p, &q[..3]) < 0.5;
    let top = cells
        .alive()
        .iter()
        .filter(in_air)
        .max_by(|a, b| a[1].total_cmp(&b[1]))
        .map_or([0.0; 3], |q| [q[0], q[1], q[2]]);
    let layers = across(cells, p, top, depth, 2.0, 64);
    let n = layers.iter().map(|l| l.cells).sum::<usize>().max(1) as f32;
    let mean = |of: fn(&geno_sim::checks::Layer) -> f32| {
        layers.iter().map(|l| of(l) * l.cells as f32).sum::<f32>() / n
    };
    (mean(|l| l.youth), mean(|l| l.relay))
}

// A tip is a place: young cells at the summit, a mature stem under them. With a relay that only
// answers cytokinin, it is not one (measured 2026-11-26): every answer along the way is a gentle
// curve, cytokinin falls twelvefold from the summit to 23 layers down and youth by a factor 1.3,
// so the whole stem sits next to the dividing threshold. A cell that holds relay hears cytokinin
// better (WUS switches off the brakes on the answer to cytokinin, Leibfried et al. 2005): the
// relay then sustains its own making and gathers at the summit. Measured: four layers under the
// summit against mid-height, relay 1.40 against 0.09 and youth 0.64 against 0.18; without it,
// 0.84 against 0.24 and 0.51 against 0.32.
#[test]
fn a_relay_that_sustains_its_own_making_sets_a_young_tip_apart_from_the_stem() -> Result<(), EngineError> {
    const NEAR_THE_SUMMIT: f32 = 4.0;
    let plain = Params::default();
    let sustained = Params {
        ck_brake: 10.0,
        relay_made: 0.1,
        restraint_made: 0.45,
        ..plain
    };
    let contrast = |p: &Params| -> Result<Option<(f32, f32)>, EngineError> {
        let Some(cells) = grow(p)? else {
            return Ok(None);
        };
        let height = cells.alive().iter().map(|q| q[1]).fold(p.surface, f32::max) - p.surface;
        let (tip, stem) = (slab(&cells, p, NEAR_THE_SUMMIT), slab(&cells, p, 0.5 * height));
        eprintln!("height {height}: youth, relay near the summit {tip:?}, at mid-height {stem:?}");
        Ok(Some((tip.0 - stem.0, tip.1 / stem.1)))
    };
    let (Some(plain), Some(sustained)) = (contrast(&plain)?, contrast(&sustained)?) else {
        return Ok(());
    };
    assert!(plain.0 < 0.3 && plain.1 < 6.0, "{plain:?}");
    assert!(sustained.0 > 0.35 && sustained.1 > 8.0, "{sustained:?}");
    Ok(())
}
