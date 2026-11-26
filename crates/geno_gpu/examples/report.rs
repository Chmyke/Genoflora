//! Probe: grows one seed cell on the GPU and prints, at every snapshot, the numbers that tell
//! whether it looks like a plant: how slender the stem and the root are, how many young foci the
//! air holds and how big, how much younger the young cells are than the others (a tip that only
//! stands a hair above the dividing threshold is no tip), and for the highest focus how youth and
//! hormones change from its centre to its edge; then the same, layer by layer, going down under
//! the highest cell and across the stem at three depths (where the relay is made, where it is
//! found, where the cells are young: a tip is a place, a stem held at the threshold is not).
//! Under each line: the silhouette of the stem and of the root in slices, and whether the mature
//! cells of the stem kept their neighbours (cells of a plant never slide past each other).
//! Last, the root across at four heights above its tip: youth, auxin and which way the auxin
//! pumps point (in a real root auxin goes down the middle and back up the outside).
//! Columns: step, cells, cells in the air, stem height, stem width (twice the root mean square
//! distance to its axis), root depth, root width, foci of six cells or more, biggest focus, top
//! focus (size, height, distance to the axis), mean youth of the young and of the other cells
//! in the air.
//! Usage: cargo run --release -p geno_gpu --example report -- <steps> <snapshots>
//!        [capacity=16384] [profile=0] [any parameter of the genome, as name=value]
//! `profile=1` prints the centre-to-edge profile at every snapshot, not only at the last.

use geno_gpu::{Adapter, GpuSim};
use geno_sim::checks::{
    Layer, across, foci, kept_neighbours, length_over_width, long_axis, profile, run, under,
};
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
        // Do the mature cells of the stem slide past each other? Their neighbours a little before
        // the snapshot, against their neighbours at the snapshot.
        let watched = SLIDE_STEPS.min(steps / shots);
        run(&mut sim, steps / shots - watched, Some(p.divide_every));
        let earlier = sim.snapshot()?;
        run(&mut sim, watched, Some(p.divide_every));
        let cells = sim.snapshot()?;
        let mature_stem = |c: &Cells, i: usize| {
            geno_sim::soil::share(&p, &c.pr[i][..3]) < 0.5 && c.youth[i] <= geno_sim::params::YOUNG
        };
        let kept = kept_neighbours(&earlier, &cells, &p, mature_stem);
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
        // The silhouette, in equal slices of the stem (from the soil up) and of the root (from the
        // soil down): width, share of young cells, mean cell radius, how much longer than wide
        // the cells are and how upright their long axis lies (1: along gravity).
        let slices = |from: f32, span: f32| {
            let mut out = String::new();
            for k in 0..SLICES {
                // How far along the organ a cell lies, from 0 at the soil surface to 1 at its end.
                let inside = |q: &[f32; 4]| {
                    let along = (q[1] - from) / span * SLICES as f32;
                    along >= k as f32 && along < (k + 1) as f32
                };
                let part: Vec<&[f32; 4]> = cells.alive().iter().filter(|q| inside(q)).collect();
                let members: Vec<usize> = (0..cells.count).filter(|&i| inside(&cells.pr[i])).collect();
                let n = members.len().max(1) as f32;
                let mean = |of: &dyn Fn(usize) -> f32| members.iter().map(|&i| of(i)).sum::<f32>() / n;
                let upright = |i: usize| {
                    long_axis(&cells.shape[i]).map_or(0.0, |a| {
                        (0..3).map(|k| a[k] as f32 * p.gravity[k]).sum::<f32>().abs()
                    })
                };
                out += &format!(
                    " {:>4.1}/{:<4.2}/{:<4.2}/{:<3.1}/{:<4.2}",
                    width(&part),
                    mean(&|i| f32::from(u8::from(cells.youth[i] > geno_sim::params::YOUNG))),
                    mean(&|i| cells.pr[i][3]),
                    mean(&|i| length_over_width(&cells.shape[i]) as f32),
                    mean(&upright),
                );
            }
            out
        };
        println!(
            "  stem from the soil up, width/young/radius/length over width/upright:{}",
            slices(p.surface, height + 0.01)
        );
        println!(
            "  root from the soil down, width/young/radius/length over width/upright:{}",
            slices(p.surface, -(depth + 0.01))
        );
        println!("  mature cells of the stem kept {kept:.2} of their neighbours over {watched} steps");
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
        let tip = soil.iter().min_by(|a, b| a[1].total_cmp(&b[1]));
        if let Some(q) = tip.filter(|_| every_profile || shot == shots || full) {
            for above in [ROOT_NEAR, ROOT_MID, ROOT_FAR, 0.5 * depth] {
                println!("  across the root {above:.0} above its tip, from its axis outward");
                print_root_cut(&cells, &p, q[1] + above);
            }
        }
        if full {
            break;
        }
    }
    Ok(())
}

/// Heights above the root tip of the slabs read there (a fourth is read at mid-depth).
const ROOT_NEAR: f32 = 1.0;
const ROOT_MID: f32 = 4.0;
const ROOT_FAR: f32 = 10.0;

/// The root across, at height `at`: the cells of a slab, sorted by their distance to its centre,
/// one dividing radius per layer. For each layer: how young, how much auxin, and which way the
/// auxin pumps point, upward (against gravity, 1: straight up, -1: straight down) and outward
/// (away from the axis, -1: straight toward it). A reverse fountain shows as pumps pointing down
/// at the axis, up in the outer layers, and inward in between.
fn print_root_cut(cells: &Cells, p: &Params, at: f32) {
    let slab: Vec<usize> = (0..cells.count)
        .filter(|&i| (cells.pr[i][1] - at).abs() <= 0.5 * ACROSS_THICK)
        .collect();
    let n = slab.len().max(1) as f32;
    let mid = [0, 2].map(|k| slab.iter().map(|&i| cells.pr[i][k]).sum::<f32>() / n);
    println!("  layer cells youth auxin pumps_up pumps_out");
    for layer in 0..ACROSS_LAYERS {
        let (mut count, mut youth, mut auxin, mut up, mut out) = (0.0_f32, 0.0, 0.0, 0.0, 0.0);
        for &i in &slab {
            let q = &cells.pr[i];
            let aside = [q[0] - mid[0], q[2] - mid[1]];
            let far = (aside[0] * aside[0] + aside[1] * aside[1]).sqrt();
            if (far / p.r_div) as usize != layer {
                continue;
            }
            let pin = &cells.pin[i];
            count += 1.0;
            youth += cells.youth[i];
            auxin += cells.h[i][0] / (q[3] * q[3] * q[3]);
            up -= (0..3).map(|k| pin[k] * p.gravity[k]).sum::<f32>();
            out += (pin[0] * aside[0] + pin[2] * aside[1]) / far.max(1e-6);
        }
        if count > 0.0 {
            println!(
                "  {layer:>5} {count:>5.0} {:>5.2} {:>5.2} {:>8.2} {:>9.2}",
                youth / count,
                auxin / count,
                up / count,
                out / count
            );
        }
    }
}

/// Steps over which the neighbours of the mature cells are followed before every snapshot.
const SLIDE_STEPS: u32 = 1000;
/// Slices of the stem and of the root whose width is printed.
const SLICES: usize = 6;
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
