//! Probe: grows one seed cell on the GPU and writes snapshots as CSV, one file per snapshot, so
//! that what emerges can be measured and drawn. Columns: x, y, z, r, auxin, cytokinin, restraint
//! signal, n00, n11, n22, n01, n02, n12, growth direction xyz, pump polarity xyz, youth, light received.
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
            "max" => max = value.parse()?,
            "capacity" => capacity = value.parse()?,
            "first" => first = value.parse()?,
            _ => p.set(key, value)?,
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
