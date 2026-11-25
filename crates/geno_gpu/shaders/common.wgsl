// Shared by grid, step and divide shaders. Layout mirrors `params_bytes` in src/lib.rs.
struct Params {
    inv_cell: f32, half: f32, g1: u32, capacity: u32,
    mob_rep: f32, mob_adh: f32, reach: f32, diffusion: f32,
    production: f32, decay: f32, growth: f32, r_div: f32,
    inv_cbrt2: f32, seed: u32, grid_cells: u32, polar: f32,
    gx: f32, gy: f32, gz: f32, half_sat: f32,
}
struct State { count: u32, epoch: u32, pad0: u32, pad1: u32 }
struct Tagged { h: f32, id: u32 }

@group(0) @binding(0) var<uniform> P: Params;

fn cell_coord(p: vec3<f32>) -> vec3<u32> {
    let top = f32(P.g1 - 1u);
    return vec3<u32>(clamp((p + vec3<f32>(P.half)) * P.inv_cell, vec3<f32>(0.0), vec3<f32>(top)));
}

fn cell_index(c: vec3<u32>) -> u32 {
    return c.x + P.g1 * (c.y + P.g1 * c.z);
}
