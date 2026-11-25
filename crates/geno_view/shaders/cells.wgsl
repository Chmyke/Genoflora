// Draws every cell as its true ellipsoid: one camera-facing quad per cell, and the fragment
// shader intersects the view ray with the ellipsoid (an impostor: exact shape, no mesh).
// The cell buffers are the simulation's own (layouts in geno_gpu/shaders/common.wgsl).
struct View {
    view_proj: mat4x4<f32>,
    eye: vec4<f32>,
    right: vec4<f32>,
    up: vec4<f32>,
    // Unit vector against gravity.
    sky: vec4<f32>,
    // Point the camera looks at (xyz) and whether the near half is cut away (w).
    focus: vec4<f32>,
    // mode, soil surface height, auxin scale, cytokinin scale.
    look: vec4<f32>,
}
struct State { count: u32, epoch: u32, pad0: u32, pad1: u32 }
struct Shape { a: vec4<f32>, b: vec4<f32>, c: vec4<f32> }

@group(0) @binding(0) var<uniform> V: View;
@group(0) @binding(1) var<storage, read> S: State;
@group(0) @binding(2) var<storage, read> pos: array<vec4<f32>>;
@group(0) @binding(3) var<storage, read> shape: array<Shape>;
@group(0) @binding(4) var<storage, read> hh: array<vec2<f32>>;
@group(0) @binding(5) var<storage, read> pin: array<vec4<f32>>;
@group(0) @binding(6) var<storage, read> youth: array<f32>;

struct Out {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) @interpolate(flat) centre: vec3<f32>,
    @location(2) @interpolate(flat) n_diag: vec3<f32>,
    @location(3) @interpolate(flat) n_off: vec3<f32>,
    @location(4) @interpolate(flat) value: f32,
}

const CORNERS = array<vec2<f32>, 6>(
    vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, -1.0), vec2<f32>(1.0, 1.0),
    vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, 1.0), vec2<f32>(-1.0, 1.0));

// What the colour shows, from 0 to 1 (see `Mode` in src/lib.rs).
fn shown(i: u32) -> f32 {
    let mode = u32(V.look.x);
    let r = pos[i].w;
    let volume = r * r * r;
    let q = pin[i];
    switch mode {
        case 0u { return q.w; }
        case 1u { return youth[i]; }
        case 2u { return sqrt(hh[i].x / volume / V.look.z); }
        case 3u { return sqrt(hh[i].y / volume / V.look.w); }
        case 4u { return 0.5 + 0.5 * dot(q.xyz, V.sky.xyz); }
        default { return length(q.xyz); }
    }
}

@vertex
fn cell(@builtin(vertex_index) v: u32, @builtin(instance_index) i: u32) -> Out {
    var out: Out;
    let p = pos[i];
    let s = shape[i];
    // Cut away the half nearer than the plane through the target, to see inside.
    let toward = normalize(V.eye.xyz - V.focus.xyz);
    let cut = V.focus.w > 0.5 && dot(p.xyz - V.focus.xyz, toward) > 0.0;
    if (i >= S.count || cut) {
        out.clip = vec4<f32>(2.0, 2.0, 2.0, 1.0);
        return out;
    }
    // The quad covers the sphere that bounds the cell (its half-length), with room for perspective.
    let corner = CORNERS[v] * s.c.y * 1.3;
    let world = p.xyz + V.right.xyz * corner.x + V.up.xyz * corner.y;
    out.clip = V.view_proj * vec4<f32>(world, 1.0);
    out.world = world;
    out.centre = p.xyz;
    out.n_diag = s.a.xyz;
    out.n_off = vec3<f32>(s.a.w, s.b.x, s.b.y);
    out.value = shown(i);
    return out;
}

// N x for the symmetric matrix N of the ellipsoid.
fn apply(d: vec3<f32>, o: vec3<f32>, x: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(
        d.x * x.x + o.x * x.y + o.y * x.z,
        o.x * x.x + d.y * x.y + o.z * x.z,
        o.y * x.x + o.z * x.y + d.z * x.z);
}

fn mix3(a: vec3<f32>, b: vec3<f32>, t: f32) -> vec3<f32> { return a + (b - a) * t; }

// Five-stop ramps, the same as tools/viewer.html.
fn sequential(t: f32) -> vec3<f32> {
    let stops = array<vec3<f32>, 5>(
        vec3<f32>(68.0, 1.0, 84.0), vec3<f32>(59.0, 82.0, 139.0), vec3<f32>(33.0, 145.0, 140.0),
        vec3<f32>(94.0, 201.0, 98.0), vec3<f32>(253.0, 231.0, 37.0));
    let x = clamp(t, 0.0, 1.0) * 4.0;
    let k = min(u32(x), 3u);
    return mix3(stops[k], stops[k + 1u], x - f32(k)) / 255.0;
}

fn diverging(t: f32) -> vec3<f32> {
    let stops = array<vec3<f32>, 5>(
        vec3<f32>(33.0, 102.0, 172.0), vec3<f32>(146.0, 197.0, 222.0), vec3<f32>(247.0, 247.0, 247.0),
        vec3<f32>(244.0, 165.0, 130.0), vec3<f32>(178.0, 24.0, 43.0));
    let x = clamp(t, 0.0, 1.0) * 4.0;
    let k = min(u32(x), 3u);
    return mix3(stops[k], stops[k + 1u], x - f32(k)) / 255.0;
}

struct Pixel {
    @location(0) colour: vec4<f32>,
    @builtin(frag_depth) depth: f32,
}

@fragment
fn paint(in: Out) -> Pixel {
    let d = normalize(in.world - V.eye.xyz);
    let oc = V.eye.xyz - in.centre;
    let nd = apply(in.n_diag, in.n_off, d);
    let a = dot(d, nd);
    let b = 2.0 * dot(oc, nd);
    let c = dot(oc, apply(in.n_diag, in.n_off, oc)) - 1.0;
    let disc = b * b - 4.0 * a * c;
    if (disc < 0.0) { discard; }
    let t = (-b - sqrt(disc)) / (2.0 * a);
    let hit = V.eye.xyz + t * d;
    let normal = normalize(apply(in.n_diag, in.n_off, hit - in.centre));
    var base = sequential(in.value);
    if (u32(V.look.x) == 4u) { base = diverging(in.value); }
    // Soft light from the camera side and from above, darker toward the rim so cells read apart.
    let facing = max(dot(normal, -d), 0.0);
    let above = 0.5 + 0.5 * dot(normal, V.sky.xyz);
    var colour = base * (0.45 + 0.4 * facing + 0.15 * above);
    // Under the surface, the soil in front tints the cell.
    if (dot(hit, V.sky.xyz) < V.look.y) { colour = mix3(colour, vec3<f32>(0.62, 0.52, 0.40), 0.15); }
    let clip = V.view_proj * vec4<f32>(hit, 1.0);
    var out: Pixel;
    out.colour = vec4<f32>(colour, 1.0);
    out.depth = clip.z / clip.w;
    return out;
}

// The soil surface: one large translucent sheet, so the roots show through it.
struct Sheet {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec3<f32>,
}

@vertex
fn ground(@builtin(vertex_index) v: u32) -> Sheet {
    let corner = CORNERS[v] * 2000.0;
    // Two directions across gravity.
    let sky = V.sky.xyz;
    var side = cross(sky, vec3<f32>(1.0, 0.0, 0.0));
    if (dot(side, side) < 0.01) { side = cross(sky, vec3<f32>(0.0, 0.0, 1.0)); }
    side = normalize(side);
    let other = cross(sky, side);
    let world = sky * V.look.y + side * corner.x + other * corner.y;
    var out: Sheet;
    out.clip = V.view_proj * vec4<f32>(world, 1.0);
    out.world = world;
    return out;
}

@fragment
fn earth(in: Sheet) -> @location(0) vec4<f32> {
    return vec4<f32>(0.62, 0.52, 0.40, 0.3);
}
