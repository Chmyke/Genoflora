//! Stateless random draws: a pure function of (seed, cell, salt), identical on CPU and in WGSL.

/// lowbias32 integer mixer (wrapping u32 arithmetic only).
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

/// Division axis (unit vector) of cell `id` at division round `epoch`.
pub fn axis(seed: u32, id: u32, epoch: u32) -> [f32; 3] {
    let salt = 16u32.wrapping_add(epoch.wrapping_mul(4));
    let v = [0, 1, 2].map(|c| unit(hash(seed, id, salt.wrapping_add(c))) * 2.0 - 1.0);
    let d2 = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).max(1e-6);
    let inv = 1.0 / d2.sqrt();
    v.map(|c| c * inv)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_stays_in_range() {
        for id in 0..10_000 {
            let u = unit(hash(7, id, 3));
            assert!((0.0..1.0).contains(&u));
        }
    }

    #[test]
    fn axis_is_a_unit_vector() {
        for id in 0..1000 {
            let a = axis(7, id, id % 5);
            let len = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
            assert!((len - 1.0).abs() < 1e-5, "length {len}");
        }
    }
}
