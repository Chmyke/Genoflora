//! GPU-only spike: every stage runs on the GPU, divisions included. Nothing is read back in the
//! loop. `--warp` runs on the software adapter (no graphics card needed).

use spike_gpu::GpuSim;
use spike_scenario::{Cells, Engine, cli};

struct GpuEngine(GpuSim);

impl Engine for GpuEngine {
    fn step(&mut self) {
        self.0.step();
    }

    fn divide(&mut self) {
        self.0.divide_on_gpu();
    }

    fn flush(&mut self) {
        self.0.flush();
    }

    fn snapshot(&mut self) -> Cells {
        let count = self.0.read_count();
        self.0.read_cells(count)
    }
}

fn main() {
    let software = std::env::args().any(|a| a == "--warp");
    let p = spike_scenario::Params::load();
    let machine = GpuSim::new(&p, &spike_scenario::model::init(1, 1, &p), software).machine().to_string();
    let name = if software { "gpu-warp" } else { "gpu" };
    cli(name, &machine, &|p, cells| Box::new(GpuEngine(GpuSim::new(p, cells, software))), false);
}
