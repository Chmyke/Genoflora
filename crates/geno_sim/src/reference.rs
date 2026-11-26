//! The judge: single thread, every pair tested (O(N^2)), sums in index order. Slow and obvious.

use crate::bonds::{Bonds, Glue};
use crate::cells::Cells;
use crate::engine::{Engine, EngineError};
use crate::params::Params;
use crate::rules::{Gathered, divide, finish, interact};

pub struct Reference {
    p: Params,
    cur: Cells,
    next: Cells,
    epoch: u32,
    /// Prototype: the walls between cells, when cells keep their neighbours (see `bonds`).
    glue: Option<Bonds>,
}

impl Reference {
    pub fn new(p: &Params, cells: &Cells) -> Self {
        Self {
            p: *p,
            cur: cells.clone(),
            next: cells.clone(),
            epoch: 0,
            glue: None,
        }
    }

    /// Prototype: the same judge, but its cells keep their neighbours. Cells bonded by a division
    /// are held by a wall (see `bonds`); cells that merely touch no longer stick.
    pub fn glued(p: &Params, cells: &Cells, glue: Glue) -> Self {
        Self {
            p: Params { mob_adh: 0.0, ..*p },
            glue: Some(Bonds::new(cells.capacity(), glue)),
            ..Self::new(p, cells)
        }
    }

    /// The bonds, when the cells keep their neighbours.
    pub fn bonds(&self) -> Option<&Bonds> {
        self.glue.as_ref()
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
            if let Some(bonds) = &self.glue {
                for (sum, part) in g.disp.iter_mut().zip(bonds.pull(&self.cur, i)) {
                    *sum += part;
                }
            }
            self.next.set(i, finish(&me, &g, &self.p));
        }
        self.next.count = n;
        if let Some(bonds) = &mut self.glue {
            bonds.creep(&self.cur);
        }
        std::mem::swap(&mut self.cur, &mut self.next);
    }

    fn divide(&mut self) {
        if let Some(bonds) = &mut self.glue {
            let before = self.cur.clone();
            divide(&mut self.cur, &self.p, self.epoch);
            bonds.inherit(&before, &self.cur);
        } else {
            divide(&mut self.cur, &self.p, self.epoch);
        }
        self.epoch += 1;
    }

    fn snapshot(&mut self) -> Result<Cells, EngineError> {
        Ok(self.cur.clone())
    }
}
