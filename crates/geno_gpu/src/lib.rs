//! GPU engine of Genoflora: wgpu + WGSL compute shaders. This is what runs the plant.
//!
//! Everything is "gather" (no float atomics); the cell count lives on the GPU and drives the
//! dispatches through an indirect buffer, so nothing is read back while the simulation runs.
//! The rules in `shaders/` mirror `geno_sim::rules`; the tests compare both.

mod grid;

use geno_sim::params::INV_CBRT2;
use geno_sim::{Cells, Engine, EngineError, Params};
pub use grid::Grid;
use wgpu::{Buffer, BufferUsages as U, ComputePipeline};

const COMMON: &str = include_str!("../shaders/common.wgsl");
/// Storage buffers bound by the busiest shader (`step`).
const STORAGE_BUFFERS: u32 = 15;
/// Queued steps are handed to the GPU in batches of this size, so the command list stays short.
const SUBMIT_EVERY: u32 = 16;
/// Longest wait for the GPU before giving up: a stuck device must fail loudly, never hang.
const GPU_PATIENCE: std::time::Duration = std::time::Duration::from_secs(120);

/// Which adapter runs the shaders.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Adapter {
    /// The graphics card.
    Hardware,
    /// WARP, the DX12 software adapter: slow, but present on every Windows machine.
    Software,
}

fn round_up(n: usize, to: usize) -> usize {
    n.div_ceil(to) * to
}

fn fail(what: &str, why: impl std::fmt::Display) -> EngineError {
    EngineError(format!("{what}: {why}"))
}

/// The five passes of a prefix sum over one array, with their bind groups and dispatch sizes.
struct Scan {
    groups: [wgpu::BindGroup; 5],
    sizes: [u32; 5],
}

pub struct GpuSim {
    device: wgpu::Device,
    queue: wgpu::Queue,
    machine: String,
    capacity: usize,
    grid_cells: usize,
    pos: [Buffer; 2],
    hh: [Buffer; 2],
    shape: [Buffer; 2],
    pin: [Buffer; 2],
    youth: [Buffer; 2],
    state: Buffer,
    indirect: Buffer,
    stage_pos: Buffer,
    stage_hh: Buffer,
    stage_shape: Buffer,
    stage_pin: Buffer,
    stage_youth: Buffer,
    stage_state: Buffer,
    cur: usize,
    pending: Option<wgpu::CommandEncoder>,
    queued: u32,
    p_count: ComputePipeline,
    p_scatter: ComputePipeline,
    p_sort: ComputePipeline,
    p_step: ComputePipeline,
    p_flag: ComputePipeline,
    p_apply: ComputePipeline,
    p_finalize: ComputePipeline,
    p_scan: [ComputePipeline; 5],
    g_count: [wgpu::BindGroup; 2],
    g_scatter: [wgpu::BindGroup; 2],
    g_sort: [wgpu::BindGroup; 2],
    g_step: [wgpu::BindGroup; 2],
    g_flag: [wgpu::BindGroup; 2],
    g_apply: [wgpu::BindGroup; 2],
    g_finalize: wgpu::BindGroup,
    scan_grid: Scan,
    scan_div: Scan,
}

/// Size of the uniform block: 54 four-byte fields.
const PARAMS_BYTES: usize = 232;
/// Bytes of one cell's hormones.
const HORMONE_BYTES: usize = 16;

/// The current cells, as GPU buffers (layouts in `shaders/common.wgsl`). `state` starts with
/// the live cell count.
pub struct Drawn<'a> {
    pub state: &'a Buffer,
    pub pos: &'a Buffer,
    pub shape: &'a Buffer,
    pub hormones: &'a Buffer,
    pub pin: &'a Buffer,
    pub youth: &'a Buffer,
}

/// Uniform block of the shaders. Layout mirrors `Params` in `shaders/common.wgsl`.
fn params_bytes(p: &Params, grid: &Grid, capacity: usize) -> Vec<u8> {
    let f = |v: f32| v.to_le_bytes();
    let u = |v: u32| v.to_le_bytes();
    [
        f(grid.inv_cell),
        f(grid.half),
        u(grid.g1),
        u(capacity as u32),
        f(p.mob_rep),
        f(p.mob_adh),
        f(p.reach),
        f(p.diffusion),
        f(p.production),
        f(p.decay),
        f(p.growth),
        f(p.r_div),
        f(INV_CBRT2),
        u(p.seed),
        u(grid.cells() as u32),
        f(p.polar),
        f(p.gravity[0]),
        f(p.gravity[1]),
        f(p.gravity[2]),
        f(p.half_sat),
        f(p.ck_production),
        f(p.ck_diffusion),
        f(p.ck_decay),
        f(p.ck_half),
        f(p.ck_block),
        f(p.align),
        f(p.elong),
        f(p.e_half),
        f(p.e_block),
        f(p.max_half_length),
        f(p.pin_rate),
        f(p.pin_switch),
        f(p.flux_half),
        f(p.gravisense),
        f(p.surface),
        f(p.soil_grip),
        f(p.soil_hardness),
        f(p.light_production),
        f(p.light_depth),
        f(p.youth_rate),
        f(p.youth_half),
        f(p.ck_light),
        f(p.shade),
        f(p.squeeze_hold),
        f(p.restraint_made),
        f(p.restraint_diffusion),
        f(p.restraint_decay),
        f(p.restraint_half),
        f(p.relay_made),
        f(p.relay_diffusion),
        f(p.relay_decay),
        f(p.relay_half),
        f(p.ck_brake),
        f(p.relay_frees),
        f(p.pin_fair),
        f(p.grav_skin),
        f(p.burn),
        f(p.burn_half),
    ]
    .concat()
}

impl GpuSim {
    /// Builds the engine and uploads `cells`. The capacity of `cells` is the most the plant can
    /// ever hold.
    pub fn new(p: &Params, cells: &Cells, adapter: Adapter) -> Result<Self, EngineError> {
        let software = adapter == Adapter::Software;
        let mut desc = wgpu::InstanceDescriptor::new_without_display_handle_from_env();
        if software {
            desc.backends = wgpu::Backends::DX12;
        }
        let instance = wgpu::Instance::new(desc);
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: software,
            ..Default::default()
        }))
        .map_err(|e| fail("no GPU adapter", e))?;
        Self::on_adapter(p, cells, &adapter)
    }

    /// Same as `new`, on an adapter chosen by the caller (a window picks one that can draw on
    /// its surface). The device is then shared: see `device`, `queue` and `drawn`.
    pub fn on_adapter(p: &Params, cells: &Cells, adapter: &wgpu::Adapter) -> Result<Self, EngineError> {
        let info = adapter.get_info();
        if adapter.limits().max_storage_buffers_per_shader_stage < STORAGE_BUFFERS {
            return Err(fail(
                "GPU too limited",
                format!("{STORAGE_BUFFERS} storage buffers per shader needed"),
            ));
        }
        let machine = format!("{} ({:?})", info.name, info.backend);
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            required_limits: adapter.limits(),
            ..Default::default()
        }))
        .map_err(|e| fail("no GPU device", e))?;

        let capacity = cells.capacity();
        let grid = Grid::for_capacity(capacity, p);
        let grid_cells = grid.cells();
        let buffer = |label: &str, bytes: usize, usage: U| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: bytes.max(16) as u64,
                usage,
                mapped_at_creation: false,
            })
        };
        let data = U::STORAGE | U::COPY_SRC | U::COPY_DST;
        let pos = [
            buffer("pos0", capacity * 16, data),
            buffer("pos1", capacity * 16, data),
        ];
        let hh = [
            buffer("h0", capacity * HORMONE_BYTES, data),
            buffer("h1", capacity * HORMONE_BYTES, data),
        ];
        let shape = [
            buffer("shape0", capacity * 48, data),
            buffer("shape1", capacity * 48, data),
        ];
        let pin = [
            buffer("pin0", capacity * 16, data),
            buffer("pin1", capacity * 16, data),
        ];
        let youth = [
            buffer("youth0", capacity * 4, data),
            buffer("youth1", capacity * 4, data),
        ];
        let state = buffer("state", 16, data);
        let indirect = buffer("indirect", 16, U::STORAGE | U::INDIRECT | U::COPY_DST);
        let params = buffer("params", PARAMS_BYTES, U::UNIFORM | U::COPY_DST);
        let stage = U::MAP_READ | U::COPY_DST;
        let stage_pos = buffer("stage pos", capacity * 16, stage);
        let stage_hh = buffer("stage h", capacity * HORMONE_BYTES, stage);
        let stage_shape = buffer("stage shape", capacity * 48, stage);
        let stage_pin = buffer("stage pin", capacity * 16, stage);
        let stage_youth = buffer("stage youth", capacity * 4, stage);
        let stage_state = buffer("stage state", 16, stage);
        let sidx = buffer("sorted idx", capacity * 4, U::STORAGE);
        let spos = buffer("sorted pos", capacity * 16, U::STORAGE);
        let sshape = buffer("sorted shape", capacity * 48, U::STORAGE);
        let shid = buffer("sorted h+id", capacity * 32, U::STORAGE);
        // Grid scan: one extra entry so `offsets[cell + 1]` exists for the last cell.
        let n0 = round_up(grid_cells + 1, 256);
        let counts = buffer("counts", n0 * 4, U::STORAGE);
        let offsets = buffer("offsets", n0 * 4, U::STORAGE);
        let d0 = round_up(capacity, 256);
        let flags = buffer("flags", d0 * 4, U::STORAGE);
        let rank = buffer("rank", d0 * 4, U::STORAGE);

        let module = |label: &str, source: String| {
            device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            })
        };
        let m_grid = module(
            "grid",
            format!("{COMMON}{}", include_str!("../shaders/grid.wgsl")),
        );
        let m_step = module(
            "step",
            format!("{COMMON}{}", include_str!("../shaders/step.wgsl")),
        );
        let m_div = module(
            "divide",
            format!("{COMMON}{}", include_str!("../shaders/divide.wgsl")),
        );
        let m_scan = module("scan", include_str!("../shaders/scan.wgsl").to_string());
        let pipeline = |module: &wgpu::ShaderModule, entry: &str| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: None,
                module,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache: None,
            })
        };
        // Bind group over the bindings an entry point really uses (automatic layout).
        let group = |pipeline: &ComputePipeline, entries: &[(u32, &Buffer)]| {
            let entries: Vec<wgpu::BindGroupEntry> = entries
                .iter()
                .map(|&(binding, b)| wgpu::BindGroupEntry {
                    binding,
                    resource: b.as_entire_binding(),
                })
                .collect();
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &pipeline.get_bind_group_layout(0),
                entries: &entries,
            })
        };

        let p_count = pipeline(&m_grid, "count_cells");
        let p_scatter = pipeline(&m_grid, "scatter");
        let p_sort = pipeline(&m_grid, "sort_cells");
        let p_step = pipeline(&m_step, "step");
        let p_flag = pipeline(&m_div, "flag");
        let p_apply = pipeline(&m_div, "apply");
        let p_finalize = pipeline(&m_div, "finalize");
        let p_scan = ["reduce1", "reduce2", "scan_top", "down1", "down0"].map(|e| pipeline(&m_scan, e));

        let scan = |a: &Buffer, out: &Buffer, n0: usize| {
            let n1 = round_up(n0 / 256, 256);
            let n2 = n1 / 256;
            let b = buffer("scan level 1", n1 * 4, U::STORAGE);
            let c = buffer("scan level 2", n2 * 4, U::STORAGE);
            let (blocks0, blocks1, blocks2) = (
                (n0 / 256).div_ceil(64) as u32,
                n1.div_ceil(64) as u32,
                n2.div_ceil(64) as u32,
            );
            Scan {
                groups: [
                    group(&p_scan[0], &[(0, a), (1, &b)]),
                    group(&p_scan[1], &[(1, &b), (2, &c)]),
                    group(&p_scan[2], &[(2, &c)]),
                    group(&p_scan[3], &[(1, &b), (2, &c)]),
                    group(&p_scan[4], &[(0, a), (1, &b), (3, out)]),
                ],
                sizes: [blocks1, blocks2, 1, blocks2, blocks0],
            }
        };
        let scan_grid = scan(&counts, &offsets, n0);
        let scan_div = scan(&flags, &rank, d0);

        let both = |f: &dyn Fn(usize) -> wgpu::BindGroup| [f(0), f(1)];
        let g_count = both(&|c| group(&p_count, &[(0, &params), (1, &state), (2, &pos[c]), (4, &counts)]));
        let g_scatter = both(&|c| {
            group(
                &p_scatter,
                &[
                    (0, &params),
                    (1, &state),
                    (2, &pos[c]),
                    (4, &counts),
                    (5, &offsets),
                    (6, &sidx),
                ],
            )
        });
        let g_sort = both(&|c| {
            group(
                &p_sort,
                &[
                    (0, &params),
                    (2, &pos[c]),
                    (3, &hh[c]),
                    (5, &offsets),
                    (6, &sidx),
                    (7, &spos),
                    (8, &shid),
                    (9, &shape[c]),
                    (10, &sshape),
                ],
            )
        });
        let g_step = both(&|c| {
            group(
                &p_step,
                &[
                    (0, &params),
                    (1, &state),
                    (2, &pos[c]),
                    (3, &hh[c]),
                    (4, &spos),
                    (5, &shid),
                    (6, &offsets),
                    (7, &pos[1 - c]),
                    (8, &hh[1 - c]),
                    (9, &shape[c]),
                    (10, &sshape),
                    (11, &shape[1 - c]),
                    (12, &pin[c]),
                    (13, &pin[1 - c]),
                    (14, &youth[c]),
                    (15, &youth[1 - c]),
                ],
            )
        });
        let g_flag = both(&|c| {
            group(
                &p_flag,
                &[
                    (0, &params),
                    (1, &state),
                    (2, &pos[c]),
                    (4, &flags),
                    (7, &shape[c]),
                    (9, &youth[c]),
                ],
            )
        });
        let g_apply = both(&|c| {
            group(
                &p_apply,
                &[
                    (0, &params),
                    (1, &state),
                    (2, &pos[c]),
                    (3, &hh[c]),
                    (4, &flags),
                    (5, &rank),
                    (7, &shape[c]),
                    (8, &pin[c]),
                    (9, &youth[c]),
                ],
            )
        });
        let g_finalize = group(
            &p_finalize,
            &[(0, &params), (1, &state), (4, &flags), (5, &rank), (6, &indirect)],
        );

        queue.write_buffer(&params, 0, &params_bytes(p, &grid, capacity));
        let mut sim = Self {
            device,
            queue,
            machine,
            capacity,
            grid_cells,
            pos,
            hh,
            shape,
            pin,
            youth,
            state,
            indirect,
            stage_pos,
            stage_hh,
            stage_shape,
            stage_pin,
            stage_youth,
            stage_state,
            cur: 0,
            pending: None,
            queued: 0,
            p_count,
            p_scatter,
            p_sort,
            p_step,
            p_flag,
            p_apply,
            p_finalize,
            p_scan,
            g_count,
            g_scatter,
            g_sort,
            g_step,
            g_flag,
            g_apply,
            g_finalize,
            scan_grid,
            scan_div,
        };
        sim.write_cells(cells, 0);
        Ok(sim)
    }

    /// The device the plant lives on, for a renderer that draws it in place.
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// Most cells the plant can ever hold.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Hands the queued steps to the GPU, then gives the buffers holding the cells as they will
    /// be once those steps are done: what a renderer draws, with no read-back.
    pub fn drawn(&mut self) -> Drawn<'_> {
        self.submit();
        Drawn {
            state: &self.state,
            pos: &self.pos[self.cur],
            shape: &self.shape[self.cur],
            hormones: &self.hh[self.cur],
            pin: &self.pin[self.cur],
            youth: &self.youth[self.cur],
        }
    }

    /// Name and backend of the adapter in use.
    pub fn machine(&self) -> &str {
        &self.machine
    }

    fn pass(&mut self) -> wgpu::ComputePass<'static> {
        let device = &self.device;
        let encoder = self
            .pending
            .get_or_insert_with(|| device.create_command_encoder(&Default::default()));
        encoder.begin_compute_pass(&Default::default()).forget_lifetime()
    }

    fn run_scan(pass: &mut wgpu::ComputePass<'_>, pipes: &[ComputePipeline; 5], scan: &Scan) {
        for ((pipe, group), &size) in pipes.iter().zip(&scan.groups).zip(&scan.sizes) {
            pass.set_pipeline(pipe);
            pass.set_bind_group(0, group, &[]);
            pass.dispatch_workgroups(size, 1, 1);
        }
    }

    /// Hands the queued work to the GPU without waiting for it.
    fn submit(&mut self) -> Option<wgpu::SubmissionIndex> {
        self.queued = 0;
        self.pending
            .take()
            .map(|encoder| self.queue.submit([encoder.finish()]))
    }

    /// Submits the queued work and waits until the GPU has finished it.
    pub fn flush(&mut self) -> Result<(), EngineError> {
        if let Some(index) = self.submit() {
            self.device
                .poll(wgpu::PollType::Wait {
                    submission_index: Some(index),
                    timeout: Some(GPU_PATIENCE),
                })
                .map_err(|e| fail("GPU wait failed", e))?;
        }
        Ok(())
    }

    fn read(&self, source: &Buffer, stage: &Buffer, bytes: usize) -> Result<Vec<u8>, EngineError> {
        if bytes == 0 {
            return Ok(Vec::new());
        }
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_buffer_to_buffer(source, 0, stage, 0, bytes as u64);
        let index = self.queue.submit([encoder.finish()]);
        let slice = stage.slice(..bytes as u64);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            // The receiver only goes away if this function already returned an error.
            let _ = sender.send(result);
        });
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(index),
                timeout: Some(GPU_PATIENCE),
            })
            .map_err(|e| fail("GPU wait failed", e))?;
        receiver
            .recv()
            .map_err(|e| fail("GPU read-back never answered", e))?
            .map_err(|e| fail("GPU read-back refused", e))?;
        let out = slice
            .get_mapped_range()
            .map_err(|e| fail("GPU read-back unmapped", e))?
            .to_vec();
        stage.unmap();
        Ok(out)
    }

    /// Number of live cells, read back from the GPU.
    pub fn read_count(&mut self) -> Result<usize, EngineError> {
        self.flush()?;
        let bytes = self.read(&self.state, &self.stage_state, 16)?;
        let count: &[u32] = bytemuck::cast_slice(&bytes);
        Ok(count.first().copied().unwrap_or(0) as usize)
    }

    /// Reads the first `count` cells back.
    pub fn read_cells(&mut self, count: usize) -> Result<Cells, EngineError> {
        self.flush()?;
        let count = count.min(self.capacity);
        let pr = self.read(&self.pos[self.cur], &self.stage_pos, count * 16)?;
        let h = self.read(&self.hh[self.cur], &self.stage_hh, count * HORMONE_BYTES)?;
        let shape = self.read(&self.shape[self.cur], &self.stage_shape, count * 48)?;
        let pin = self.read(&self.pin[self.cur], &self.stage_pin, count * 16)?;
        let youth = self.read(&self.youth[self.cur], &self.stage_youth, count * 4)?;
        let mut cells = Cells::empty(self.capacity);
        cells.count = count;
        cells.pr[..count].copy_from_slice(bytemuck::cast_slice(&pr));
        cells.h[..count].copy_from_slice(bytemuck::cast_slice(&h));
        cells.shape[..count].copy_from_slice(bytemuck::cast_slice(&shape));
        cells.pin[..count].copy_from_slice(bytemuck::cast_slice(&pin));
        cells.youth[..count].copy_from_slice(bytemuck::cast_slice(&youth));
        Ok(cells)
    }

    /// Uploads the cells and sets the count, the division epoch and the indirect dispatch size.
    /// Must not be called with work still queued (the constructor is the only caller so far).
    fn write_cells(&mut self, cells: &Cells, epoch: u32) {
        let n = cells.count;
        self.queue
            .write_buffer(&self.pos[self.cur], 0, bytemuck::cast_slice(cells.alive()));
        self.queue
            .write_buffer(&self.hh[self.cur], 0, bytemuck::cast_slice(cells.hormone()));
        self.queue
            .write_buffer(&self.shape[self.cur], 0, bytemuck::cast_slice(cells.shapes()));
        self.queue
            .write_buffer(&self.pin[self.cur], 0, bytemuck::cast_slice(cells.polarity()));
        self.queue
            .write_buffer(&self.youth[self.cur], 0, bytemuck::cast_slice(cells.youths()));
        self.queue
            .write_buffer(&self.state, 0, bytemuck::cast_slice(&[n as u32, epoch, 0, 0]));
        self.queue.write_buffer(
            &self.indirect,
            0,
            bytemuck::cast_slice(&[n.div_ceil(256) as u32, 1, 1, 0]),
        );
    }
}

impl Engine for GpuSim {
    /// Queues one simulation step: grid rebuild (count, prefix sum, scatter, sort), then gather.
    fn step(&mut self) {
        let c = self.cur;
        let mut pass = self.pass();
        pass.set_pipeline(&self.p_count);
        pass.set_bind_group(0, &self.g_count[c], &[]);
        pass.dispatch_workgroups_indirect(&self.indirect, 0);
        Self::run_scan(&mut pass, &self.p_scan, &self.scan_grid);
        pass.set_pipeline(&self.p_scatter);
        pass.set_bind_group(0, &self.g_scatter[c], &[]);
        pass.dispatch_workgroups_indirect(&self.indirect, 0);
        pass.set_pipeline(&self.p_sort);
        pass.set_bind_group(0, &self.g_sort[c], &[]);
        // Two dimensions (rows of 2^23 grid cells) to stay under 65535 groups per dimension.
        let groups = self.grid_cells.div_ceil(256) as u32;
        pass.dispatch_workgroups(groups.min(32768), groups.div_ceil(32768), 1);
        pass.set_pipeline(&self.p_step);
        pass.set_bind_group(0, &self.g_step[c], &[]);
        pass.dispatch_workgroups_indirect(&self.indirect, 0);
        drop(pass);
        self.cur = 1 - c;
        self.queued += 1;
        if self.queued >= SUBMIT_EVERY {
            self.submit();
        }
    }

    /// Queues one division round, entirely on the GPU.
    fn divide(&mut self) {
        let c = self.cur;
        let mut pass = self.pass();
        pass.set_pipeline(&self.p_flag);
        pass.set_bind_group(0, &self.g_flag[c], &[]);
        pass.dispatch_workgroups(round_up(self.capacity, 256) as u32 / 256, 1, 1);
        Self::run_scan(&mut pass, &self.p_scan, &self.scan_div);
        pass.set_pipeline(&self.p_apply);
        pass.set_bind_group(0, &self.g_apply[c], &[]);
        pass.dispatch_workgroups_indirect(&self.indirect, 0);
        pass.set_pipeline(&self.p_finalize);
        pass.set_bind_group(0, &self.g_finalize, &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }

    fn snapshot(&mut self) -> Result<Cells, EngineError> {
        let count = self.read_count()?;
        self.read_cells(count)
    }
}
