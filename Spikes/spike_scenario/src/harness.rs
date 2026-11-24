//! Validation (V1-V5) and bench protocols, identical for every engine, plus the result files.

use crate::model::{Cells, Params, init};
use crate::reference::Reference;
use std::time::Instant;

/// What a spike must provide. `step` and `divide` may only queue work; `flush` waits until it is
/// really done (GPU included), so timings are end to end.
pub trait Engine {
    fn step(&mut self);
    fn divide(&mut self);
    fn flush(&mut self);
    fn snapshot(&mut self) -> Cells;
}

pub type Make<'a> = &'a dyn Fn(&Params, &Cells) -> Box<dyn Engine>;

/// Runs `steps` steps, divisions every K steps when `divide`, flushing every `batch` steps.
pub fn run(e: &mut dyn Engine, steps: u32, divide: bool, p: &Params, batch: u32) {
    for s in 1..=steps {
        e.step();
        if divide && s % p.divide_every == 0 {
            e.divide();
        }
        if s % batch == 0 {
            e.flush();
        }
    }
    e.flush();
}

fn mass(c: &Cells) -> f64 {
    c.h[..c.count].iter().map(|&h| h as f64).sum()
}

fn volume(c: &Cells) -> f64 {
    c.pr[..c.count].iter().map(|q| (q[3] as f64).powi(3)).sum()
}

fn mean_radius(c: &Cells) -> f64 {
    c.pr[..c.count].iter().map(|q| q[3] as f64).sum::<f64>() / c.count as f64
}

/// Largest position gap between two states, in units of the mean radius.
fn position_gap(a: &Cells, b: &Cells) -> f64 {
    let mut worst = 0.0f64;
    for i in 0..a.count.min(b.count) {
        let d2: f64 = (0..3).map(|k| ((a.pr[i][k] - b.pr[i][k]) as f64).powi(2)).sum();
        worst = worst.max(d2.sqrt());
    }
    worst / mean_radius(a)
}

/// Largest overlap between two cells, as a fraction of the smaller radius.
fn worst_overlap(c: &Cells) -> f64 {
    let mut worst = 0.0f64;
    for i in 0..c.count {
        for j in i + 1..c.count {
            let (a, b) = (c.pr[i], c.pr[j]);
            let d2: f32 = (0..3).map(|k| (a[k] - b[k]).powi(2)).sum();
            let overlap = (a[3] + b[3] - d2.sqrt()) / a[3].min(b[3]);
            worst = worst.max(overlap as f64);
        }
    }
    worst
}

struct Report {
    rows: Vec<(String, bool, String)>,
}

impl Report {
    fn add(&mut self, code: &str, ok: bool, detail: String) {
        println!("{} {code:<4} {detail}", if ok { "OK  " } else { "FAIL" });
        self.rows.push((code.to_string(), ok, detail));
    }
}

/// Radius of gyration of the tissue: a shape measure that does not depend on which cell is where.
fn gyration(c: &Cells) -> f64 {
    let n = c.count as f64;
    let centre: Vec<f64> = (0..3).map(|k| c.pr[..c.count].iter().map(|q| q[k] as f64).sum::<f64>() / n).collect();
    let sum: f64 = c.pr[..c.count].iter().map(|q| (0..3).map(|k| (q[k] as f64 - centre[k]).powi(2)).sum::<f64>()).sum();
    (sum / n).sqrt()
}

/// Same run on the oracle and on the engine. `exact`: cell-by-cell position gap (short horizon).
/// Otherwise statistical: a dividing tissue amplifies rounding differences exponentially, so over
/// a long horizon only the cell count, the shape and the hormone mass can be compared.
fn against_reference(make: Make, p: &Params, n: usize, capacity: usize, steps: u32, exact: bool) -> (bool, String) {
    let start = init(n, capacity, p);
    let mut oracle = Reference::new(p, &start);
    run(&mut oracle, steps, true, p, 10);
    let want = oracle.snapshot();
    let mut engine = make(p, &start);
    run(engine.as_mut(), steps, true, p, 10);
    let got = engine.snapshot();
    let gap = position_gap(&want, &got);
    let shape = ((gyration(&want) - gyration(&got)) / gyration(&want)).abs();
    let mass_gap = ((mass(&want) - mass(&got)) / mass(&want)).abs();
    let ok = want.count == got.count && mass_gap <= 1e-4 && if exact { gap <= 1e-3 } else { shape <= 1e-3 };
    let head = format!("{n} cells, cap {capacity}, {steps} steps: count {} vs ref {}", got.count, want.count);
    let tail = format!("hormone mass gap {mass_gap:.2e} (max 1e-4)");
    if exact {
        (ok, format!("{head}, position gap {gap:.2e} r (max 1e-3), {tail}"))
    } else {
        (ok, format!("{head}, shape gap {shape:.2e} (max 1e-3), position gap {gap:.2e} r (informative), {tail}"))
    }
}

/// Runs V1 to V5. `strict_bits`: V5 must be bit-identical (CPU); otherwise it is only reported.
pub fn validate(name: &str, machine: &str, make: Make, strict_bits: bool) -> bool {
    let p = Params::load();
    let k = p.divide_every;
    let mut rep = Report { rows: Vec::new() };
    println!("== validate {name} ({machine})");

    let (ok, detail) = against_reference(make, &p, 1000, 8192, 200, true);
    rep.add("V1a", ok, format!("exact, short horizon. {detail}"));
    let (ok, detail) = against_reference(make, &p, 1000, 1500, 300, true);
    rep.add("V1b", ok, format!("capacity ceiling. {detail}"));
    let (ok, detail) = against_reference(make, &p, 1000, 8192, 600, false);
    rep.add("V1c", ok, format!("statistical, long horizon. {detail}"));

    let mut closed = p;
    closed.production = 0.0;
    closed.decay = 0.0;
    let start = init(1000, 1000, &closed);
    let mut e = make(&closed, &start);
    run(e.as_mut(), 1000, false, &closed, 10);
    let drift = ((mass(&e.snapshot()) - mass(&start)) / mass(&start)).abs();
    rep.add("V2", drift <= 1e-5, format!("hormone mass drift over 1000 steps: {drift:.2e} (max 1e-5)"));

    let start = init(1000, 2048, &p);
    let mut e = make(&p, &start);
    run(e.as_mut(), k, false, &p, 10);
    let before = e.snapshot();
    e.divide();
    e.flush();
    let after = e.snapshot();
    let vol = ((volume(&after) - volume(&before)) / volume(&before)).abs();
    let mas = ((mass(&after) - mass(&before)) / mass(&before)).abs();
    rep.add(
        "V3",
        vol <= 1e-5 && mas <= 1e-5 && after.count > before.count,
        format!("division {} -> {} cells: volume gap {vol:.2e}, hormone gap {mas:.2e} (max 1e-5)", before.count, after.count),
    );

    let start = init(1000, 1000, &p);
    let mut e = make(&p, &start);
    run(e.as_mut(), 300, false, &p, 10);
    let overlap = worst_overlap(&e.snapshot());
    rep.add("V4", overlap <= 0.2, format!("worst overlap after 300 steps: {:.1} % of radius (max 20 %)", overlap * 100.0));

    let start = init(1000, 4096, &p);
    let twice: Vec<Cells> = (0..2)
        .map(|_| {
            let mut e = make(&p, &start);
            run(e.as_mut(), 300, true, &p, 10);
            e.snapshot()
        })
        .collect();
    let same = twice[0] == twice[1];
    let note = if strict_bits { "required" } else { "wished, not required" };
    rep.add("V5", same || !strict_bits, format!("two runs bit-identical: {same} ({note})"));

    let all = rep.rows.iter().all(|r| r.1);
    println!("VALIDATION {name}: {}", if all { "PASS" } else { "FAIL" });
    let rows: Vec<String> = rep
        .rows
        .iter()
        .map(|(c, ok, d)| format!("    {{\"check\": \"{c}\", \"ok\": {ok}, \"detail\": \"{}\"}}", d.replace('"', "'")))
        .collect();
    write_result(
        &format!("{name}-validate"),
        &format!("{{\n  \"engine\": \"{name}\",\n  \"machine\": \"{machine}\",\n  \"pass\": {all},\n  \"bits_identical\": {same},\n  \"checks\": [\n{}\n  ]\n}}\n", rows.join(",\n")),
    );
    all
}

/// Diagnostic: how the gap to the reference grows with time (sensitivity to rounding).
pub fn diverge(make: Make, divide: bool) {
    let p = Params::load();
    let start = init(1000, 8192, &p);
    let mut oracle = Reference::new(&p, &start);
    let mut engine = make(&p, &start);
    for s in 1..=600u32 {
        oracle.step();
        engine.step();
        if divide && s % p.divide_every == 0 {
            oracle.divide();
            engine.divide();
        }
        if [1, 2, 5, 10, 20, 21, 40, 60, 100, 150, 200, 300, 400, 500, 600].contains(&s) {
            engine.flush();
            let (a, b) = (oracle.snapshot(), engine.snapshot());
            println!("step {s:>4}: {} cells, position gap {:.2e} r", a.count, position_gap(&a, &b));
        }
    }
}

/// Bench protocol of section 5. Stops growing the population once a step costs over 150 ms.
pub fn bench(name: &str, machine: &str, make: Make, max_cells: usize) {
    let p = Params::load();
    println!("== bench {name} ({machine})");
    let mut rows = Vec::new();
    for n in [1_000usize, 10_000, 100_000, 300_000, 1_000_000, 2_000_000, 4_000_000, 8_000_000, 16_000_000] {
        if n > max_cells {
            break;
        }
        let start = init(n, n, &p);
        let mut e = make(&p, &start);
        run(e.as_mut(), 50, false, &p, 1);
        let mut times: Vec<f64> = (0..200)
            .map(|_| {
                let t = Instant::now();
                e.step();
                e.flush();
                t.elapsed().as_secs_f64() * 1e3
            })
            .collect();
        times.sort_by(|a, b| a.total_cmp(b));
        let (median, p95) = (times[100], times[189]);
        let t = Instant::now();
        run(e.as_mut(), 200, false, &p, 10);
        let batched = t.elapsed().as_secs_f64() * 1e3 / 200.0;
        println!("{n:>9} cells: median {median:8.3} ms, p95 {p95:8.3} ms, batched {batched:8.3} ms/step");
        rows.push(format!(
            "    {{\"cells\": {n}, \"median_ms\": {median:.4}, \"p95_ms\": {p95:.4}, \"batched_ms\": {batched:.4}}}"
        ));
        if median > 150.0 {
            break;
        }
    }
    let (steps, cap) = (1600u32, 100_000usize);
    let start = init(1000, cap, &p);
    let mut e = make(&p, &start);
    let t = Instant::now();
    run(e.as_mut(), steps, true, &p, 10);
    let seconds = t.elapsed().as_secs_f64();
    let reached = e.snapshot().count;
    println!("growth 1000 -> {reached} cells in {steps} steps: {seconds:.3} s");

    // Cost of one division round at scale: 5 rounds on a tissue of one million cells and more.
    let big = 1_000_000usize.min(max_cells);
    let start = init(big, big * 2, &p);
    let mut e = make(&p, &start);
    let mut rounds: Vec<f64> = (0..5)
        .map(|_| {
            run(e.as_mut(), p.divide_every, false, &p, 10);
            let t = Instant::now();
            e.divide();
            e.flush();
            t.elapsed().as_secs_f64() * 1e3
        })
        .collect();
    rounds.sort_by(|a, b| a.total_cmp(b));
    let divide_ms = rounds[2];
    let divided = e.snapshot().count;
    println!("division round at {big} cells (5 rounds, now {divided} cells): median {divide_ms:.3} ms");
    write_result(
        &format!("{name}-bench"),
        &format!(
            "{{\n  \"engine\": \"{name}\",\n  \"machine\": \"{machine}\",\n  \"steady\": [\n{}\n  ],\n  \"growth\": {{\"steps\": {steps}, \"capacity\": {cap}, \"final_cells\": {reached}, \"seconds\": {seconds:.4}}},
  \"division\": {{\"cells\": {big}, \"cells_after_5_rounds\": {divided}, \"median_ms\": {divide_ms:.4}}}
}}
",
            rows.join(",\n")
        ),
    );
}

fn write_result(file: &str, json: &str) {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../Résultats");
    let path = dir.join(format!("{file}.json"));
    std::fs::write(&path, json).unwrap_or_else(|e| panic!("cannot write {}: {e}", path.display()));
}

/// Shared command line: `validate`, `bench [max_cells]`. Exit code 1 when validation fails.
pub fn cli(name: &str, machine: &str, make: Make, strict_bits: bool) {
    let args: Vec<String> = std::env::args().skip(1).filter(|a| !a.starts_with("--")).collect();
    match args.first().map(String::as_str) {
        Some("validate") => {
            if !validate(name, machine, make, strict_bits) {
                std::process::exit(1);
            }
        }
        Some("bench") => {
            let max = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(usize::MAX);
            bench(name, machine, make, max);
        }
        Some("diverge") => diverge(make, args.get(1).map(String::as_str) != Some("nodiv")),
        _ => {
            eprintln!("usage: spike_{name} validate | bench [max_cells]");
            std::process::exit(2);
        }
    }
}
