use geo::Point;

use super::*;
use crate::forest::tests::{canopy, wooded};
use crate::grid::tests::{building, site, square, value_at};
use crate::scene::Attributes;

const EQUINOX: (u32, u32) = (3, 21);
const MIDSUMMER: (u32, u32) = (6, 21);

fn hours_at(grid: &SunGrid, x: f64, y: f64) -> Option<f64> {
    value_at(&grid.raster, &grid.hours, x, y)
}

/// A 40 m wide, 30 m tall block whose north face runs along y = 0.
fn tall_block() -> Feature {
    building(square(-20.0, -20.0, 20.0, 0.0), 30.0)
}

#[test]
fn open_ground_sees_the_sun_all_day() {
    let grid = sun_hours(&site(Vec::new(), Vec::new()), EQUINOX.0, EQUINOX.1);
    let hours = hours_at(&grid, 0.0, 0.0).unwrap();

    // About twelve hours on the equinox, less the low sun at either end.
    assert!((11.0..=12.2).contains(&hours), "{hours}");
}

#[test]
fn a_tall_building_to_the_south_takes_the_midday_sun() {
    let grid = sun_hours(&site(vec![tall_block()], Vec::new()), EQUINOX.0, EQUINOX.1);
    let behind = hours_at(&grid, 0.0, 10.0).unwrap();
    let in_front = hours_at(&grid, 0.0, -40.0).unwrap();

    // 10 m north of a 30 m wall the noon sun, at 38 degrees, is blocked for hours.
    assert!(behind < 6.0, "{behind}");
    assert!(in_front > 11.0, "{in_front}");
}

#[test]
fn the_shade_behind_a_building_shrinks_in_summer() {
    let scene = site(vec![tall_block()], Vec::new());
    let spring = hours_at(&sun_hours(&scene, EQUINOX.0, EQUINOX.1), 0.0, 30.0).unwrap();
    let summer = hours_at(&sun_hours(&scene, MIDSUMMER.0, MIDSUMMER.1), 0.0, 30.0).unwrap();

    assert!(summer > spring + 3.0, "{spring} in March, {summer} in June");
}

#[test]
fn ground_under_a_roof_has_no_hours() {
    let grid = sun_hours(&site(vec![tall_block()], Vec::new()), EQUINOX.0, EQUINOX.1);
    assert_eq!(hours_at(&grid, 0.0, -10.0), None);
}

#[test]
fn a_building_outside_the_site_still_casts_its_shadow_in() {
    // The tall block moved south of the site, in the neighbourhood.
    let outside = building(square(-20.0, -130.0, 20.0, -110.0), 30.0);
    let grid = sun_hours(&site(Vec::new(), vec![outside]), EQUINOX.0, EQUINOX.1);
    let open = sun_hours(&site(Vec::new(), Vec::new()), EQUINOX.0, EQUINOX.1);

    let shaded = hours_at(&grid, 0.0, -100.0 + 1.0).unwrap();
    let unshaded = hours_at(&open, 0.0, -100.0 + 1.0).unwrap();
    assert!(shaded < unshaded - 2.0, "{shaded} against {unshaded}");
}

#[test]
fn a_tree_shades_the_ground_behind_it() {
    let tree = Feature {
        layer: Layer::Tree,
        geometry: Geometry::Point(Point::new(0.0, 0.0)),
        attributes: Attributes::default(),
    };
    let grid = sun_hours(&site(vec![tree], Vec::new()), EQUINOX.0, EQUINOX.1);
    let open = sun_hours(&site(Vec::new(), Vec::new()), EQUINOX.0, EQUINOX.1);

    assert!(hours_at(&grid, 0.0, 8.0).unwrap() < hours_at(&open, 0.0, 8.0).unwrap());
}

#[test]
fn a_wood_shades_the_ground_behind_it_and_under_it() {
    // 100 m by 60 m of 20 m trees, south of the middle of the site: a wood by the FAO rule.
    let wood = canopy(&[(-50.0, -70.0, 50.0, -10.0)], 20.0);
    let grid = sun_hours(
        &wooded(site(Vec::new(), Vec::new()), wood),
        EQUINOX.0,
        EQUINOX.1,
    );
    let open = sun_hours(&site(Vec::new(), Vec::new()), EQUINOX.0, EQUINOX.1);

    let behind = hours_at(&grid, 0.0, -5.0).unwrap();
    let open_behind = hours_at(&open, 0.0, -5.0).unwrap();
    assert!(behind < open_behind - 2.0, "{behind} against {open_behind}");
    assert_eq!(hours_at(&grid, 0.0, -40.0), Some(0.0));
}

#[test]
fn bands_carry_the_lower_edge_of_their_class() {
    let grid = sun_hours(&site(vec![tall_block()], Vec::new()), EQUINOX.0, EQUINOX.1);
    let bands = bands(&grid, &[2.0, 4.0, 8.0]);
    let values: Vec<f64> = bands
        .iter()
        .map(|band| band.attributes.value.unwrap())
        .collect();

    assert!(values.contains(&8.0));
    assert!(values
        .iter()
        .all(|value| [0.0, 2.0, 4.0, 8.0].contains(value)));
    assert!(bands.iter().all(|band| band.layer == Layer::SunHours));
}
