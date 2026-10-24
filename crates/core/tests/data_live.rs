//! Tests against the real PDOK services. Ignored by default because they need
//! the network and PDOK's data changes over time; run them with
//! `cargo test -p openarchi-core --test data_live -- --ignored`.

use openarchi_core::area::{Area, LonLat};
use openarchi_core::geocode;
use openarchi_core::scene::Layer;
use openarchi_core::site::build_scene;

const WESTERMARKT: LonLat = LonLat {
    lon: 4.8840,
    lat: 52.3745,
};

#[test]
#[ignore = "needs the network"]
fn a_small_site_in_amsterdam_has_every_source_and_no_warnings() {
    let area = Area::Circle {
        center: WESTERMARKT,
        radius_m: 100.0,
    };
    let scene = build_scene(&area, &|_| {}).unwrap();

    assert!(scene.warnings.is_empty(), "{:?}", scene.warnings);
    assert_eq!(scene.credits.len(), 4);
    for layer in [
        Layer::Building,
        Layer::Road,
        Layer::Water,
        Layer::Parcel,
        Layer::Contour,
    ] {
        assert!(
            scene.features.iter().any(|feature| feature.layer == layer),
            "no {layer:?}"
        );
    }
}

#[test]
#[ignore = "needs the network"]
fn a_rectangle_works_as_well_as_a_circle() {
    let area = Area::Rectangle {
        south_west: LonLat {
            lon: 4.8820,
            lat: 52.3735,
        },
        north_east: LonLat {
            lon: 4.8860,
            lat: 52.3755,
        },
    };
    let scene = build_scene(&area, &|_| {}).unwrap();
    assert!(scene
        .features
        .iter()
        .any(|feature| feature.layer == Layer::Building));
}

#[test]
#[ignore = "needs the network"]
fn westermarkt_20_is_found_at_its_address() {
    let places = geocode::search("Westermarkt 20 Amsterdam", 3).unwrap();
    let first = &places[0];
    assert_eq!(first.label, "Westermarkt 20, 1016DK Amsterdam");
    assert_eq!(first.kind, "adres");
    assert!((first.location.lon - 4.8843).abs() < 0.001);
    assert!((first.location.lat - 52.3749).abs() < 0.001);
}
