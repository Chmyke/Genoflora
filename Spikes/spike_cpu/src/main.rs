//! CPU-only spike: uniform grid rebuilt every step, rayon over fixed-size blocks.
//! Each cell gathers its own neighbours in a fixed order, so results do not depend on threads.

use rayon::prelude::*;
use spike_scenario::{Cells, Engine, Grid, Params, cli, divide, finish, interact};

const BLOCK: usize = 512;

struct CpuEngine {
    p: Params,
    grid: Grid,
    cur: Cells,
    next: Cells,
    epoch: u32,
    keys: Vec<u64>,
    sorted_pr: Vec<[f32; 4]>,
    sorted_h: Vec<f32>,
    sorted_id: Vec<u32>,
    offsets: Vec<u32>,
}

impl CpuEngine {
    fn new(p: &Params, cells: &Cells) -> Self {
        let cap = cells.capacity();
        let grid = Grid::for_capacity(cap, p);
        Self {
            p: *p,
            grid,
            cur: cells.clone(),
            next: cells.clone(),
            epoch: 0,
            keys: vec![0; cap],
            sorted_pr: vec![[0.0; 4]; cap],
            sorted_h: vec![0.0; cap],
            sorted_id: vec![0; cap],
            offsets: vec![0; grid.cells() + 1],
        }
    }

    /// Sorts the cells by grid cell (then by id) and records where each grid cell starts.
    fn build_grid(&mut self) {
        let n = self.cur.count;
        let grid = self.grid;
        let pr = &self.cur.pr;
        self.keys[..n].par_chunks_mut(BLOCK).enumerate().for_each(|(b, chunk)| {
            for (k, key) in chunk.iter_mut().enumerate() {
                let i = b * BLOCK + k;
                *key = ((grid.index(grid.coord(&pr[i])) as u64) << 32) | i as u64;
            }
        });
        self.keys[..n].par_sort_unstable();
        let (keys, h) = (&self.keys, &self.cur.h);
        (self.sorted_pr[..n].par_chunks_mut(BLOCK), self.sorted_h[..n].par_chunks_mut(BLOCK), self.sorted_id[..n].par_chunks_mut(BLOCK))
            .into_par_iter()
            .enumerate()
            .for_each(|(b, (spr, sh, sid))| {
                for k in 0..spr.len() {
                    let i = keys[b * BLOCK + k] as u32;
                    spr[k] = pr[i as usize];
                    sh[k] = h[i as usize];
                    sid[k] = i;
                }
            });
        let mut cell = 0usize;
        for (k, key) in self.keys[..n].iter().enumerate() {
            let c = (key >> 32) as usize;
            while cell <= c {
                self.offsets[cell] = k as u32;
                cell += 1;
            }
        }
        self.offsets[cell..].fill(n as u32);
    }
}

impl Engine for CpuEngine {
    fn step(&mut self) {
        self.build_grid();
        let n = self.cur.count;
        let (p, grid) = (self.p, self.grid);
        let (pr, h) = (&self.cur.pr, &self.cur.h);
        let (spr, sh, sid, offsets) = (&self.sorted_pr, &self.sorted_h, &self.sorted_id, &self.offsets);
        let g1 = grid.g1 as i32;
        (self.next.pr[..n].par_chunks_mut(BLOCK), self.next.h[..n].par_chunks_mut(BLOCK)).into_par_iter().enumerate().for_each(
            |(b, (out_pr, out_h))| {
                for k in 0..out_pr.len() {
                    let i = b * BLOCK + k;
                    let (pi, hi) = (pr[i], h[i]);
                    let c = grid.coord(&pi).map(|v| v as i32);
                    let (mut disp, mut hs) = ([0.0f32; 3], 0.0f32);
                    for z in (c[2] - 1).max(0)..=(c[2] + 1).min(g1 - 1) {
                        for y in (c[1] - 1).max(0)..=(c[1] + 1).min(g1 - 1) {
                            for x in (c[0] - 1).max(0)..=(c[0] + 1).min(g1 - 1) {
                                let cell = grid.index([x as u32, y as u32, z as u32]) as usize;
                                for s in offsets[cell] as usize..offsets[cell + 1] as usize {
                                    if sid[s] as usize != i {
                                        interact(&pi, hi, &spr[s], sh[s], &p, &mut disp, &mut hs);
                                    }
                                }
                            }
                        }
                    }
                    (out_pr[k], out_h[k]) = finish(&pi, hi, &disp, hs, &p);
                }
            },
        );
        self.next.count = n;
        std::mem::swap(&mut self.cur, &mut self.next);
    }

    fn divide(&mut self) {
        divide(&mut self.cur, &self.p, self.epoch);
        self.epoch += 1;
    }

    fn flush(&mut self) {}

    fn snapshot(&mut self) -> Cells {
        self.cur.clone()
    }
}

fn main() {
    let machine = format!("{} threads", rayon::current_num_threads());
    cli("cpu", &machine, &|p, cells| Box::new(CpuEngine::new(p, cells)), true);
}
