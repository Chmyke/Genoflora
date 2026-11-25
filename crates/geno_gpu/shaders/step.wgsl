// One step for every cell: gather neighbours (no float atomics), mechanics, hormone diffusion and
// fall, growth, stretching and turning of the long axis. Reads the current buffers, writes the
// next ones. Mirrors geno_sim::rules::{interact, finish}.
@group(0) @binding(1) var<storage, read> S: State;
@group(0) @binding(2) var<storage, read> pos: array<vec4<f32>>;
@group(0) @binding(3) var<storage, read> hh: array<vec2<f32>>;
@group(0) @binding(4) var<storage, read> spos: array<vec4<f32>>;
@group(0) @binding(5) var<storage, read> shid: array<Tagged>;
@group(0) @binding(6) var<storage, read> offsets: array<u32>;
@group(0) @binding(7) var<storage, read_write> pos_out: array<vec4<f32>>;
@group(0) @binding(8) var<storage, read_write> hh_out: array<vec2<f32>>;
@group(0) @binding(9) var<storage, read> shape: array<vec4<f32>>;
@group(0) @binding(10) var<storage, read> sshape: array<vec4<f32>>;
@group(0) @binding(11) var<storage, read_write> shape_out: array<vec4<f32>>;

@compute @workgroup_size(256)
fn step(@builtin(global_invocation_id) g: vec3<u32>) {
    let i = g.x;
    if (i >= S.count) { return; }
    let pi = pos[i];
    let si = shape[i];
    let hi = hh[i];
    let c = vec3<i32>(cell_coord(pi.xyz));
    let top = i32(P.g1) - 1;
    var disp = vec3<f32>(0.0);
    var spread = vec2<f32>(0.0);
    var fall = 0.0;
    var flux = vec3<f32>(0.0);
    let down = vec3<f32>(P.gx, P.gy, P.gz);
    for (var z = max(c.z - 1, 0); z <= min(c.z + 1, top); z++) {
        for (var y = max(c.y - 1, 0); y <= min(c.y + 1, top); y++) {
            for (var x = max(c.x - 1, 0); x <= min(c.x + 1, top); x++) {
                let cell = cell_index(vec3<u32>(vec3<i32>(x, y, z)));
                let e = offsets[cell + 1u];
                for (var k = offsets[cell]; k < e; k++) {
                    let t = shid[k];
                    if (t.id == i) { continue; }
                    let q = spos[k];
                    let sq = sshape[k];
                    let dv = pi.xyz - q.xyz;
                    let d2 = dot(dv, dv);
                    let far = P.reach * (pi.w * si.w * si.w + q.w * sq.w * sq.w);
                    if (d2 >= far * far || d2 <= 1e-12) { continue; }
                    let d = sqrt(d2);
                    let u = dv / d;
                    let rs = extent(pi.w, si, u) + extent(q.w, sq, u);
                    let reach = P.reach * rs;
                    if (d >= reach) { continue; }
                    let s = rs - d;
                    var f = P.mob_rep * s;
                    if (s <= 0.0) { f = -P.mob_adh * min(-s, reach - d); }
                    disp += (f / d) * dv;
                    spread += t.h - hi;
                    let above = dot(dv, down) / -d;
                    let moved = t.h.x * max(-above, 0.0) - hi.x * max(above, 0.0);
                    fall += moved;
                    let sent = P.diffusion * (t.h.x - hi.x) + P.polar * moved;
                    flux += sent * u;
                }
            }
        }
    }
    let volume = pi.w * pi.w * pi.w;
    let auxin = hi.x + P.diffusion * spread.x + P.polar * fall + P.production * volume - P.decay * hi.x;
    let cytokinin = hi.y + P.ck_diffusion * spread.y + P.ck_production * hi.x - P.ck_decay * hi.y;
    hh_out[i] = vec2<f32>(auxin, cytokinin);
    let ca = hi.x / volume;
    let cc = hi.y / volume;
    let drive = ca / (P.half_sat + ca) + (cc / (P.ck_half + cc)) * (P.ck_block / (P.ck_block + ca));
    pos_out[i] = vec4<f32>(pi.xyz + disp, min(pi.w + P.growth * min(drive, 1.0), P.r_div));
    let k = min(si.w + P.elong * (ca / (P.e_half + ca)) * (P.e_block / (P.e_block + ca)), P.stretch_max);
    let fh = flux * (1.0 / sqrt(dot(flux, flux) + 1e-12));
    let a = si.xyz + (P.align * dot(si.xyz, fh)) * fh;
    shape_out[i] = vec4<f32>(a * (1.0 / sqrt(max(dot(a, a), 1e-12))), k);
}
