//! GPU engine shared by the GPU-only and the hybrid spikes: wgpu + WGSL compute shaders.
//! Everything is "gather" (no float atomics); the cell count lives on the GPU and drives the
//! dispatches through an indirect buffer, so no read-back is needed in the loop.

use spike_scenario::{Cells, Grid, Params};
use wgpu::{Buffer, BufferUsages as U, ComputePipeline};

const COMMON: &str = include_str!("../shaders/common.wgsl");

fn round_up(n: usize, to: usize) -> usize {
    n.div_ceil(to) * to
}

/// The five passes of a prefix sum over one array, with their bind groups.
struct Scan {
    groups: [wgpu::BindGroup; 5],
    blocks0: u32,
    blocks1: u32,
    blocks2: u32,
}

pub struct GpuSim {
    device: wgpu::Device,
    queue: wgpu::Queue,
    machine: String,
    capacity: usize,
    grid_cells: usize,
    pos: [Buffer; 2],
    hh: [Buffer; 2],
    state: Buffer,
    indirect: Buffer,
    stage_pos: Buffer,
    stage_hh: Buffer,
    stage_state: Buffer,
    cur: usize,
    pending: Option<wgpu::CommandEncoder>,
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
        f(p.inv_cbrt2),
        u(p.seed),
        u(grid.cells() as u32),
        u(0),
    ]
    .concat()
}

impl GpuSim {
    /// `software`: run on WARP (DX12 software rasterizer), to prove it works without a GPU.
    pub fn new(p: &Params, cells: &Cells, software: bool) -> Self {
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
        .expect("no GPU adapter");
        let info = adapter.get_info();
        let machine = format!("{} ({:?})", info.name, info.backend);
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            required_limits: adapter.limits(),
            ..Default::default()
        }))
        .expect("no GPU device");

        let capacity = cells.capacity();
        let grid = Grid::for_capacity(capacity, p);
        let grid_cells = grid.cells();
        let buffer = |label: &str, bytes: usize, usage: U| {
            device.create_buffer(&wgpu::BufferDescriptor { label: Some(label), size: bytes.max(16) as u64, usage, mapped_at_creation: false })
        };
        let data = U::STORAGE | U::COPY_SRC | U::COPY_DST;
        let pos = [buffer("pos0", capacity * 16, data), buffer("pos1", capacity * 16, data)];
        let hh = [buffer("h0", capacity * 4, data), buffer("h1", capacity * 4, data)];
        let state = buffer("state", 16, data);
        let indirect = buffer("indirect", 16, U::STORAGE | U::INDIRECT | U::COPY_DST);
        let params = buffer("params", 64, U::UNIFORM | U::COPY_DST);
        let stage = U::MAP_READ | U::COPY_DST;
        let stage_pos = buffer("stage pos", capacity * 16, stage);
        let stage_hh = buffer("stage h", capacity * 4, stage);
        let stage_state = buffer("stage state", 16, stage);
        let sidx = buffer("sorted idx", capacity * 4, U::STORAGE);
        let spos = buffer("sorted pos", capacity * 16, U::STORAGE);
        let shid = buffer("sorted h+id", capacity * 8, U::STORAGE);
        // Grid scan: one extra entry so `offsets[cell + 1]` exists for the last cell.
        let n0 = round_up(grid_cells + 1, 256);
        let counts = buffer("counts", n0 * 4, U::STORAGE);
        let offsets = buffer("offsets", n0 * 4, U::STORAGE);
        let d0 = round_up(capacity, 256);
        let flags = buffer("flags", d0 * 4, U::STORAGE);
        let rank = buffer("rank", d0 * 4, U::STORAGE);

        let module = |label: &str, source: String| {
            device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some(label), source: wgpu::ShaderSource::Wgsl(source.into()) })
        };
        let m_grid = module("grid", format!("{COMMON}{}", include_str!("../shaders/grid.wgsl")));
        let m_step = module("step", format!("{COMMON}{}", include_str!("../shaders/step.wgsl")));
        let m_div = module("divide", format!("{COMMON}{}", include_str!("../shaders/divide.wgsl")));
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
            let entries: Vec<wgpu::BindGroupEntry> =
                entries.iter().map(|&(binding, b)| wgpu::BindGroupEntry { binding, resource: b.as_entire_binding() }).collect();
            device.create_bind_group(&wgpu::BindGroupDescriptor { label: None, layout: &pipeline.get_bind_group_layout(0), entries: &entries })
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
            Scan {
                groups: [
                    group(&p_scan[0], &[(0, a), (1, &b)]),
                    group(&p_scan[1], &[(1, &b), (2, &c)]),
                    group(&p_scan[2], &[(2, &c)]),
                    group(&p_scan[3], &[(1, &b), (2, &c)]),
                    group(&p_scan[4], &[(0, a), (1, &b), (3, out)]),
                ],
                blocks0: (n0 / 256).div_ceil(64) as u32,
                blocks1: n1.div_ceil(64) as u32,
                blocks2: n2.div_ceil(64) as u32,
            }
        };
        let scan_grid = scan(&counts, &offsets, n0);
        let scan_div = scan(&flags, &rank, d0);

        let both = |f: &dyn Fn(usize) -> wgpu::BindGroup| [f(0), f(1)];
        let g_count = both(&|c| group(&p_count, &[(0, &params), (1, &state), (2, &pos[c]), (4, &counts)]));
        let g_scatter = both(&|c| group(&p_scatter, &[(0, &params), (1, &state), (2, &pos[c]), (4, &counts), (5, &offsets), (6, &sidx)]));
        let g_sort =
            both(&|c| group(&p_sort, &[(0, &params), (2, &pos[c]), (3, &hh[c]), (5, &offsets), (6, &sidx), (7, &spos), (8, &shid)]));
        let g_step = both(&|c| {
            group(
                &p_step,
                &[(0, &params), (1, &state), (2, &pos[c]), (3, &hh[c]), (4, &spos), (5, &shid), (6, &offsets), (7, &pos[1 - c]), (8, &hh[1 - c])],
            )
        });
        let g_flag = both(&|c| group(&p_flag, &[(0, &params), (1, &state), (2, &pos[c]), (4, &flags)]));
        let g_apply = both(&|c| group(&p_apply, &[(0, &params), (1, &state), (2, &pos[c]), (3, &hh[c]), (4, &flags), (5, &rank)]));
        let g_finalize = group(&p_finalize, &[(0, &params), (1, &state), (4, &flags), (5, &rank), (6, &indirect)]);

        queue.write_buffer(&params, 0, &params_bytes(p, &grid, capacity));
        let mut sim = Self {
            device,
            queue,
            machine,
            capacity,
            grid_cells,
            pos,
            hh,
            state,
            indirect,
            stage_pos,
            stage_hh,
            stage_state,
            cur: 0,
            pending: None,
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
        sim
    }

    pub fn machine(&self) -> &str {
        &self.machine
    }

    fn pass(&mut self) -> wgpu::ComputePass<'static> {
        let device = &self.device;
        let encoder = self.pending.get_or_insert_with(|| device.create_command_encoder(&Default::default()));
        encoder.begin_compute_pass(&Default::default()).forget_lifetime()
    }

    fn run_scan(pass: &mut wgpu::ComputePass<'_>, pipes: &[ComputePipeline; 5], scan: &Scan) {
        let sizes = [scan.blocks1, scan.blocks2, 1, scan.blocks2, scan.blocks0];
        for k in 0..5 {
            pass.set_pipeline(&pipes[k]);
            pass.set_bind_group(0, &scan.groups[k], &[]);
            pass.dispatch_workgroups(sizes[k], 1, 1);
        }
    }

    /// Queues one simulation step: grid rebuild (count, prefix sum, scatter, sort), then gather.
    pub fn step(&mut self) {
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
        let groups = self.grid_cells.div_ceil(256) as u32;
        pass.dispatch_workgroups(groups.min(32768), groups.div_ceil(32768), 1);
        pass.set_pipeline(&self.p_step);
        pass.set_bind_group(0, &self.g_step[c], &[]);
        pass.dispatch_workgroups_indirect(&self.indirect, 0);
        drop(pass);
        self.cur = 1 - c;
    }

    /// Queues one division round entirely on the GPU.
    pub fn divide_on_gpu(&mut self) {
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

    /// Submits the queued work and waits until the GPU has finished it.
    pub fn flush(&mut self) {
        if let Some(encoder) = self.pending.take() {
            let index = self.queue.submit([encoder.finish()]);
            self.device.poll(wgpu::PollType::Wait { submission_index: Some(index), timeout: None }).expect("GPU wait failed");
        }
    }

    fn read(&self, source: &Buffer, stage: &Buffer, bytes: usize) -> Vec<u8> {
        if bytes == 0 {
            return Vec::new();
        }
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_buffer_to_buffer(source, 0, stage, 0, bytes as u64);
        let index = self.queue.submit([encoder.finish()]);
        let slice = stage.slice(..bytes as u64);
        slice.map_async(wgpu::MapMode::Read, |r| r.expect("map failed"));
        self.device.poll(wgpu::PollType::Wait { submission_index: Some(index), timeout: None }).expect("GPU wait failed");
        let out = slice.get_mapped_range().expect("mapped range").to_vec();
        stage.unmap();
        out
    }

    /// Number of live cells, read back from the GPU.
    pub fn read_count(&mut self) -> usize {
        self.flush();
        let bytes = self.read(&self.state, &self.stage_state, 16);
        u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize
    }

    /// Reads the first `count` cells back.
    pub fn read_cells(&mut self, count: usize) -> Cells {
        self.flush();
        let pr = self.read(&self.pos[self.cur], &self.stage_pos, count * 16);
        let h = self.read(&self.hh[self.cur], &self.stage_hh, count * 4);
        let mut cells = Cells { count, pr: vec![[0.0; 4]; self.capacity], h: vec![0.0; self.capacity] };
        cells.pr[..count].copy_from_slice(bytemuck::cast_slice(&pr));
        cells.h[..count].copy_from_slice(bytemuck::cast_slice(&h));
        cells
    }

    /// Uploads the cells and sets the count, the division epoch and the indirect dispatch size.
    pub fn write_cells(&mut self, cells: &Cells, epoch: u32) {
        self.flush();
        let n = cells.count;
        self.queue.write_buffer(&self.pos[self.cur], 0, bytemuck::cast_slice(&cells.pr[..n]));
        self.queue.write_buffer(&self.hh[self.cur], 0, bytemuck::cast_slice(&cells.h[..n]));
        self.queue.write_buffer(&self.state, 0, bytemuck::cast_slice(&[n as u32, epoch, 0, 0]));
        self.queue.write_buffer(&self.indirect, 0, bytemuck::cast_slice(&[n.div_ceil(256) as u32, 1, 1, 0]));
    }
}
