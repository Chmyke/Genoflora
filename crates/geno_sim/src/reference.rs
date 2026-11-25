//! The judge: single thread, every pair tested (O(N^2)), sums in index order. Slow and obvious.

use crate::cells::Cells;
use crate::engine::{Engine, EngineError};
use crate::params::Params;
use crate::rules::{Gathered, divide, finish, interact};

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
            let me = self.cur.get(i);
            let mut g = Gathered::default();
            for j in 0..n {
                if j != i {
                    interact(&me, &self.cur.get(j), &self.p, &mut g);
                }
            }
            self.next.set(i, finish(&me, &g, &self.p));
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
