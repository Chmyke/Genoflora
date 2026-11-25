//! Probe: grows one seed cell on the GPU and writes snapshots as CSV, one file per snapshot, so
//! that what emerges can be measured and drawn. Columns: x, y, z, r, auxin, cytokinin, n00, n11,
//! n22, n01, n02, n12, growth direction xyz, pump polarity xyz, youth, light received.
//! Usage: cargo run --release -p geno_gpu --example germinate -- <out dir> <steps> <snapshots>
//!        [polar=0.05] [half_sat=4] [production=..] [decay=..] [diffusion=..] [max=50000] [capacity=131072] [first=1]
//! The run stops once `max` cells are alive.

use geno_gpu::{Adapter, GpuSim};
use geno_sim::checks::{light_on, run};
use geno_sim::{Cells, Engine, EngineError, Params};
use std::fmt::Write as _;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let dir = args.get(1).ok_or("missing output directory")?;
    let steps: u32 = args.get(2).map_or(Ok(20_000), |s| s.parse())?;
    let shots: u32 = args.get(3).map_or(Ok(40), |s| s.parse())?;
    let mut p = Params::default();
    let mut max = usize::MAX;
    let mut capacity = 1 << 17;
    let mut first = 1;
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
            "max_half_length" => p.max_half_length = value.parse()?,
            "align" => p.align = value.parse()?,
            "pin_rate" => p.pin_rate = value.parse()?,
            "pin_switch" => p.pin_switch = value.parse()?,
            "flux_half" => p.flux_half = value.parse()?,
            "gravisense" => p.gravisense = value.parse()?,
            "surface" => p.surface = value.parse()?,
            "soil_grip" => p.soil_grip = value.parse()?,
            "soil_hardness" => p.soil_hardness = value.parse()?,
            "light_production" => p.light_production = value.parse()?,
            "light_depth" => p.light_depth = value.parse()?,
            "youth_rate" => p.youth_rate = value.parse()?,
            "youth_half" => p.youth_half = value.parse()?,
            "ck_light" => p.ck_light = value.parse()?,
            "shade" => p.shade = value.parse()?,
            "squeeze_hold" => p.squeeze_hold = value.parse()?,
            "seed" => p.seed = value.parse()?,
            "max" => max = value.parse()?,
            "capacity" => capacity = value.parse()?,
            "first" => first = value.parse()?,
            _ => return Err(format!("unknown parameter {key}").into()),
        }
    }
    let mut sim = GpuSim::new(&p, &Cells::seed(&p, capacity, 1.0), Adapter::Hardware)?;
    println!("{}", sim.machine());
    std::fs::create_dir_all(dir)?;
    for shot in 1..=shots {
        run(&mut sim, steps / shots, Some(p.divide_every));
        if shot < first {
            continue;
        }
        let cells = sim.snapshot()?;
        let mut text = String::new();
        let light = light_on(&cells, &p);
        for (i, (q, h)) in cells.alive().iter().zip(cells.hormone()).enumerate() {
            let s = cells.shape[i];
            let more = [cells.youth[i], light[i]];
            let cols: Vec<String> = q
                .iter()
                .chain(h)
                .chain(&s[..9])
                .chain(&cells.pin[i][..3])
                .chain(&more)
                .map(|v| v.to_string())
                .collect();
            writeln!(text, "{}", cols.join(",")).map_err(|e| EngineError(e.to_string()))?;
        }
        std::fs::write(format!("{dir}/{shot:04}.csv"), text)?;
        println!("step {:>6}: {} cells", shot * (steps / shots), cells.count);
        if cells.count >= max {
            break;
        }
    }
    Ok(())
}
