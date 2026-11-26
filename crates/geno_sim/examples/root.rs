//! Probe of the root on the judge: grows one seed cell and prints the root cut across at several
//! heights above its tip, with the way the auxin pumps point in each layer. It tells whether
//! auxin goes down the middle of the root and back up its outside (the reverse fountain of a
//! real root, which keeps the auxin maximum narrow) or funnels down everywhere.
//! Usage: cargo run --release -p geno_sim --example root -- <steps> [capacity=4096]
//!        [any parameter of the genome, as name=value]

use geno_sim::checks::{pumps_across, run};
use geno_sim::{Cells, Engine, Params, Reference};

/// Heights above the root tip of the slabs read (a last one is read at mid-depth), their
/// thickness, and how far from the axis they are read.
const HEIGHTS: [f32; 3] = [1.0, 4.0, 8.0];
const THICK: f32 = 2.0;
const LAYERS: usize = 16;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let steps: u32 = args.get(1).map_or(Ok(20_000), |s| s.parse())?;
    let mut p = Params::default();
    let mut capacity = 1 << 12;
    for arg in args.iter().skip(2) {
        let (key, value) = arg.split_once('=').ok_or("expected key=value")?;
        match key {
            "capacity" => capacity = value.parse()?,
            _ => p.set(key, value)?,
        }
    }
    let mut sim = Reference::new(&p, &Cells::seed(&p, capacity, 1.0));
    // Stops short of a full tissue: a full one no longer divides, which is another plant.
    let mut done = 0;
    while done < steps && sim.snapshot()?.count < capacity * 9 / 10 {
        run(&mut sim, 500, Some(p.divide_every));
        done += 500;
    }
    let cells = sim.snapshot()?;
    let in_soil = |q: &[f32; 4]| geno_sim::soil::share(&p, &q[..3]) >= 0.5;
    let soil: Vec<&[f32; 4]> = cells.alive().iter().filter(|q| in_soil(q)).collect();
    let Some(tip) = soil.iter().min_by(|a, b| a[1].total_cmp(&b[1])) else {
        println!("step {done}: {} cells, no root", cells.count);
        return Ok(());
    };
    let depth = p.surface - tip[1];
    println!(
        "step {done}: {} cells, {} in the soil, root {depth:.1} deep",
        cells.count,
        soil.len()
    );
    for above in HEIGHTS.into_iter().chain([0.5 * depth]) {
        println!("across the root {above:.0} above its tip, from its axis outward");
        println!("  layer cells youth auxin pumps_up pumps_out");
        let cut = pumps_across(&cells, &p, [tip[0], tip[1], tip[2]], above, THICK, LAYERS);
        for (layer, f) in cut.iter().enumerate().filter(|(_, f)| f.cells > 0) {
            println!(
                "  {layer:>5} {:>5} {:>5.2} {:>6.2} {:>8.2} {:>9.2}",
                f.cells, f.youth, f.auxin, f.up, f.out
            );
        }
    }
    Ok(())
}
