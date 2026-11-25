// Uniform grid by counting sort: count, (prefix sum in scan.wgsl), scatter, then per grid cell
// an insertion sort by id so the neighbour order (hence every float sum) is deterministic.
@group(0) @binding(1) var<storage, read> S: State;
@group(0) @binding(2) var<storage, read> pos: array<vec4<f32>>;
@group(0) @binding(3) var<storage, read> hh: array<vec2<f32>>;
@group(0) @binding(4) var<storage, read_write> counts: array<atomic<u32>>;
@group(0) @binding(5) var<storage, read> offsets: array<u32>;
@group(0) @binding(6) var<storage, read_write> sidx: array<u32>;
@group(0) @binding(7) var<storage, read_write> spos: array<vec4<f32>>;
@group(0) @binding(8) var<storage, read_write> shid: array<Tagged>;

@compute @workgroup_size(256)
fn count_cells(@builtin(global_invocation_id) g: vec3<u32>) {
    let i = g.x;
    if (i >= S.count) { return; }
    atomicAdd(&counts[cell_index(cell_coord(pos[i].xyz))], 1u);
}

// Integer atomics only: each cell takes a distinct slot of its grid cell, and the counters are
// back to zero when every cell has been placed (no clear pass needed).
@compute @workgroup_size(256)
fn scatter(@builtin(global_invocation_id) g: vec3<u32>) {
    let i = g.x;
    if (i >= S.count) { return; }
    let c = cell_index(cell_coord(pos[i].xyz));
    let k = atomicSub(&counts[c], 1u);
    sidx[offsets[c] + k - 1u] = i;
}

@compute @workgroup_size(256)
fn sort_cells(@builtin(global_invocation_id) g: vec3<u32>) {
    // Dispatched in two dimensions (rows of 2^23 cells) to stay under 65535 groups per dimension.
    let c = g.x + g.y * 8388608u;
    if (c >= P.grid_cells) { return; }
    let s = offsets[c];
    let e = offsets[c + 1u];
    for (var a = s + 1u; a < e; a++) {
        let v = sidx[a];
        var b = a;
        while (b > s && sidx[b - 1u] > v) {
            sidx[b] = sidx[b - 1u];
            b--;
        }
        sidx[b] = v;
    }
    for (var k = s; k < e; k++) {
        let i = sidx[k];
        spos[k] = pos[i];
        shid[k] = Tagged(hh[i], i, 0u);
    }
}
