use super::*;
use crate::skyview::tests::{building, site, square};

/// The tree raster's cell size, as RIVM serves it.
const CELL_M: f64 = 5.0;
/// Half the side of the test raster: 500 m across, centred on the site.
const HALF_SIDE_M: f64 = 250.0;

/// A tree raster with trees of `height_m` in each of `stands`, as
/// (min x, min y, max x, max y), and no tree elsewhere.
pub(crate) fn canopy(stands: &[(f64, f64, f64, f64)], height_m: f32) -> Grid {
    let side = (2.0 * HALF_SIDE_M / CELL_M) as usize;
    let mut grid = Grid {
        west: -HALF_SIDE_M,
        north: HALF_SIDE_M,
        cell_m: CELL_M,
        width: side,
        height: side,
        values: vec![f32::NAN; side * side],
    };
    for row in 0..side {
        for column in 0..side {
            let centre = grid.cell_centre(column, row);
            let is_stand = stands.iter().any(|&(min_x, min_y, max_x, max_y)| {
                centre.x > min_x && centre.x < max_x && centre.y > min_y && centre.y < max_y
            });
            if is_stand {
                grid.values[row * side + column] = height_m;
            }
        }
    }

    grid
}

/// The test site with `grid` kept as its tree raster.
pub(crate) fn wooded(mut scene: Scene, grid: Grid) -> Scene {
    scene.measured.grids.insert(Layer::TreeCanopy, grid);
    scene
}

#[test]
fn a_wood_of_a_hectare_is_forest_at_its_canopy_height() {
    let scene = wooded(
        site(Vec::new(), Vec::new()),
        canopy(&[(0.0, 0.0, 100.0, 100.0)], 15.0),
    );
    let forest = forest(&scene).unwrap();

    assert_eq!(forest.height_at((50.0, 50.0)), Some(15.0));
    assert_eq!(forest.height_at((-50.0, 50.0)), None);
}

#[test]
fn a_row_of_street_trees_is_too_narrow_to_be_forest() {
    // 10 m wide and 600 m long: 0.6 ha, but a strip under 20 m wide.
    let scene = wooded(
        site(Vec::new(), Vec::new()),
        canopy(&[(-240.0, 0.0, 240.0, 10.0)], 15.0),
    );
    assert!(forest(&scene).is_none());
}

#[test]
fn a_grove_under_half_a_hectare_is_not_forest() {
    // 60 by 60 m is 0.36 ha.
    let scene = wooded(
        site(Vec::new(), Vec::new()),
        canopy(&[(0.0, 0.0, 60.0, 60.0)], 15.0),
    );
    assert!(forest(&scene).is_none());
}

#[test]
fn shrubs_of_five_metres_or_less_are_not_forest() {
    let scene = wooded(
        site(Vec::new(), Vec::new()),
        canopy(&[(0.0, 0.0, 100.0, 100.0)], 5.0),
    );
    assert!(forest(&scene).is_none());
}

#[test]
fn a_tree_under_a_building_footprint_is_left_out() {
    // A hectare of 60 m "trees" that is in fact a roof.
    let roof = square(0.0, 0.0, 100.0, 100.0);
    let grid = canopy(&[(0.0, 0.0, 100.0, 100.0)], 60.0);
    let scene = wooded(
        site(vec![building(roof.clone(), 60.0)], Vec::new()),
        grid.clone(),
    );

    assert!(forest(&scene).is_none());
    let cleared = without_buildings(&grid, [&roof]);
    assert!(cleared.values.iter().all(|value| value.is_nan()));
}

#[test]
fn a_cell_a_facade_reaches_into_is_cleared_too() {
    // The cell from 0 to 5 m; the building covers its corner, not its centre.
    let grid = canopy(&[(0.0, 0.0, 5.0, 5.0)], 54.0);
    let tower = square(-10.0, -10.0, 1.0, 1.0);

    let cleared = without_buildings(&grid, [&tower]);
    assert!(cleared.values.iter().all(|value| value.is_nan()));
}

#[test]
fn a_tree_beside_a_building_stays() {
    let grid = canopy(&[(10.0, 0.0, 15.0, 5.0)], 12.0);
    let house = square(-10.0, -10.0, 9.0, 9.0);

    let kept = without_buildings(&grid, [&house]);
    assert_eq!(
        kept.values.iter().filter(|value| **value == 12.0).count(),
        1
    );
}

#[test]
fn a_scene_without_a_tree_raster_has_no_forest() {
    assert!(forest(&site(Vec::new(), Vec::new())).is_none());
}
