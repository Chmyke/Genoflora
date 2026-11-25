//! Probe: grows one seed cell on the GPU and writes snapshots as CSV (x,y,z,r,auxin,cytokinin,axis xyz,stretch per line,
//! one file per snapshot) so that what emerges can be measured and drawn.
//! Usage: cargo run --release -p geno_gpu --example germinate -- <out dir> <steps> <snapshots>
//!        [polar=0.05] [half_sat=4] [production=..] [decay=..] [diffusion=..] [max=50000]
//! The run stops once `max` cells are alive.

use geno_gpu::{Adapter, GpuSim};
use geno_sim::checks::run;
use geno_sim::{Cells, Engine, EngineError, Params};
use std::fmt::Write as _;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let dir = args.get(1).ok_or("missing output directory")?;
    let steps: u32 = args.get(2).map_or(Ok(20_000), |s| s.parse())?;
    let shots: u32 = args.get(3).map_or(Ok(40), |s| s.parse())?;
    let mut p = Params::default();
    let mut max = usize::MAX;
    for arg in args.iter().skip(4) {
        let (key, value) = arg.split_once('=').ok_or("expected key=value")?;
        match key {
            "polar" => p.polar = value.parse()?,
            "half_sat" => p.half_sat = value.parse()?,
            "production" => p.production = value.parse()?,
            "decay" => p.decay = value.parse()?,
            "diffusion" => p.diffusion = value.parse()?,
            "ck_production" => p.ck_production = value.parse()?,
            "ck_diffusion" => p.ck_diffusion = value.parse()?,
            "ck_decay" => p.ck_decay = value.parse()?,
            "ck_half" => p.ck_half = value.parse()?,
            "ck_block" => p.ck_block = value.parse()?,
            "elong" => p.elong = value.parse()?,
            "e_half" => p.e_half = value.parse()?,
            "e_block" => p.e_block = value.parse()?,
            "stretch_max" => p.stretch_max = value.parse()?,
            "align" => p.align = value.parse()?,
            "seed" => p.seed = value.parse()?,
            "max" => max = value.parse()?,
            _ => return Err(format!("unknown parameter {key}").into()),
        }
    }
    let mut sim = GpuSim::new(&p, &Cells::seed(&p, 1 << 17, 1.0), Adapter::Hardware)?;
    println!("{}", sim.machine());
    std::fs::create_dir_all(dir)?;
    for shot in 1..=shots {
        run(&mut sim, steps / shots, Some(p.divide_every));
        let cells = sim.snapshot()?;
        let mut text = String::new();
        for (i, (q, h)) in cells.alive().iter().zip(cells.hormone()).enumerate() {
            let s = cells.shape[i];
            writeln!(
                text,
                "{},{},{},{},{},{},{},{},{},{}",
                q[0], q[1], q[2], q[3], h[0], h[1], s[0], s[1], s[2], s[3]
            )
            .map_err(|e| EngineError(e.to_string()))?;
        }
        std::fs::write(format!("{dir}/{shot:04}.csv"), text)?;
        println!("step {:>6}: {} cells", shot * (steps / shots), cells.count);
        if cells.count >= max {
            break;
        }
    }
    Ok(())
}
