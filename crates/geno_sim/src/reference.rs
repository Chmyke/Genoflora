//! The judge: single thread, every pair tested (O(N^2)), sums in index order. Slow and obvious.

use crate::cells::Cells;
use crate::engine::{Engine, EngineError};
use crate::params::Params;
use crate::rules::{divide, finish, interact};

pub struct Reference {
    p: Params,
    cur: Cells,
    next: Cells,
    epoch: u32,
}

impl Reference {
    pub fn new(p: &Params, cells: &Cells) -> Self {
        Self {
            p: *p,
            cur: cells.clone(),
            next: cells.clone(),
            epoch: 0,
        }
    }
}

impl Engine for Reference {
    fn step(&mut self) {
        let n = self.cur.count;
        for i in 0..n {
            let (pi, hi) = (self.cur.pr[i], self.cur.h[i]);
            let (mut disp, mut hs) = ([0.0f32; 3], 0.0f32);
            for j in 0..n {
                if j != i {
                    interact(
                        &pi,
                        hi,
                        &self.cur.pr[j],
                        self.cur.h[j],
                        &self.p,
                        &mut disp,
                        &mut hs,
                    );
                }
            }
            let (pr, h) = finish(&pi, hi, &disp, hs, &self.p);
            self.next.pr[i] = pr;
            self.next.h[i] = h;
        }
        self.next.count = n;
        std::mem::swap(&mut self.cur, &mut self.next);
    }

    fn divide(&mut self) {
        divide(&mut self.cur, &self.p, self.epoch);
        self.epoch += 1;
    }

    fn snapshot(&mut self) -> Result<Cells, EngineError> {
        Ok(self.cur.clone())
    }
}
