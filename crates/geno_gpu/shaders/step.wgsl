// One step for every cell: gather neighbours (no float atomics), mechanics, hormone diffusion and
// pumping, growth, turning of the growth direction, moving of the pumps. Reads the current buffers, writes the next ones.
// Mirrors geno_sim::rules::{interact, finish}.
@group(0) @binding(1) var<storage, read> S: State;
@group(0) @binding(2) var<storage, read> pos: array<vec4<f32>>;
@group(0) @binding(3) var<storage, read> hh: array<vec4<f32>>;
@group(0) @binding(4) var<storage, read> spos: array<vec4<f32>>;
@group(0) @binding(5) var<storage, read> shid: array<Tagged>;
@group(0) @binding(6) var<storage, read> offsets: array<u32>;
@group(0) @binding(7) var<storage, read_write> pos_out: array<vec4<f32>>;
@group(0) @binding(8) var<storage, read_write> hh_out: array<vec4<f32>>;
@group(0) @binding(9) var<storage, read> shape: array<Shape>;
@group(0) @binding(10) var<storage, read> sshape: array<Shape>;
@group(0) @binding(11) var<storage, read_write> shape_out: array<Shape>;
// Pump polarity (xyz; w: light received, display only), read by cell id: neighbours are looked up unsorted.
@group(0) @binding(12) var<storage, read> pin: array<vec4<f32>>;
@group(0) @binding(13) var<storage, read_write> pin_out: array<vec4<f32>>;
@group(0) @binding(14) var<storage, read> youth: array<f32>;
@group(0) @binding(15) var<storage, read_write> youth_out: array<f32>;

@compute @workgroup_size(256)
fn step(@builtin(global_invocation_id) g: vec3<u32>) {
    let i = g.x;
    if (i >= S.count) { return; }
    let pi = pos[i];
    let si = shape[i];
    let hi = hh[i];
    let vi = pi.w * pi.w * pi.w;
    let c = vec3<i32>(cell_coord(pi.xyz));
    let top = i32(P.g1) - 1;
    var disp = vec3<f32>(0.0);
    var spread = vec4<f32>(0.0);
    var pumped = 0.0;
    var flux = vec3<f32>(0.0);
    var richer = vec3<f32>(0.0);
    var richness = 0.0;
    var around = vec3<f32>(0.0);
    var neighbours = 0.0;
    var cover = 0.0;
    var squeeze = 0.0;
    let qi = pin[i].xyz;
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
                    let far = P.reach * (si.c.y + sq.c.y);
                    if (d2 >= far * far || d2 <= 1e-12) { continue; }
                    let d = sqrt(d2);
                    let u = dv / d;
                    let rs = extent(si, u) + extent(sq, u);
                    let reach = P.reach * rs;
                    if (d >= reach) { continue; }
                    around += u;
                    neighbours += 1.0;
                    cover += max(dot(u, vec3<f32>(P.gx, P.gy, P.gz)), 0.0);
                    let s = rs - d;
                    squeeze += max(s, 0.0);
                    var f = P.mob_rep * s;
                    if (s <= 0.0) { f = -P.mob_adh * min(-s, reach - d); }
                    disp += (f / d) * dv;
                    let vj = q.w * q.w * q.w;
                    let level = 2.0 * vi * vj / (vi + vj);
                    let gap = (t.h / vj - hi / vi) * level;
                    spread += gap;
                    let qj = pin[t.id].xyz;
                    let moved = t.h.x * max(dot(qj, u), 0.0) - hi.x * max(-dot(qi, u), 0.0);
                    pumped += moved;
                    let cj = t.h.x / vj;
                    richer -= cj * u;
                    richness += cj;
                    let sent = P.diffusion * gap.x + P.polar * moved;
                    flux += sent * u;
                }
            }
        }
    }
    let lit = light(pi.xyz) * exp(-P.shade * cover);
    let made = (P.production + P.light_production * lit) * vi;
    let auxin = hi.x + P.diffusion * spread.x + P.polar * pumped + made - P.decay * hi.x;
    let cytokinin = hi.y + P.ck_diffusion * spread.y + P.ck_production * hi.x + P.ck_light * lit * vi - P.ck_decay * hi.y;
    let ca = hi.x / vi;
    let cc = hi.y / vi;
    let ck_push = (cc / (P.ck_half + cc)) * (P.ck_block / (P.ck_block + ca));
    let drive = ca / (P.half_sat + ca) + ck_push;
    // Soil: solid friction on the move, and push-back on walls pressing on it (mirrors finish).
    let soil = soil_share(pi.xyz);
    var exposed = 1.0;
    if (neighbours > 0.0) { exposed = sqrt(dot(around, around)) / neighbours; }
    // Relay made where cytokinin calls, restraint signal made by the skin that holds relay.
    let reading = ck_push * (P.restraint_half / (P.restraint_half + hi.z / vi));
    let relay = hi.w + P.relay_diffusion * spread.w + P.relay_made * reading * vi - P.relay_decay * hi.w;
    let cw = hi.w / vi;
    let told = cw / (P.relay_half + cw);
    let restraint = hi.z + P.restraint_diffusion * spread.z + P.restraint_made * told * exposed * vi - P.restraint_decay * hi.z;
    hh_out[i] = vec4<f32>(auxin, cytokinin, restraint, relay);
    let held = max(1.0 - P.soil_grip * soil * exposed / max(sqrt(dot(disp, disp)), 1e-12), 0.0);
    let free = max(1.0 - P.squeeze_hold * squeeze / pi.w, 0.0);
    let slowed = free / (1.0 + P.soil_hardness * soil * exposed);
    let even = max(min(pi.w + P.growth * min(drive, 1.0) * slowed, P.r_div), pi.w);
    let old = sh_dir(si);
    let turn = P.align * dot(old, flux);
    let a0 = old + turn * flux;
    let a = a0 * (1.0 / sqrt(max(dot(a0, a0), 1e-12)));
    let room = max(1.0 - si.c.y / (P.r_div * P.max_half_length), 0.0);
    let eps = P.elong * (ca / (P.e_half + ca)) * (P.e_block / (P.e_block + ca)) * room * slowed;
    shape_out[i] = with_dir(grow_along(grow_even(si, even / pi.w), a, eps), a);
    pos_out[i] = vec4<f32>(pi.xyz + held * disp, even * pow(1.0 + eps, 1.0 / 3.0));
    // Pumps: toward richer neighbours, along the flow, down (mirrors rules::polarise).
    let follow = ca / (P.pin_switch + ca);
    let uphill = (1.0 - follow) / max(richness, 1e-12);
    let along = follow / (P.flux_half + sqrt(dot(flux, flux)));
    let goal = uphill * richer + along * flux + P.gravisense * vec3<f32>(P.gx, P.gy, P.gz);
    let n = qi + P.pin_rate * (goal - qi);
    let len = sqrt(dot(n, n));
    var keep = 1.0;
    if (len > 1.0) { keep = 1.0 / len; }
    // w keeps the light received, for display only.
    pin_out[i] = vec4<f32>(n * keep, lit);
    // Youth: toward what the auxin calls for (mirrors finish).
    let calls = min(ca / (P.youth_half + ca) + told, 1.0);
    youth_out[i] = youth[i] + P.youth_rate * (calls - youth[i]);
}
