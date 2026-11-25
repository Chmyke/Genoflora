// Shared by grid, step and divide shaders. Layout mirrors `params_bytes` in src/lib.rs.
struct Params {
    inv_cell: f32, half: f32, g1: u32, capacity: u32,
    mob_rep: f32, mob_adh: f32, reach: f32, diffusion: f32,
    production: f32, decay: f32, growth: f32, r_div: f32,
    inv_cbrt2: f32, seed: u32, grid_cells: u32, polar: f32,
    gx: f32, gy: f32, gz: f32, half_sat: f32,
    ck_production: f32, ck_diffusion: f32, ck_decay: f32, ck_half: f32,
    ck_block: f32, align: f32, elong: f32, e_half: f32,
    e_block: f32, stretch_max: f32, pad0: u32, pad1: u32,
}
struct State { count: u32, epoch: u32, pad0: u32, pad1: u32 }
// h = hormone amounts (auxin, cytokinin).
struct Tagged { h: vec2<f32>, id: u32, pad: u32 }

@group(0) @binding(0) var<uniform> P: Params;

// Distance from the centre of a cell to its surface along the unit vector `u`. `s` = long axis
// + stretch k: half-axes r k^2 along the axis and r / k across it (mirrors rules::extent).
fn extent(r: f32, s: vec4<f32>, u: vec3<f32>) -> f32 {
    let c = dot(s.xyz, u);
    let k2 = s.w * s.w;
    return r / sqrt(c * c / (k2 * k2) + (1.0 - c * c) * k2);
}

fn cell_coord(p: vec3<f32>) -> vec3<u32> {
    let top = f32(P.g1 - 1u);
    return vec3<u32>(clamp((p + vec3<f32>(P.half)) * P.inv_cell, vec3<f32>(0.0), vec3<f32>(top)));
}

fn cell_index(c: vec3<u32>) -> u32 {
    return c.x + P.g1 * (c.y + P.g1 * c.z);
}
