//! Checks on the recorded Westermarkt scene, so the fixture other parts of
//! openarchi render from keeps the shape the data layer promises.

use geo::{BoundingRect, Geometry};
use openarchi_core::scene::{Layer, Scene};

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/westermarkt.scene.json"
);
/// The fixture is a 300 m circle; rounding to centimetres may push a vertex just past it.
const RADIUS_M: f64 = 300.0;
const TOLERANCE_M: f64 = 0.05;

fn westermarkt() -> Scene {
    let json = std::fs::read_to_string(FIXTURE).expect("fixture is checked in");
    serde_json::from_str(&json).expect("fixture is a serialised Scene")
}

#[test]
fn the_fixture_is_in_rd_new_around_westermarkt() {
    let scene = westermarkt();
    assert_eq!(scene.crs, "EPSG:28992");
    assert!(
        (scene.origin.0 - 120_733.0).abs() < 5.0,
        "origin x {}",
        scene.origin.0
    );
    assert!(
        (scene.origin.1 - 487_522.0).abs() < 5.0,
        "origin y {}",
        scene.origin.1
    );
}

#[test]
fn every_feature_lies_inside_the_circle() {
    for feature in westermarkt().features {
        let rect = feature.geometry.bounding_rect().unwrap();
        for corner in [rect.min(), rect.max()] {
            assert!(
                corner.x.abs() <= RADIUS_M + TOLERANCE_M,
                "{:?} at {corner:?}",
                feature.layer
            );
            assert!(
                corner.y.abs() <= RADIUS_M + TOLERANCE_M,
                "{:?} at {corner:?}",
                feature.layer
            );
        }
    }
}

#[test]
fn the_main_layers_are_all_filled() {
    let scene = westermarkt();
    let expected = [
        Layer::Building,
        Layer::Road,
        Layer::Footpath,
        Layer::Water,
        Layer::Yard,
        Layer::Tree,
        Layer::Parcel,
        Layer::Contour,
    ];
    for layer in expected {
        assert!(
            scene.features.iter().any(|feature| feature.layer == layer),
            "no {layer:?}"
        );
    }
}

#[test]
fn nearly_every_building_has_a_plausible_height() {
    let scene = westermarkt();
    let buildings: Vec<_> = scene
        .features
        .iter()
        .filter(|feature| feature.layer == Layer::Building)
        .collect();
    let heights: Vec<f64> = buildings
        .iter()
        .filter_map(|building| building.attributes.height_m)
        .collect();
    assert!(heights.len() * 100 >= buildings.len() * 95);
    // The Westerkerk's nave is the tallest roof here; no house is above 40 m.
    assert!(heights.iter().all(|height| (-5.0..40.0).contains(height)));
}

#[test]
fn trees_are_points_and_tracks_and_contours_are_lines() {
    for feature in westermarkt().features {
        match feature.layer {
            Layer::Tree => assert!(matches!(feature.geometry, Geometry::Point(_))),
            Layer::RailTrack | Layer::Contour => assert!(matches!(
                feature.geometry,
                Geometry::LineString(_) | Geometry::MultiLineString(_)
            )),
            _ => {}
        }
    }
}

#[test]
fn every_source_is_credited() {
    let names: Vec<String> = westermarkt()
        .credits
        .into_iter()
        .map(|credit| credit.name)
        .collect();
    for source in ["BGT", "BAG", "BRK", "AHN"] {
        assert!(
            names.iter().any(|name| name.contains(source)),
            "{source} missing from {names:?}"
        );
    }
}
