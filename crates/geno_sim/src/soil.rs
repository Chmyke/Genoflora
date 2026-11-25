//! The ground the plant grows in: what the environment is at a point, never what a cell does.
//!
//! The soil fills everything below a flat surface. Heights are measured against gravity from the
//! origin, where the seed starts. Across the surface the soil thins out over one dividing radius,
//! so that nothing it acts on jumps (a jump would amplify rounding differences).

use crate::params::Params;

/// Height of a point, against gravity.
fn height(p: &Params, at: &[f32]) -> f32 {
    -(at[0] * p.gravity[0] + at[1] * p.gravity[1] + at[2] * p.gravity[2])
}

/// How much of a point is soil: 1 deep down, 0 in the air, a linear ramp one dividing radius
/// wide centred on the surface.
#[inline]
pub fn share(p: &Params, at: &[f32]) -> f32 {
    ((p.surface - height(p, at)) / p.r_div + 0.5).clamp(0.0, 1.0)
}

/// Light reaching a point, from 1 in the open air: the soil dims it by a factor e every
/// `light_depth` of depth. No rule uses it yet.
pub fn light(p: &Params, at: &[f32]) -> f32 {
    let depth = (p.surface - height(p, at)).max(0.0);
    (-depth / p.light_depth).exp()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_soil_lies_below_the_surface_and_thins_out_smoothly() {
        let p = Params::default();
        let at = |h: f32| [0.0, h, 0.0];
        assert_eq!(share(&p, &at(p.surface - 5.0)), 1.0);
        assert_eq!(share(&p, &at(p.surface + 5.0)), 0.0);
        assert!((share(&p, &at(p.surface)) - 0.5).abs() < 1e-6);
        // No jump across the surface.
        let step = 1e-3;
        let mut h = p.surface - 2.0;
        while h < p.surface + 2.0 {
            assert!((share(&p, &at(h)) - share(&p, &at(h + step))).abs() <= 2.0 * step);
            h += step;
        }
        // The seed starts in the soil.
        assert_eq!(share(&p, &[0.0; 3]), 1.0);
    }

    #[test]
    fn the_surface_follows_gravity() {
        let p = Params {
            gravity: [1.0, 0.0, 0.0],
            ..Params::default()
        };
        assert_eq!(share(&p, &[-(p.surface + 5.0), 0.0, 0.0]), 0.0);
        assert_eq!(share(&p, &[5.0, 0.0, 0.0]), 1.0);
    }

    #[test]
    fn light_fades_with_depth() {
        let p = Params::default();
        assert_eq!(light(&p, &[0.0, p.surface + 1.0, 0.0]), 1.0);
        let deep = light(&p, &[0.0, p.surface - p.light_depth, 0.0]);
        assert!((deep - (-1.0f32).exp()).abs() < 1e-6, "{deep}");
    }
}
