//! Probe: grows one seed cell on the GPU and prints, at every snapshot, what the young foci in the
//! air look like: how many, how big, and for the highest one how youth and hormones change from
//! its centre to its edge (does the centre mature before the edge?).
//! Usage: cargo run --release -p geno_gpu --example tip -- <steps> <snapshots>
//!        [capacity=16384] [profile=0] [any parameter of the genome, as name=value]
//! `profile=1` prints the centre-to-edge profile at every snapshot, not only at the last.

use geno_gpu::{Adapter, GpuSim};
use geno_sim::checks::{foci, profile, run};
use geno_sim::{Cells, Engine, Params};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let steps: u32 = args.get(1).map_or(Ok(30_000), |s| s.parse())?;
    let shots: u32 = args.get(2).map_or(Ok(30), |s| s.parse())?;
    let mut p = Params::default();
    let mut capacity = 1 << 14;
    let mut every_profile = false;
    for arg in args.iter().skip(3) {
        let (key, value) = arg.split_once('=').ok_or("expected key=value")?;
        match key {
            "capacity" => capacity = value.parse()?,
            "profile" => every_profile = value != "0",
            _ => p.set(key, value)?,
        }
    }
    let mut sim = GpuSim::new(&p, &Cells::seed(&p, capacity, 1.0), Adapter::Hardware)?;
    println!("step cells air height foci biggest top_size top_height top_off_axis");
    for shot in 1..=shots {
        run(&mut sim, steps / shots, Some(p.divide_every));
        let cells = sim.snapshot()?;
        let in_air = |q: &[f32; 4]| geno_sim::soil::share(&p, &q[..3]) < 0.5;
        let air: Vec<&[f32; 4]> = cells.alive().iter().filter(|q| in_air(q)).collect();
        let height = air.iter().map(|q| q[1]).fold(p.surface, f32::max) - p.surface;
        let n = air.len().max(1) as f32;
        let axis = [0, 2].map(|k| air.iter().map(|q| q[k]).sum::<f32>() / n);
        let found = foci(&cells, &p, in_air);
        let top = found
            .iter()
            .filter(|f| f.cells >= 3)
            .max_by(|a, b| a.centre[1].total_cmp(&b.centre[1]));
        let (size, at, off) = top.map_or((0, 0.0, 0.0), |f| {
            (
                f.cells,
                f.centre[1] - p.surface,
                (f.centre[0] - axis[0]).hypot(f.centre[2] - axis[1]),
            )
        });
        println!(
            "{:>6} {:>6} {:>5} {height:>5.1} {:>3} {:>4} {size:>4} {at:>5.1} {off:>5.1}",
            shot * (steps / shots),
            cells.count,
            air.len(),
            found.iter().filter(|f| f.cells >= 6).count(),
            found.first().map_or(0, |f| f.cells),
        );
        let full = cells.count >= capacity * 97 / 100;
        if let Some(f) = top.filter(|_| every_profile || shot == shots || full) {
            println!(
                "  ring lit young auxin cytokinin restraint (rings one radius wide around the top focus)"
            );
            for (ring, r) in profile(&cells, f.centre, 8).iter().enumerate() {
                println!(
                    "  {ring:>4} {:>3} {:>5.2} {:>5.2} {:>9.2} {:>9.3}",
                    r.cells, r.young, r.auxin, r.cytokinin, r.restraint
                );
            }
        }
        if full {
            break;
        }
    }
    Ok(())
}
