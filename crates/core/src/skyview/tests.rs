use super::*;
use crate::grid::tests::{building, site, square, value_at};
use crate::sources::raster::Grid;

/// Johnson & Watson's sky view factor at the middle of an endless canyon as
/// tall as it is wide: cos(atan(2 H / W)) = cos(atan(2)).
const CANYON_FACTOR: f64 = 0.447;

fn factor_at_point(grid: &SkyGrid, x: f64, y: f64) -> Option<f64> {
    value_at(&grid.raster, &grid.factors, x, y)
}

/// Two 20 m walls 20 m apart along the y axis, running far past the site.
fn canyon() -> Vec<Feature> {
    vec![
        building(square(-40.0, -400.0, -10.0, 400.0), 20.0),
        building(square(10.0, -400.0, 40.0, 400.0), 20.0),
    ]
}

#[test]
fn open_ground_sees_the_whole_sky() {
    let grid = sky_view(&site(Vec::new(), Vec::new()));
    assert_eq!(factor_at_point(&grid, 0.0, 0.0), Some(1.0));
}

#[test]
fn the_middle_of_a_square_canyon_sees_what_johnson_and_watson_give() {
    let grid = sky_view(&site(Vec::new(), canyon()));
    let factor = factor_at_point(&grid, 1.0, 0.0).unwrap();

    // Sampled on a 2 m grid and searched to 150 m, so close rather than exact.
    assert!((factor - CANYON_FACTOR).abs() < 0.04, "{factor}");
}

#[test]
fn ground_under_a_roof_has_no_factor() {
    let grid = sky_view(&site(
        vec![building(square(-20.0, -20.0, 20.0, 0.0), 10.0)],
        Vec::new(),
    ));
    assert_eq!(factor_at_point(&grid, 0.0, -10.0), None);
}

#[test]
fn rising_terrain_hides_part_of_the_sky() {
    // Flat at 0 m west of x = 20, a 30 m bank east of it.
    let width = 200;
    let values = (0..width * width)
        .map(|index| if index % width >= 120 { 30.0 } else { 0.0 })
        .collect();
    let mut scene = site(Vec::new(), Vec::new());
    scene.terrain = Some(Grid {
        west: -100.0,
        north: 100.0,
        cell_m: 1.0,
        width,
        height: width,
        values,
    });
    let grid = sky_view(&scene);

    let below = factor_at_point(&grid, 10.0, 0.0).unwrap();
    let far = factor_at_point(&grid, -90.0, 0.0).unwrap();
    assert!(below < 0.8, "{below}");
    assert!(far > below, "{far} against {below}");
}

#[test]
fn bands_are_sky_view_and_carry_their_lower_edge() {
    let grid = sky_view(&site(Vec::new(), canyon()));
    let bands = bands(&grid, &[0.2, 0.4, 0.6, 0.8, 0.9]);
    let values: Vec<f64> = bands
        .iter()
        .map(|band| band.attributes.value.unwrap())
        .collect();

    assert!(values.contains(&0.4));
    assert!(values.contains(&0.9));
    assert!(bands.iter().all(|band| band.layer == Layer::SkyView));
}
