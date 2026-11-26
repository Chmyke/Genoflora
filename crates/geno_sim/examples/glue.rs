//! Probe of the prototype "cells keep their neighbours", on the judge: grows one seed cell with
//! or without bonds and prints, at every snapshot, whether the stem lengthens or widens and
//! whether its mature cells kept their neighbours.
//! Columns: step, cells, cells in the air, stem height, stem width (twice the root mean square
//! distance to its axis), root depth, root width, share of their neighbours the mature cells of
//! the stem kept over the last steps, mean bonds per cell, bonds lost for want of a free slot,
//! share of the bonds whose two cells no longer touch, mean pressure on a cell (the sum of how
//! deep its neighbours overlap it, in radii: growth stops when it reaches `1 / squeeze_hold`).
//! Usage: cargo run --release -p geno_sim --example glue -- <steps> <snapshots> <bond strength>
//!        [capacity=2048] [touch=1.2] [any parameter of the genome, as name=value]
//! A bond strength of 0 runs the plain judge (cells stick to whatever they touch). `touch`: how
//! close a neighbour must lie to a daughter to stay bonded to it (see `bonds`).

use geno_sim::checks::{kept_neighbours, run};
use geno_sim::rules::{Gathered, interact};
use geno_sim::{Cells, Engine, Params, Reference};

/// Steps over which the neighbours of the mature cells are followed before every snapshot.
const SLIDE_STEPS: u32 = 1000;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let steps: u32 = args.get(1).map_or(Ok(20_000), |s| s.parse())?;
    let shots: u32 = args.get(2).map_or(Ok(10), |s| s.parse())?;
    let strength: f32 = args.get(3).map_or(Ok(0.0), |s| s.parse())?;
    let mut p = Params::default();
    let mut capacity = 1 << 11;
    let mut touch = p.reach;
    for arg in args.iter().skip(4) {
        let (key, value) = arg.split_once('=').ok_or("expected key=value")?;
        match key {
            "capacity" => capacity = value.parse()?,
            "touch" => touch = value.parse()?,
            _ => p.set(key, value)?,
        }
    }
    let seed = Cells::seed(&p, capacity, 1.0);
    let mut sim = if strength > 0.0 {
        Reference::glued(&p, &seed, strength, touch)
    } else {
        Reference::new(&p, &seed)
    };
    println!("step cells air height width root_depth root_width kept bonds dropped apart pressure");
    for shot in 1..=shots {
        let watched = SLIDE_STEPS.min(steps / shots);
        run(&mut sim, steps / shots - watched, Some(p.divide_every));
        let earlier = sim.snapshot()?;
        run(&mut sim, watched, Some(p.divide_every));
        let cells = sim.snapshot()?;
        let in_air = |q: &[f32; 4]| geno_sim::soil::share(&p, &q[..3]) < 0.5;
        let mature_stem = |c: &Cells, i: usize| in_air(&c.pr[i]) && c.youth[i] <= geno_sim::params::YOUNG;
        let kept = kept_neighbours(&earlier, &cells, &p, mature_stem);
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
        let air: Vec<&[f32; 4]> = cells.alive().iter().filter(|q| in_air(q)).collect();
        let soil: Vec<&[f32; 4]> = cells.alive().iter().filter(|q| !in_air(q)).collect();
        let height = air.iter().map(|q| q[1]).fold(p.surface, f32::max) - p.surface;
        let depth = p.surface - soil.iter().map(|q| q[1]).fold(p.surface, f32::min);
        let (bonds, dropped) = sim.bonds().map_or((0.0, 0), |b| (b.mean(cells.count), b.dropped));
        // Bonded cells that no longer touch, and how hard the cells are pressed.
        let (mut apart, mut all, mut pressure) = (0, 0, 0.0);
        for i in 0..cells.count {
            let me = cells.get(i);
            let mut g = Gathered::default();
            for j in (0..cells.count).filter(|&j| j != i) {
                interact(&me, &cells.get(j), &p, &mut g);
            }
            pressure += g.squeeze / me.pr[3];
            for j in sim.bonds().into_iter().flat_map(|b| b.of(i)) {
                let mut pair = Gathered::default();
                interact(&me, &cells.get(j), &p, &mut pair);
                all += 1;
                apart += usize::from(pair.neighbours == 0.0);
            }
        }
        let apart = apart as f32 / all.max(1) as f32;
        let pressure = pressure / cells.count.max(1) as f32;
        println!(
            "{:>6} {:>5} {:>5} {height:>5.1} {:>5.1} {depth:>5.1} {:>5.1} {kept:>4.2} {bonds:>4.1} {dropped} {apart:>4.2} {pressure:>4.2}",
            shot * (steps / shots),
            cells.count,
            air.len(),
            width(&air),
            width(&soil),
        );
        if cells.count >= capacity * 97 / 100 {
            break;
        }
    }
    Ok(())
}
