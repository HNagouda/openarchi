//! Tests against the real PDOK services. Ignored by default because they need
//! the network and PDOK's data changes over time; run them with
//! `cargo test -p openarchi-core --test data_live -- --ignored`.

use openarchi_core::area::{Area, LonLat};
use openarchi_core::geocode;
use openarchi_core::scene::Layer;
use openarchi_core::site::build_scene;
use openarchi_core::sources::cache::TileCache;

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
    let folder = tempfile::TempDir::new().unwrap();
    let scene = build_scene(&area, &TileCache::at(folder.path()), &|_| {}).unwrap();

    assert!(scene.warnings.is_empty(), "{:?}", scene.warnings);
    // BGT, BAG, BRK and AHN, then RIVM, LIWO and CBS.
    assert_eq!(scene.credits.len(), 7);
    for layer in [
        Layer::Building,
        Layer::Road,
        Layer::Water,
        Layer::Parcel,
        Layer::Contour,
        Layer::RoadNoise,
        Layer::No2,
        Layer::Flood,
        Layer::Slope,
        Layer::Population,
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
    let folder = tempfile::TempDir::new().unwrap();
    let scene = build_scene(&area, &TileCache::at(folder.path()), &|_| {}).unwrap();
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

#[test]
#[ignore = "needs the network"]
fn the_rivm_and_liwo_rasters_answer_on_their_own_grids() {
    use openarchi_core::sources::{liwo, rivm, wcs};

    let tile = geo::Rect::new(
        geo::Coord {
            x: 120_500.0,
            y: 487_500.0,
        },
        geo::Coord {
            x: 120_750.0,
            y: 487_750.0,
        },
    );
    let noise = wcs::fetch(&rivm::ROAD_NOISE, tile).unwrap();
    assert_eq!((noise.width, noise.height, noise.cell_m), (25, 25, 10.0));
    assert!(noise
        .values
        .iter()
        .all(|value| (30.0..90.0).contains(value)));

    // The air quality grid starts 11.6 m east of the tile.
    let no2 = wcs::fetch(&rivm::NO2, tile).unwrap();
    assert_eq!(no2.cell_m, 25.0);
    assert!((no2.west - 120_511.6).abs() < 0.01, "{}", no2.west);

    let flood = wcs::fetch(&liwo::FLOOD_DEPTH, tile).unwrap();
    assert_eq!((flood.width, flood.cell_m), (50, 5.0));
    assert!(
        flood.values.iter().any(|value| value.is_nan()),
        "dry cells are no data"
    );
}
