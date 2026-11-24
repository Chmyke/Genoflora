//! Hybrid spike: mechanics, diffusion and growth on the GPU; divisions decided and applied on the
//! CPU every K steps (read the cells back, divide with the shared rule, upload).

use spike_gpu::GpuSim;
use spike_scenario::{Cells, Engine, Params, cli, divide};

struct HybridEngine {
    sim: GpuSim,
    p: Params,
    count: usize,
    epoch: u32,
}

impl Engine for HybridEngine {
    fn step(&mut self) {
        self.sim.step();
    }

    fn divide(&mut self) {
        let mut cells = self.sim.read_cells(self.count);
        divide(&mut cells, &self.p, self.epoch);
        self.epoch += 1;
        self.count = cells.count;
        self.sim.write_cells(&cells, self.epoch);
    }

    fn flush(&mut self) {
        self.sim.flush();
    }

    fn snapshot(&mut self) -> Cells {
        self.sim.read_cells(self.count)
    }
}

fn main() {
    let software = std::env::args().any(|a| a == "--warp");
    let p = Params::load();
    let machine = GpuSim::new(&p, &spike_scenario::model::init(1, 1, &p), software).machine().to_string();
    let name = if software { "hybrid-warp" } else { "hybrid" };
    cli(name, &machine, &|p, cells| {
        Box::new(HybridEngine { sim: GpuSim::new(p, cells, software), p: *p, count: cells.count, epoch: 0 })
    }, false);
}
