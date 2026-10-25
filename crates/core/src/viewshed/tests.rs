use geo::{polygon, Contains, Geometry, Point};

use super::*;
use crate::area::{Area as SiteArea, LonLat};

fn square(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Polygon<f64> {
    polygon![
        (x: min_x, y: min_y),
        (x: max_x, y: min_y),
        (x: max_x, y: max_y),
        (x: min_x, y: max_y),
    ]
}

fn building(footprint: Polygon<f64>, height_m: f64) -> Feature {
    Feature {
        layer: Layer::Building,
        geometry: Geometry::Polygon(footprint),
        attributes: Attributes {
            height_m: Some(height_m),
            ..Attributes::default()
        },
    }
}

/// A 200 m square site in Amsterdam, on flat ground.
fn site(features: Vec<Feature>) -> Scene {
    Scene {
        area: SiteArea::Circle {
            center: LonLat {
                lon: 4.9041,
                lat: 52.3676,
            },
            radius_m: 100.0,
        },
        crs: "EPSG:28992".to_string(),
        origin: (120_000.0, 487_000.0),
        boundary: square(-100.0, -100.0, 100.0, 100.0),
        features,
        credits: Vec::new(),
        fetched_at: String::new(),
        warnings: Vec::new(),
        measured: Default::default(),
        walk: None,
        editions: Default::default(),
        context: Vec::new(),
        terrain: None,
        water_bed: None,
        water_levels: Vec::new(),
        solar: None,
        wind: None,
    }
}

fn site_extent() -> Rect<f64> {
    Rect::new((-100.0, -100.0), (100.0, 100.0))
}

fn is_seen(view: &Viewshed, x: f64, y: f64) -> bool {
    let Geometry::MultiPolygon(area) = view.area().geometry else {
        unreachable!("the viewshed area is always a multipolygon");
    };
    area.0.iter().any(|part| part.contains(&Point::new(x, y)))
}

/// A 4 m thick wall of `height_m` across the site along x = 0.
fn wall(height_m: f64) -> Feature {
    building(square(0.0, -100.0, 4.0, 100.0), height_m)
}

#[test]
fn open_flat_ground_is_all_in_view() {
    let view = viewshed(&site(Vec::new()), Some((-50.0, 0.0)), site_extent());

    assert_eq!(view.share_of_site(), Some(1.0));
    assert!(!view.is_on_roof);
}

#[test]
fn a_wall_hides_the_ground_behind_it() {
    let view = viewshed(&site(vec![wall(10.0)]), Some((-50.0, 0.0)), site_extent());

    assert!(is_seen(&view, -20.0, 30.0));
    assert!(!is_seen(&view, 30.0, 0.0));
    assert!(!is_seen(&view, 80.0, -40.0));
    let share = view.share_of_site().unwrap();
    assert!((0.45..0.55).contains(&share), "{share}");
}

#[test]
fn a_wall_below_the_eye_hides_only_a_strip_behind_it() {
    // A 0.5 m wall seen from 1.5 m up, 50 m off, hides the ground 25 m behind it.
    let view = viewshed(&site(vec![wall(0.5)]), Some((-50.0, 0.0)), site_extent());

    assert!(!is_seen(&view, 15.0, 0.0));
    assert!(is_seen(&view, 40.0, 0.0));
}

#[test]
fn an_eye_on_a_roof_sees_over_the_wall() {
    let view = viewshed(&site(vec![wall(10.0)]), Some((2.0, 0.0)), site_extent());

    assert!(view.is_on_roof);
    assert!(is_seen(&view, 60.0, 0.0));
    assert!(is_seen(&view, -60.0, 0.0));
}

#[test]
fn the_default_eye_steps_out_of_a_building_at_the_centre() {
    let block = building(square(-10.0, -10.0, 10.0, 10.0), 20.0);
    let view = viewshed(&site(vec![block]), None, site_extent());

    assert!(!view.is_on_roof);
    let (x, y) = view.eye;
    let distance_out = x.abs().max(y.abs());
    assert!((10.0..13.0).contains(&distance_out), "{x}, {y}");
}

#[test]
fn a_large_extent_coarsens_the_grid() {
    let wide = Rect::new((-2000.0, -2000.0), (2000.0, 2000.0));
    let view = viewshed(&site(Vec::new()), None, wide);

    assert!(view.raster.len() as f64 <= MAX_CELLS * 1.02);
    assert!(view.raster.cell_m > FINEST_CELL_M);
}

#[test]
fn a_viewpoint_off_the_drawn_area_is_refused() {
    let scene = site(Vec::new());

    assert!(check_view_point(&scene, (120_010.0, 487_020.0)).is_ok());
    assert!(check_view_point(&scene, (125_000.0, 487_000.0)).is_err());
    assert!(check_view_point(&scene, (f64::NAN, 487_000.0)).is_err());
}
