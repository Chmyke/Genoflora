//! Shape of a cell: an ellipsoid built by its own growth, never rotated.
//!
//! A plant cell cannot turn inside its tissue, and its wall cannot stretch without growing: it
//! grows by taking up water, more along the direction its wall lets it. So the shape is the
//! symmetric matrix `N` of the ellipsoid `x^T N x = 1` (centred on the cell), changed only by
//! growth and division, plus the direction in which the wall lets the cell grow.
//!
//! Layout (12 floats, the GPU reads the same bytes as three vec4):
//! `[n00, n11, n22, n01, n02, n12, dir x, dir y, dir z, half-length, 0, 0]`.
//! The half-length is cached: it is the distance from the centre to the farthest point.

pub type Shape = [f32; 12];

/// Index of the growth direction (3 floats) in a shape.
pub const DIR: usize = 6;
/// Index of the cached half-length in a shape.
pub const HALF: usize = 9;

fn dot(a: &[f32], b: &[f32]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// A sphere of radius `r` whose wall lets it grow along `dir` (unit vector).
pub fn sphere(r: f32, dir: [f32; 3]) -> Shape {
    let n = 1.0 / (r * r);
    [n, n, n, 0.0, 0.0, 0.0, dir[0], dir[1], dir[2], r, 0.0, 0.0]
}

pub fn dir(s: &Shape) -> [f32; 3] {
    [s[DIR], s[DIR + 1], s[DIR + 2]]
}

/// `u^T N u`.
fn quad(s: &Shape, u: &[f32; 3]) -> f32 {
    s[0] * u[0] * u[0]
        + s[1] * u[1] * u[1]
        + s[2] * u[2] * u[2]
        + 2.0 * (s[3] * u[0] * u[1] + s[4] * u[0] * u[2] + s[5] * u[1] * u[2])
}

/// Distance from the centre of the cell to its surface along the unit vector `u`.
#[inline]
pub fn extent(s: &Shape, u: &[f32; 3]) -> f32 {
    1.0 / quad(s, u).sqrt()
}

fn det(s: &Shape) -> f32 {
    let [a, b, c, d, e, f] = [s[0], s[1], s[2], s[3], s[4], s[5]];
    a * (b * c - f * f) - d * (d * c - f * e) + e * (d * f - b * e)
}

/// Radius of the sphere of the same volume.
pub fn radius(s: &Shape) -> f32 {
    det(s).powf(-1.0 / 6.0)
}

/// Smallest and largest eigenvalues of `N`. The half-axes of the ellipsoid are `1 / sqrt` of the
/// eigenvalues, so the smallest one gives the half-length and the largest one the half-width.
///
/// Cyclic Jacobi rotations, a fixed number of sweeps (deterministic, mirrored in WGSL). Unlike the
/// closed trigonometric form, it stays accurate when two axes are equal, which is the shape of
/// every freshly divided cell.
pub fn eigen_extremes(s: &Shape) -> (f32, f32) {
    let mut a = [[s[0], s[3], s[4]], [s[3], s[1], s[5]], [s[4], s[5], s[2]]];
    for _ in 0..JACOBI_SWEEPS {
        for (p, q, r) in [(0, 1, 2), (0, 2, 1), (1, 2, 0)] {
            rotate(&mut a, p, q, r);
        }
    }
    let d = [a[0][0], a[1][1], a[2][2]];
    (d[0].min(d[1]).min(d[2]), d[0].max(d[1]).max(d[2]))
}

/// Sweeps of Jacobi rotations: 4 bring a 3x3 matrix to f32 precision.
const JACOBI_SWEEPS: usize = 4;

/// One Jacobi rotation zeroing `a[p][q]` (`r` is the third index).
fn rotate(a: &mut [[f32; 3]; 3], p: usize, q: usize, r: usize) {
    let apq = a[p][q];
    if apq == 0.0 {
        return;
    }
    let theta = (a[q][q] - a[p][p]) / (2.0 * apq);
    let t = (if theta < 0.0 { -1.0 } else { 1.0 }) / (theta.abs() + (theta * theta + 1.0).sqrt());
    let c = 1.0 / (t * t + 1.0).sqrt();
    let s = t * c;
    a[p][p] -= t * apq;
    a[q][q] += t * apq;
    a[p][q] = 0.0;
    a[q][p] = 0.0;
    let (arp, arq) = (a[r][p], a[r][q]);
    a[r][p] = c * arp - s * arq;
    a[p][r] = a[r][p];
    a[r][q] = s * arp + c * arq;
    a[q][r] = a[r][q];
}

/// Recomputes the cached half-length after `N` changed.
fn settle(mut s: Shape) -> Shape {
    s[HALF] = 1.0 / eigen_extremes(&s).0.max(1e-12).sqrt();
    s
}

/// Even growth: the radius of the same-volume sphere goes from `r` to `k r` (k >= 1), and the new
/// wall is laid the same way everywhere. Pressed by the water inside, such a wall rounds the cell
/// off: the share of new volume goes to a sphere, the old volume keeps its shape. A flat cell
/// fresh from division thus rounds up as it grows; a cell that does not grow keeps its shape.
pub fn grow_even(s: &Shape, k: f32) -> Shape {
    if k <= 1.0 {
        return *s;
    }
    let added = 1.0 - 1.0 / (k * k * k);
    let d = det(s);
    let round = d.powf(1.0 / 3.0);
    let mut out = *s;
    for (i, v) in out[..6].iter_mut().enumerate() {
        *v = (1.0 - added) * *v + if i < 3 { added * round } else { 0.0 };
    }
    // Back to the target volume: det(N) = d / k^6.
    let fix = (d / (k * k * k * k * k * k * det(&out))).powf(1.0 / 3.0);
    for v in &mut out[..6] {
        *v *= fix;
    }
    settle(out)
}

/// Grows the cell by the factor `1 + eps` along the unit vector `g` only (its volume grows by the
/// same factor). When `eps` is negative the cell shrinks along `g`.
pub fn grow_along(s: &Shape, g: &[f32; 3], eps: f32) -> Shape {
    // New N = F^-1 N F^-1 with F = I + eps g g^T, and F^-1 = I + c g g^T.
    let c = -eps / (1.0 + eps);
    let v = [
        s[0] * g[0] + s[3] * g[1] + s[4] * g[2],
        s[3] * g[0] + s[1] * g[1] + s[5] * g[2],
        s[4] * g[0] + s[5] * g[1] + s[2] * g[2],
    ];
    let w = dot(g, &v);
    let entry = |i: usize, j: usize| c * (g[i] * v[j] + v[i] * g[j]) + c * c * w * g[i] * g[j];
    let mut out = *s;
    for (k, (i, j)) in [(0, 0), (1, 1), (2, 2), (0, 1), (0, 2), (1, 2)]
        .into_iter()
        .enumerate()
    {
        out[k] += entry(i, j);
    }
    settle(out)
}

/// The vector `x` stretched like the cell: `N^-1 x`, up to a positive factor. It leans toward the
/// long axis of the cell, the more so the longer the cell.
pub fn stretched(s: &Shape, x: &[f32; 3]) -> [f32; 3] {
    let [a, b, c, d, e, f] = [s[0], s[1], s[2], s[3], s[4], s[5]];
    let adj = [
        b * c - f * f,
        e * f - d * c,
        d * f - b * e,
        a * c - e * e,
        d * e - a * f,
        a * b - d * d,
    ];
    [
        adj[0] * x[0] + adj[1] * x[1] + adj[2] * x[2],
        adj[1] * x[0] + adj[3] * x[1] + adj[4] * x[2],
        adj[2] * x[0] + adj[4] * x[1] + adj[5] * x[2],
    ]
}

/// Replaces the growth direction.
pub fn with_dir(s: &Shape, g: [f32; 3]) -> Shape {
    let mut out = *s;
    out[DIR..DIR + 3].copy_from_slice(&g);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sphere_has_the_same_extent_everywhere() {
        let s = sphere(0.8, [0.0, 1.0, 0.0]);
        assert!((extent(&s, &[0.6, 0.8, 0.0]) - 0.8).abs() < 1e-6);
        assert!((radius(&s) - 0.8).abs() < 1e-5);
        assert!((s[HALF] - 0.8).abs() < 1e-6);
    }

    #[test]
    fn growing_along_a_direction_lengthens_only_that_way_and_adds_volume() {
        let g = [0.0, 0.6, 0.8];
        let s = grow_along(&sphere(1.0, g), &g, 1.0);
        assert!((extent(&s, &g) - 2.0).abs() < 1e-5);
        assert!((extent(&s, &[1.0, 0.0, 0.0]) - 1.0).abs() < 1e-5);
        assert!((radius(&s) - 2f32.cbrt()).abs() < 1e-5);
        assert!((s[HALF] - 2.0).abs() < 1e-4);
        // Growth in a new direction keeps the old elongation: nothing turns.
        let t = grow_along(&s, &[1.0, 0.0, 0.0], 0.5);
        assert!((extent(&t, &g) - 2.0).abs() < 1e-4 && (extent(&t, &[1.0, 0.0, 0.0]) - 1.5).abs() < 1e-4);
    }

    #[test]
    fn even_growth_rounds_a_flat_cell_and_keeps_a_sphere_round() {
        let g = [0.0, 0.0, 1.0];
        let flat = grow_along(&sphere(1.0, g), &g, -0.5);
        let grown = grow_even(&flat, 1.25);
        assert!((radius(&grown) - 1.25 * radius(&flat)).abs() < 1e-4);
        let ratio = |s: &Shape| extent(s, &[1.0, 0.0, 0.0]) / extent(s, &g);
        assert!(
            ratio(&grown) < 0.8 * ratio(&flat),
            "{} {}",
            ratio(&flat),
            ratio(&grown)
        );
        let ball = grow_even(&sphere(0.8, g), 1.1);
        assert!((extent(&ball, &[0.6, 0.8, 0.0]) - 0.88).abs() < 1e-5);
    }

    #[test]
    fn halving_along_the_length_halves_the_volume() {
        let g = [1.0, 0.0, 0.0];
        let s = grow_along(&sphere(1.0, g), &g, 2.0);
        let h = grow_along(&s, &g, -0.5);
        assert!((extent(&h, &g) - 1.5).abs() < 1e-5);
        assert!((radius(&h) - radius(&s) * crate::params::INV_CBRT2).abs() < 1e-5);
    }

    #[test]
    fn the_half_length_is_exact_for_flat_and_long_cells() {
        let g = [0.0, 0.6, 0.8];
        // A sphere halved across g: a flat cell with two equal long axes.
        let flat = grow_along(&sphere(1.0, g), &g, -0.5);
        assert!((flat[HALF] - 1.0).abs() < 1e-6, "{}", flat[HALF]);
        let long = grow_along(&sphere(0.5, g), &g, 3.0);
        assert!((long[HALF] - 2.0).abs() < 1e-5, "{}", long[HALF]);
    }

    #[test]
    fn a_stretched_vector_leans_toward_the_long_axis() {
        let g = [0.0, 0.0, 1.0];
        let s = grow_along(&sphere(1.0, g), &g, 1.0);
        let v = stretched(&s, &[1.0, 0.0, 1.0]);
        assert!(v[2] > 3.9 * v[0], "{v:?}");
    }
}
