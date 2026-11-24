//! Cells, parameters, stateless hash, initial state and the rules shared by every CPU path.

/// Scenario parameters, read from `Spikes/scenario.toml` (embedded at compile time).
#[derive(Clone, Copy, Debug)]
pub struct Params {
    pub seed: u32,
    pub r_div: f32,
    pub inv_cbrt2: f32,
    pub growth: f32,
    pub mob_rep: f32,
    pub mob_adh: f32,
    pub reach: f32,
    pub diffusion: f32,
    pub production: f32,
    pub decay: f32,
    pub divide_every: u32,
    pub spacing: f32,
    pub noise: f32,
}

impl Params {
    pub fn load() -> Self {
        let text = include_str!("../../scenario.toml");
        let get = |key: &str| -> f64 {
            for line in text.lines() {
                let line = line.split('#').next().unwrap_or("");
                if let Some((k, v)) = line.split_once('=') {
                    if k.trim() == key {
                        return v.trim().parse().unwrap_or_else(|_| panic!("bad value for {key}"));
                    }
                }
            }
            panic!("missing key {key} in scenario.toml")
        };
        Self {
            seed: get("seed") as u32,
            r_div: get("r_div") as f32,
            inv_cbrt2: (1.0f64 / 2.0f64.cbrt()) as f32,
            growth: get("growth") as f32,
            mob_rep: get("mob_rep") as f32,
            mob_adh: get("mob_adh") as f32,
            reach: get("reach") as f32,
            diffusion: get("diffusion") as f32,
            production: get("production") as f32,
            decay: get("decay") as f32,
            divide_every: get("divide_every") as u32,
            spacing: get("spacing") as f32,
            noise: get("noise") as f32,
        }
    }
}

/// Struct of arrays. A cell's index is its stable identifier (cells never die in this scenario).
/// `pr[i]` = position xyz + radius, `h[i]` = hormone amount. Vectors are `capacity` long.
#[derive(Clone, Debug, PartialEq)]
pub struct Cells {
    pub count: usize,
    pub pr: Vec<[f32; 4]>,
    pub h: Vec<f32>,
}

impl Cells {
    pub fn capacity(&self) -> usize {
        self.pr.len()
    }
}

/// lowbias32 integer mixer: identical on CPU and in WGSL (wrapping u32 arithmetic).
pub fn mix(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^= x >> 16;
    x
}

pub fn hash(seed: u32, id: u32, salt: u32) -> u32 {
    mix(seed ^ mix(id ^ mix(salt)))
}

/// Exact conversion to [0, 1): 24 bits, power-of-two scale.
pub fn unit(h: u32) -> f32 {
    (h >> 8) as f32 * (1.0 / 16_777_216.0)
}

/// Division axis of cell `id` at division round `epoch`.
pub fn axis(seed: u32, id: u32, epoch: u32) -> [f32; 3] {
    let salt = 16 + epoch * 4;
    let v = [0, 1, 2].map(|c| unit(hash(seed, id, salt + c)) * 2.0 - 1.0);
    let d2 = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).max(1e-6);
    let inv = 1.0 / d2.sqrt();
    v.map(|c| c * inv)
}

/// N cells on a noisy cubic lattice, the N lattice points closest to the origin.
pub fn init(n: usize, capacity: usize, p: &Params) -> Cells {
    assert!(n <= capacity);
    let mut m = 1i32;
    while ((2 * m + 1) as usize).pow(3) < n * 22 / 10 + 27 {
        m += 1;
    }
    let mut pts: Vec<(u32, [i32; 3])> = Vec::with_capacity(((2 * m + 1) as usize).pow(3));
    for z in -m..=m {
        for y in -m..=m {
            for x in -m..=m {
                pts.push(((x * x + y * y + z * z) as u32, [x, y, z]));
            }
        }
    }
    pts.sort_unstable_by_key(|&(d2, [x, y, z])| (d2, z, y, x));
    let mut cells = Cells { count: n, pr: vec![[0.0; 4]; capacity], h: vec![0.0; capacity] };
    for (i, &(_, q)) in pts.iter().take(n).enumerate() {
        let id = i as u32;
        let jitter = |c: u32| (unit(hash(p.seed, id, c)) * 2.0 - 1.0) * p.noise;
        cells.pr[i] = [
            q[0] as f32 * p.spacing + jitter(0),
            q[1] as f32 * p.spacing + jitter(1),
            q[2] as f32 * p.spacing + jitter(2),
            p.r_div * (0.8 + 0.2 * unit(hash(p.seed, id, 3))),
        ];
        cells.h[i] = unit(hash(p.seed, id, 4));
    }
    cells
}

/// Uniform grid covering the ball that `capacity` cells can form. Out-of-range cells are clamped
/// into the border cells, so the neighbour search stays correct anywhere.
#[derive(Clone, Copy, Debug)]
pub struct Grid {
    pub g1: u32,
    pub inv_cell: f32,
    pub half: f32,
}

impl Grid {
    pub fn for_capacity(capacity: usize, p: &Params) -> Self {
        let cell = 2.5 * p.r_div; // > reach * 2 * r_div
        let radius = 1.6 * (capacity as f32).cbrt() * p.r_div;
        let g1 = (2.0 * radius / cell).ceil() as u32 + 2;
        Self { g1, inv_cell: 1.0 / cell, half: g1 as f32 * cell * 0.5 }
    }

    pub fn cells(&self) -> usize {
        (self.g1 as usize).pow(3)
    }

    #[inline]
    pub fn coord(&self, p: &[f32; 4]) -> [u32; 3] {
        let top = (self.g1 - 1) as f32;
        [0, 1, 2].map(|a| ((p[a] + self.half) * self.inv_cell).clamp(0.0, top) as u32)
    }

    #[inline]
    pub fn index(&self, c: [u32; 3]) -> u32 {
        c[0] + self.g1 * (c[1] + self.g1 * c[2])
    }
}

/// Contribution of neighbour `j` to cell `i`: displacement and hormone exchange (gather form).
#[inline]
pub fn interact(pi: &[f32; 4], hi: f32, pj: &[f32; 4], hj: f32, p: &Params, disp: &mut [f32; 3], hs: &mut f32) {
    let dv = [pi[0] - pj[0], pi[1] - pj[1], pi[2] - pj[2]];
    let d2 = dv[0] * dv[0] + dv[1] * dv[1] + dv[2] * dv[2];
    let rs = pi[3] + pj[3];
    let reach = p.reach * rs;
    if d2 < reach * reach && d2 > 1e-12 {
        let d = d2.sqrt();
        // Overlap: linear repulsion. Gap: adhesion shaped as a tent that returns to zero at the
        // reach, so the force has no jump (a jump would amplify rounding differences).
        let s = rs - d;
        let pull = (-s).min(reach - d);
        let coef = (if s > 0.0 { p.mob_rep * s } else { -p.mob_adh * pull }) / d;
        disp[0] += coef * dv[0];
        disp[1] += coef * dv[1];
        disp[2] += coef * dv[2];
        *hs += hj - hi;
    }
}

/// New state of cell `i` once every neighbour has been gathered.
#[inline]
pub fn finish(pi: &[f32; 4], hi: f32, disp: &[f32; 3], hs: f32, p: &Params) -> ([f32; 4], f32) {
    let source = if pi[0] > 0.0 { p.production } else { 0.0 };
    let h = hi + p.diffusion * hs + source - p.decay * hi;
    let r = (pi[3] + p.growth).min(p.r_div);
    ([pi[0] + disp[0], pi[1] + disp[1], pi[2] + disp[2], r], h)
}

/// One division round, in index order. Daughter of the n-th dividing cell gets index `count + n`;
/// nothing divides beyond the capacity.
pub fn divide(cells: &mut Cells, p: &Params, epoch: u32) {
    let count = cells.count;
    let capacity = cells.capacity();
    let mut rank = 0;
    for i in 0..count {
        if cells.pr[i][3] >= p.r_div {
            let j = count + rank;
            rank += 1;
            if j >= capacity {
                continue;
            }
            let a = axis(p.seed, i as u32, epoch);
            let m = cells.pr[i];
            let r = m[3] * p.inv_cbrt2;
            let o = a.map(|c| c * (0.5 * r));
            let h = cells.h[i] * 0.5;
            cells.pr[i] = [m[0] - o[0], m[1] - o[1], m[2] - o[2], r];
            cells.pr[j] = [m[0] + o[0], m[1] + o[1], m[2] + o[2], r];
            cells.h[i] = h;
            cells.h[j] = h;
        }
    }
    cells.count = (count + rank).min(capacity);
}
