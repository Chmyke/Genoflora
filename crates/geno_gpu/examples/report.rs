//! Probe: grows one seed cell on the GPU and prints, at every snapshot, the numbers that tell
//! whether it looks like a plant: how slender the stem and the root are, how many young foci the
//! air holds and how big, how much younger the young cells are than the others (a tip that only
//! stands a hair above the dividing threshold is no tip), and for the highest focus how youth and
//! hormones change from its centre to its edge; then the same, layer by layer, going down under
//! the highest cell and across the stem at three depths (where the relay is made, where it is
//! found, where the cells are young: a tip is a place, a stem held at the threshold is not).
//! Columns: step, cells, cells in the air, stem height, stem width (twice the root mean square
//! distance to its axis), root depth, root width, foci of six cells or more, biggest focus, top
//! focus (size, height, distance to the axis), mean youth of the young and of the other cells
//! in the air.
//! Usage: cargo run --release -p geno_gpu --example report -- <steps> <snapshots>
//!        [capacity=16384] [profile=0] [any parameter of the genome, as name=value]
//! `profile=1` prints the centre-to-edge profile at every snapshot, not only at the last.

use geno_gpu::{Adapter, GpuSim};
use geno_sim::checks::{Layer, across, foci, profile, run, under};
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
    println!(
        "step cells air height width root_depth root_width foci biggest top_size top_height top_off_axis youth_young youth_other"
    );
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
        // Twice the root mean square distance of some cells to their own vertical axis.
        let width = |part: &[&[f32; 4]]| {
            let n = part.len().max(1) as f32;
            let mid = [0, 2].map(|k| part.iter().map(|q| q[k]).sum::<f32>() / n);
            let sum: f32 = part
                .iter()
                .map(|q| (q[0] - mid[0]).powi(2) + (q[2] - mid[1]).powi(2))
                .sum();
            2.0 * (sum / n).sqrt()
        };
        let soil: Vec<&[f32; 4]> = cells.alive().iter().filter(|q| !in_air(q)).collect();
        let depth = p.surface - soil.iter().map(|q| q[1]).fold(p.surface, f32::min);
        let mean_youth = |young: bool| {
            let kept: Vec<f32> = (0..cells.count)
                .filter(|&i| in_air(&cells.pr[i]) && (cells.youth[i] > geno_sim::params::YOUNG) == young)
                .map(|i| cells.youth[i])
                .collect();
            kept.iter().sum::<f32>() / kept.len().max(1) as f32
        };
        println!(
            "{:>6} {:>6} {:>5} {height:>5.1} {:>5.1} {depth:>5.1} {:>5.1} {:>3} {:>4} {size:>4} {at:>5.1} {off:>5.1} {:>4.2} {:>4.2}",
            shot * (steps / shots),
            cells.count,
            air.len(),
            width(&air),
            width(&soil),
            found.iter().filter(|f| f.cells >= 6).count(),
            found.first().map_or(0, |f| f.cells),
            mean_youth(true),
            mean_youth(false),
        );
        let full = cells.count >= capacity * 97 / 100;
        if let Some(f) = top.filter(|_| every_profile || shot == shots || full) {
            println!(
                "  ring lit young auxin cytokinin restraint relay (rings one radius wide around the top focus)"
            );
            for (ring, r) in profile(&cells, f.centre, 8).iter().enumerate() {
                println!(
                    "  {ring:>4} {:>3} {:>5.2} {:>5.2} {:>9.2} {:>9.3} {:>5.2}",
                    r.cells, r.young, r.auxin, r.cytokinin, r.restraint, r.relay
                );
            }
        }
        let summit = air.iter().max_by(|a, b| a[1].total_cmp(&b[1]));
        if let Some(q) = summit.filter(|_| every_profile || shot == shots || full) {
            let top = [q[0], q[1], q[2]];
            println!("  under the summit, going down (cells within {UNDER_REACH} of its vertical)");
            print_layers(&under(&cells, &p, top, UNDER_REACH, UNDER_LAYERS));
            for depth in [ACROSS_NEAR, ACROSS_FAR, 0.5 * height] {
                println!("  across the stem {depth:.0} under the summit, from its axis outward");
                print_layers(&across(&cells, &p, top, depth, ACROSS_THICK, ACROSS_LAYERS));
            }
        }
        if full {
            break;
        }
    }
    Ok(())
}

/// Half-width of the column read under the summit, and how deep it is read.
const UNDER_REACH: f32 = 2.0;
const UNDER_LAYERS: usize = 24;
/// Depths under the summit of the two slabs read near the tip (a third is read at mid-height),
/// their thickness, and how far from the axis they are read.
const ACROSS_NEAR: f32 = 4.0;
const ACROSS_FAR: f32 = 12.0;
const ACROSS_THICK: f32 = 2.0;
const ACROSS_LAYERS: usize = 20;

fn print_layers(layers: &[Layer]) {
    println!("  layer cells youth light skin auxin cytokinin restraint relay making");
    for (k, l) in layers.iter().enumerate().filter(|(_, l)| l.cells > 0) {
        println!(
            "  {k:>5} {:>5} {:>5.2} {:>5.2} {:>4.2} {:>5.2} {:>9.3} {:>9.3} {:>5.2} {:>6.3}",
            l.cells, l.youth, l.light, l.skin, l.auxin, l.cytokinin, l.restraint, l.relay, l.making
        );
    }
}
