//! Probe of the relay and restraint loop alone, on the judge: a tissue that neither grows, moves
//! nor divides, standing in the air and lit from above, left until its hormones settle. It shows
//! what the loop sets by itself, away from growth: which cells end up holding enough relay to stay
//! young, and whether that depends on the size and on the shape of the tissue.
//! The tissue is a column of round cells on a cubic grid, with a flat top or a rounded one.
//! Prints the cut under the summit and the cuts across, like the `report` probe of the card.
//! Usage: cargo run --release -p geno_sim --example relay_loop -- <steps> <radius> <height>
//!        [dome=0] [start=0] [any parameter of the genome, as name=value]
//! `radius` and `height` are in cells. `dome=1` rounds the top into a half ball of that radius.
//! `start` is the relay concentration every cell holds at first (to see whether the outcome
//! depends on where the loop starts from).

use geno_sim::cells::RELAY;
use geno_sim::checks::{Layer, across, under};
use geno_sim::{Cells, Engine, Params, Reference};

/// Steps the cells are left to settle against each other before the tissue is frozen.
const SETTLING_STEPS: u32 = 300;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let steps: u32 = args.get(1).map_or(Ok(3000), |s| s.parse())?;
    let radius: i32 = args.get(2).map_or(Ok(5), |s| s.parse())?;
    let height: i32 = args.get(3).map_or(Ok(12), |s| s.parse())?;
    // Frozen tissue: no growth, no force, and no auxin, so that only the loop is at work.
    let mut p = Params {
        growth: 0.0,
        elong: 0.0,
        mob_rep: 0.0,
        mob_adh: 0.0,
        production: 0.0,
        light_production: 0.0,
        ..Params::default()
    };
    let (mut dome, mut start) = (false, 0.0_f32);
    for arg in args.iter().skip(4) {
        let (key, value) = arg.split_once('=').ok_or("expected key=value")?;
        match key {
            "dome" => dome = value != "0",
            "start" => start = value.parse()?,
            _ => p.set(key, value)?,
        }
    }
    // PITFALL: a perfect grid has no skin (`rules::skin` reads how much the neighbours of a cell
    // lie on one side beyond chance, and the four side neighbours of a grid cancel exactly). The
    // cells start off the grid by a share of their radius, then settle under their own forces
    // before the tissue is frozen, which packs them unevenly like a grown tissue.
    let (r, pitch) = (1.0_f32, 1.7_f32);
    let shift = |n: usize, k: u32| 0.4 * r * geno_sim::hash::axis(p.seed, n as u32, k)[0];
    let foot = p.surface + 4.0 * pitch;
    let mut spots = Vec::new();
    for y in 0..height {
        // Under a rounded top, the column narrows as a half ball over its last `radius` layers.
        let above = y - (height - 1 - radius);
        let reach2 = if dome && above > 0 {
            radius * radius - above * above
        } else {
            radius * radius
        };
        for x in -radius..=radius {
            for z in -radius..=radius {
                if x * x + z * z <= reach2 {
                    spots.push([x as f32 * pitch, foot + y as f32 * pitch, z as f32 * pitch, r]);
                }
            }
        }
    }
    let mut cells = Cells::empty(spots.len());
    cells.count = spots.len();
    for (i, spot) in spots.iter().enumerate() {
        cells.pr[i] = [
            spot[0] + shift(i, 0),
            spot[1] + shift(i, 1),
            spot[2] + shift(i, 2),
            r,
        ];
    }
    let loose = Params {
        mob_rep: Params::default().mob_rep,
        mob_adh: Params::default().mob_adh,
        ck_light: 0.0,
        ..p
    };
    let mut settling = Reference::new(&loose, &cells);
    for _ in 0..SETTLING_STEPS {
        settling.step();
    }
    let mut cells = settling.snapshot()?;
    for i in 0..cells.count {
        cells.h[i] = [0.0; 4];
        cells.h[i][RELAY] = start * r * r * r;
    }
    let mut sim = Reference::new(&p, &cells);
    for _ in 0..steps {
        sim.step();
    }
    let cells = sim.snapshot()?;
    let top = [0.0, foot + (height - 1) as f32 * pitch, 0.0];
    let young = (0..cells.count)
        .filter(|&i| cells.h[i][RELAY] / (r * r * r) > p.relay_half)
        .count();
    println!(
        "{} cells, radius {radius}, height {height}, {}: {young} hold enough relay to stay young",
        cells.count,
        if dome { "rounded top" } else { "flat top" }
    );
    println!("under the summit, going down");
    print_layers(&under(&cells, &p, top, 0.5 * pitch, height as usize));
    for depth in [1, 4, 8] {
        if depth < height {
            println!("across, {depth} cells under the summit, from the axis outward");
            let cut = across(
                &cells,
                &p,
                top,
                depth as f32 * pitch,
                0.5 * pitch,
                2 * radius as usize + 2,
            );
            print_layers(&cut);
        }
    }
    Ok(())
}

fn print_layers(layers: &[Layer]) {
    println!("  layer cells light skin cytokinin restraint relay making");
    for (n, l) in layers.iter().enumerate().filter(|(_, l)| l.cells > 0) {
        println!(
            "  {n:>5} {:>5} {:>5.2} {:>4.2} {:>9.3} {:>9.3} {:>5.2} {:>6.3}",
            l.cells, l.light, l.skin, l.cytokinin, l.restraint, l.relay, l.making
        );
    }
}
