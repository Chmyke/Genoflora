// Division on the GPU: flag the cells to divide, (prefix sum in scan.wgsl gives each one its
// rank), write the daughters at `count + rank`, then update the count and the indirect dispatch.
@group(0) @binding(1) var<storage, read_write> S: State;
@group(0) @binding(2) var<storage, read_write> pos: array<vec4<f32>>;
@group(0) @binding(3) var<storage, read_write> hh: array<vec4<f32>>;
@group(0) @binding(4) var<storage, read_write> flags: array<u32>;
@group(0) @binding(5) var<storage, read> rank: array<u32>;
@group(0) @binding(6) var<storage, read_write> indirect: array<u32>;
@group(0) @binding(7) var<storage, read_write> shape: array<Shape>;
@group(0) @binding(8) var<storage, read_write> pin: array<vec4<f32>>;
@group(0) @binding(9) var<storage, read_write> youth: array<f32>;

fn mix(v: u32) -> u32 {
    var x = v;
    x ^= x >> 16u;
    x *= 0x7feb352du;
    x ^= x >> 15u;
    x *= 0x846ca68bu;
    x ^= x >> 16u;
    return x;
}

fn hash(seed: u32, id: u32, salt: u32) -> u32 {
    return mix(seed ^ mix(id ^ mix(salt)));
}

fn unit(h: u32) -> f32 {
    return f32(h >> 8u) * (1.0 / 16777216.0);
}

fn axis(id: u32, epoch: u32) -> vec3<f32> {
    let salt = 16u + epoch * 4u;
    let v = vec3<f32>(unit(hash(P.seed, id, salt)), unit(hash(P.seed, id, salt + 1u)), unit(hash(P.seed, id, salt + 2u))) * 2.0 - 1.0;
    let d2 = max(dot(v, v), 1e-6);
    return v * (1.0 / sqrt(d2));
}

@compute @workgroup_size(256)
fn flag(@builtin(global_invocation_id) g: vec3<u32>) {
    let i = g.x;
    if (i >= arrayLength(&flags)) { return; }
    var f = 0u;
    if (i < S.count) {
        // 1.001 = geno_sim::params::LENGTH_MARGIN, 0.5 = geno_sim::params::YOUNG.
        let ready = pos[i].w >= P.r_div || shape[i].c.y >= P.r_div * P.max_half_length * 1.001;
        if (youth[i] > 0.5 && ready) { f = 1u; }
    }
    flags[i] = f;
}

@compute @workgroup_size(256)
fn apply(@builtin(global_invocation_id) g: vec3<u32>) {
    let i = g.x;
    if (i >= S.count) { return; }
    if (flags[i] == 0u) { return; }
    let j = S.count + rank[i];
    if (j >= P.capacity) { return; }
    let m = pos[i];
    let s = shape[i];
    // Random direction leaning toward the long axis; the cell is cut in two halves across it.
    let v = stretched(s, axis(i, S.epoch));
    let d = v * (1.0 / sqrt(max(dot(v, v), 1e-24)));
    let o = d * (0.5 * extent(s, d));
    let daughter = grow_along(s, d, -0.5);
    let r = m.w * P.inv_cbrt2;
    let h = hh[i] * 0.5;
    pos[i] = vec4<f32>(m.xyz - o, r);
    pos[j] = vec4<f32>(m.xyz + o, r);
    shape[i] = daughter;
    shape[j] = daughter;
    hh[i] = h;
    hh[j] = h;
    pin[j] = pin[i];
    youth[j] = youth[i];
}

@compute @workgroup_size(1)
fn finalize() {
    let last = arrayLength(&flags) - 1u;
    let count = min(S.count + rank[last] + flags[last], P.capacity);
    S.count = count;
    S.epoch += 1u;
    indirect[0] = (count + 255u) / 256u;
    indirect[1] = 1u;
    indirect[2] = 1u;
}
