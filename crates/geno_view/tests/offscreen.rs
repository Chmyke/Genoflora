//! The renderer checked by numbers, off screen, on the software adapter: a known cell must show
//! at the right place, with the right size and colour.

use geno_gpu::{Adapter, GpuSim};
use geno_sim::shape::{grow_along, sphere};
use geno_sim::{Cells, EngineError, Params};
use geno_view::{Camera, Look, Mode, Renderer, STILL, still};

const SIZE: u32 = 200;

/// A soil surface so far below that it stays out of the picture.
fn no_ground() -> Params {
    Params {
        surface: -1000.0,
        ..Params::default()
    }
}

/// One cell of radius 1 at the origin, seen level from 20 away.
fn scene(youth: f32, stretch: f32) -> (Cells, Look) {
    let centre = [0.0; 3];
    let mut cells = Cells::empty(4);
    cells.count = 1;
    cells.pr[0] = [centre[0], centre[1], centre[2], 1.0];
    cells.shape[0] = grow_along(&sphere(1.0, [0.0, 1.0, 0.0]), &[0.0, 1.0, 0.0], stretch);
    cells.youth[0] = youth;
    let look = Look {
        mode: Mode::Youth,
        ..Look::new(Camera {
            target: centre,
            yaw: 0.0,
            pitch: 0.0,
            distance: 20.0,
        })
    };
    (cells, look)
}

/// One picture at a time: creating several software devices at once crashed the test binary
/// (access violation, 2026-11-25), like the hang noted in geno_gpu's tests.
fn picture(p: &Params, cells: &Cells, look: &Look) -> Result<Vec<u8>, EngineError> {
    static ONE_AT_A_TIME: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _turn = ONE_AT_A_TIME
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut sim = GpuSim::new(p, cells, Adapter::Software)?;
    let renderer = Renderer::new(sim.device(), STILL);
    still(&mut sim, &renderer, p, look, SIZE, SIZE)
}

fn pixel(picture: &[u8], x: u32, y: u32) -> [u8; 4] {
    let at = ((y * SIZE + x) * 4) as usize;
    [picture[at], picture[at + 1], picture[at + 2], picture[at + 3]]
}

/// Width and height, in pixels, of what is not the air colour.
fn covered(picture: &[u8]) -> (u32, u32) {
    let air = pixel(picture, 2, 2);
    let (mut x, mut y) = ((SIZE, 0), (SIZE, 0));
    for py in 0..SIZE {
        for px in 0..SIZE {
            if pixel(picture, px, py) != air {
                x = (x.0.min(px), x.1.max(px));
                y = (y.0.min(py), y.1.max(py));
            }
        }
    }
    (x.1.saturating_sub(x.0) + 1, y.1.saturating_sub(y.0) + 1)
}

#[test]
fn a_young_cell_shows_yellow_in_the_middle_and_air_around() -> Result<(), EngineError> {
    let p = no_ground();
    let (cells, look) = scene(1.0, 0.0);
    let picture = picture(&p, &cells, &look)?;
    // Top of the scale is yellow (253, 231, 37), dimmed a little by the shading.
    let [r, g, b, _] = pixel(&picture, SIZE / 2, SIZE / 2);
    assert!(r > 180 && g > 160 && b < 90, "middle is {r} {g} {b}");
    assert_eq!(pixel(&picture, 3, 3), [246, 244, 238, 255]);
    Ok(())
}

#[test]
fn a_mature_cell_shows_dark_purple() -> Result<(), EngineError> {
    let p = no_ground();
    let (cells, look) = scene(0.0, 0.0);
    let picture = picture(&p, &cells, &look)?;
    let [r, g, b, _] = pixel(&picture, SIZE / 2, SIZE / 2);
    assert!(r < 90 && g < 40 && b > 50 && b > r, "middle is {r} {g} {b}");
    Ok(())
}

#[test]
fn a_cell_has_its_true_size_and_shape() -> Result<(), EngineError> {
    let p = no_ground();
    // The outline of a sphere of radius 1 seen from 20 away, in a 0.6 rad field of view.
    let wide = SIZE as f32 * (1.0f32 / 20.0).asin().tan() / 0.3f32.tan();
    let (cells, look) = scene(1.0, 0.0);
    let (w, h) = covered(&picture(&p, &cells, &look)?);
    assert!(
        (w as f32 - wide).abs() <= 2.5 && (h as f32 - wide).abs() <= 2.5,
        "{w} x {h}, expected {wide}"
    );
    // Twice as long upward: twice as tall, no wider.
    let (cells, look) = scene(1.0, 1.0);
    let (w2, h2) = covered(&picture(&p, &cells, &look)?);
    assert!(
        (h2 as f32 - 2.0 * wide).abs() <= 3.0 && (w2 as f32 - wide).abs() <= 2.5,
        "{w2} x {h2}"
    );
    Ok(())
}

#[test]
fn cutting_hides_the_near_half() -> Result<(), EngineError> {
    let p = no_ground();
    let (mut cells, look) = scene(1.0, 0.0);
    // Nudge the cell toward the camera (which stands on -x at yaw 0): it is on the near side.
    cells.pr[0][0] -= 0.5;
    let cut = Look { cut: true, ..look };
    let picture = picture(&p, &cells, &cut)?;
    assert!(
        picture
            .as_chunks::<4>()
            .0
            .iter()
            .all(|px| *px == [246, 244, 238, 255])
    );
    // Pushed to the far side, it shows again.
    cells.pr[0][0] += 1.0;
    let (w, _) = covered(&self::picture(&p, &cells, &cut)?);
    assert!(w > 20, "only {w} pixels wide");
    Ok(())
}
