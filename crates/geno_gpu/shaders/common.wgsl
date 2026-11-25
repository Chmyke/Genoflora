// Shared by grid, step and divide shaders. Layout mirrors `params_bytes` in src/lib.rs.
struct Params {
    inv_cell: f32, half: f32, g1: u32, capacity: u32,
    mob_rep: f32, mob_adh: f32, reach: f32, diffusion: f32,
    production: f32, decay: f32, growth: f32, r_div: f32,
    inv_cbrt2: f32, seed: u32, grid_cells: u32, polar: f32,
    gx: f32, gy: f32, gz: f32, half_sat: f32,
    ck_production: f32, ck_diffusion: f32, ck_decay: f32, ck_half: f32,
    ck_block: f32, align: f32, elong: f32, e_half: f32,
    e_block: f32, max_half_length: f32, pin_rate: f32, pin_switch: f32,
    flux_half: f32, gravisense: f32, surface: f32, soil_grip: f32,
    soil_hardness: f32, light_production: f32, light_depth: f32, youth_rate: f32,
    youth_half: f32, ck_light: f32, shade: f32, pad0: u32,
}
struct State { count: u32, epoch: u32, pad0: u32, pad1: u32 }
// h = hormone amounts (auxin, cytokinin).
struct Tagged { h: vec2<f32>, id: u32, pad: u32 }

@group(0) @binding(0) var<uniform> P: Params;

// Cell shape, mirrors geno_sim::shape (same 48 bytes): the ellipsoid x^T N x = 1 and the growth
// direction. a = (n00, n11, n22, n01), b = (n02, n12, dir x, dir y), c = (dir z, half-length, -, -).
struct Shape { a: vec4<f32>, b: vec4<f32>, c: vec4<f32> }

fn sh_dir(s: Shape) -> vec3<f32> {
    return vec3<f32>(s.b.z, s.b.w, s.c.x);
}

fn sh_quad(s: Shape, u: vec3<f32>) -> f32 {
    return s.a.x * u.x * u.x + s.a.y * u.y * u.y + s.a.z * u.z * u.z
        + 2.0 * (s.a.w * u.x * u.y + s.b.x * u.x * u.z + s.b.y * u.y * u.z);
}

// Distance from the centre of a cell to its surface along the unit vector `u`.
fn extent(s: Shape, u: vec3<f32>) -> f32 {
    return 1.0 / sqrt(sh_quad(s, u));
}

// Smallest eigenvalue of N: 4 sweeps of cyclic Jacobi rotations (mirrors shape::eigen_extremes).
fn eigen_low(s: Shape) -> f32 {
    var a = mat3x3<f32>(s.a.x, s.a.w, s.b.x, s.a.w, s.a.y, s.b.y, s.b.x, s.b.y, s.a.z);
    for (var sweep = 0; sweep < 4; sweep++) {
        a = jacobi(a, 0, 1, 2);
        a = jacobi(a, 0, 2, 1);
        a = jacobi(a, 1, 2, 0);
    }
    return min(min(a[0][0], a[1][1]), a[2][2]);
}

// One Jacobi rotation zeroing a[p][q] (r is the third index), mirrors shape::rotate.
fn jacobi(m: mat3x3<f32>, p: i32, q: i32, r: i32) -> mat3x3<f32> {
    var a = m;
    let apq = a[p][q];
    if (apq == 0.0) { return a; }
    let theta = (a[q][q] - a[p][p]) / (2.0 * apq);
    var sgn = 1.0;
    if (theta < 0.0) { sgn = -1.0; }
    let t = sgn / (abs(theta) + sqrt(theta * theta + 1.0));
    let c = 1.0 / sqrt(t * t + 1.0);
    let s = t * c;
    a[p][p] -= t * apq;
    a[q][q] += t * apq;
    a[p][q] = 0.0;
    a[q][p] = 0.0;
    let arp = a[r][p];
    let arq = a[r][q];
    a[r][p] = c * arp - s * arq;
    a[p][r] = a[r][p];
    a[r][q] = s * arp + c * arq;
    a[q][r] = a[r][q];
    return a;
}

fn settle(s: Shape) -> Shape {
    var out = s;
    out.c.y = 1.0 / sqrt(max(eigen_low(s), 1e-12));
    return out;
}

fn sh_det(s: Shape) -> f32 {
    let a = s.a.x; let b = s.a.y; let c = s.a.z; let d = s.a.w; let e = s.b.x; let f = s.b.y;
    return a * (b * c - f * f) - d * (d * c - f * e) + e * (d * f - b * e);
}

// Even growth by the radius factor k: the added volume goes to a sphere, which rounds the cell off
// (mirrors shape::grow_even).
fn grow_even(s: Shape, k: f32) -> Shape {
    if (k <= 1.0) { return s; }
    let added = 1.0 - 1.0 / (k * k * k);
    let d = sh_det(s);
    let round = pow(d, 1.0 / 3.0);
    var out = s;
    out.a = vec4<f32>(
        (1.0 - added) * s.a.x + added * round,
        (1.0 - added) * s.a.y + added * round,
        (1.0 - added) * s.a.z + added * round,
        (1.0 - added) * s.a.w);
    out.b = vec4<f32>((1.0 - added) * s.b.x, (1.0 - added) * s.b.y, s.b.z, s.b.w);
    let fix = pow(d / (k * k * k * k * k * k * sh_det(out)), 1.0 / 3.0);
    out.a = out.a * fix;
    out.b = vec4<f32>(out.b.x * fix, out.b.y * fix, out.b.z, out.b.w);
    return settle(out);
}

// Grows the cell by the factor 1 + eps along the unit vector g only.
fn grow_along(s: Shape, g: vec3<f32>, eps: f32) -> Shape {
    let c = -eps / (1.0 + eps);
    let v = vec3<f32>(
        s.a.x * g.x + s.a.w * g.y + s.b.x * g.z,
        s.a.w * g.x + s.a.y * g.y + s.b.y * g.z,
        s.b.x * g.x + s.b.y * g.y + s.a.z * g.z);
    let w = dot(g, v);
    let cw = c * c * w;
    var out = s;
    out.a = s.a + vec4<f32>(
        c * (g.x * v.x + v.x * g.x) + cw * g.x * g.x,
        c * (g.y * v.y + v.y * g.y) + cw * g.y * g.y,
        c * (g.z * v.z + v.z * g.z) + cw * g.z * g.z,
        c * (g.x * v.y + v.x * g.y) + cw * g.x * g.y);
    out.b = vec4<f32>(
        s.b.x + c * (g.x * v.z + v.x * g.z) + cw * g.x * g.z,
        s.b.y + c * (g.y * v.z + v.y * g.z) + cw * g.y * g.z,
        s.b.z, s.b.w);
    return settle(out);
}

// N^-1 x up to a positive factor: x leaning toward the long axis of the cell.
fn stretched(s: Shape, x: vec3<f32>) -> vec3<f32> {
    let a = s.a.x; let b = s.a.y; let c = s.a.z; let d = s.a.w; let e = s.b.x; let f = s.b.y;
    let m0 = b * c - f * f; let m1 = e * f - d * c; let m2 = d * f - b * e;
    let m3 = a * c - e * e; let m4 = d * e - a * f; let m5 = a * b - d * d;
    return vec3<f32>(m0 * x.x + m1 * x.y + m2 * x.z, m1 * x.x + m3 * x.y + m4 * x.z, m2 * x.x + m4 * x.y + m5 * x.z);
}

fn with_dir(s: Shape, g: vec3<f32>) -> Shape {
    var out = s;
    out.b = vec4<f32>(s.b.x, s.b.y, g.x, g.y);
    out.c = vec4<f32>(g.z, s.c.y, s.c.z, s.c.w);
    return out;
}

// How much of a point is soil (mirrors soil::share).
fn soil_share(at: vec3<f32>) -> f32 {
    let height = -dot(at, vec3<f32>(P.gx, P.gy, P.gz));
    return clamp((P.surface - height) / P.r_div + 0.5, 0.0, 1.0);
}

// Light reaching a point (mirrors soil::light).
fn light(at: vec3<f32>) -> f32 {
    let height = -dot(at, vec3<f32>(P.gx, P.gy, P.gz));
    return exp(-max(P.surface - height, 0.0) / P.light_depth);
}

fn cell_coord(p: vec3<f32>) -> vec3<u32> {
    let top = f32(P.g1 - 1u);
    return vec3<u32>(clamp((p + vec3<f32>(P.half)) * P.inv_cell, vec3<f32>(0.0), vec3<f32>(top)));
}

fn cell_index(c: vec3<u32>) -> u32 {
    return c.x + P.g1 * (c.y + P.g1 * c.z);
}
