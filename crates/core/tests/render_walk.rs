//! The OpenStreetMap diagrams on the made-up site, with a walk added by hand.

mod render_support;

use geo::{Geometry, MultiPolygon, Point};
use openarchi_core::render::{render_svg, RenderInput};
use openarchi_core::scene::{Attributes, Credit, Feature, Layer, Scene, Walk};
use openarchi_core::walk::neighbourhood;
use render_support::{circle, diagram, look, scene, theme};

fn place(layer: Layer, group: &str, (x, y): (f64, f64), walk_min: f64) -> Feature {
    Feature {
        layer,
        geometry: Geometry::Point(Point::new(x, y)),
        attributes: Attributes {
            group: Some(group.to_string()),
            name: Some(format!("{group} at {x}")),
            walk_min: Some(walk_min),
            ..Attributes::default()
        },
    }
}

fn band(minutes: f64, radius: f64) -> Feature {
    Feature {
        layer: Layer::WalkBand,
        geometry: Geometry::MultiPolygon(MultiPolygon::new(vec![circle(radius)])),
        attributes: Attributes {
            walk_min: Some(minutes),
            ..Attributes::default()
        },
    }
}

/// The test site with two shops on it, and a walk reaching 1 km with a
/// school, a tram stop at 8 minutes and one at 14.
fn walk_scene() -> Scene {
    let mut scene = scene();
    scene
        .features
        .push(place(Layer::Business, "shop", (10.0, 10.0), 0.5));
    scene
        .features
        .push(place(Layer::Business, "shop", (100.0, -50.0), 1.5));
    let mut tram = place(Layer::TransitStop, "tram", (600.0, 0.0), 8.0);
    tram.attributes.lines = vec!["Tram 13".to_string(), "Tram 17".to_string()];
    scene.walk = Some(Walk {
        bands: vec![band(5.0, 400.0), band(10.0, 750.0), band(15.0, 1000.0)],
        paths: vec![],
        places: vec![
            place(Layer::Facility, "education", (300.0, 300.0), 6.0),
            place(Layer::Facility, "bicycle_parking", (50.0, 300.0), 3.0),
            tram,
            place(Layer::TransitStop, "bus", (-900.0, 0.0), 14.0),
        ],
    });
    scene.credits.push(Credit {
        name: "© OpenStreetMap contributors".to_string(),
        license: "ODbL 1.0".to_string(),
        url: "https://www.openstreetmap.org/copyright".to_string(),
        layers: vec![
            Layer::Business,
            Layer::Facility,
            Layer::TransitStop,
            Layer::WalkBand,
        ],
    });
    scene
}

fn svg_for(scene: &Scene, diagram_id: &str) -> String {
    let (diagram, theme) = (diagram(diagram_id), theme("paper"));
    render_svg(&RenderInput {
        scene,
        diagram: &diagram,
        theme: &theme,
        look: &look(),
    })
    .expect("render")
}

#[test]
fn the_osm_credit_shows_only_on_sheets_that_draw_osm_data() {
    let scene = walk_scene();
    assert!(svg_for(&scene, "business").contains("OpenStreetMap contributors"));
    assert!(!svg_for(&scene, "figure-ground").contains("OpenStreetMap contributors"));
}

#[test]
fn the_walkability_plan_zooms_out_to_the_whole_walk() {
    let svg = svg_for(&walk_scene(), "walkability");

    // A 1 km walk does not fit an A3 drawing at 1:2500.
    assert!(!svg.contains("1:2500"), "the scale note still says 1:2500");
    assert!(svg.contains("Within 5 minutes on foot"));
    assert!(svg.contains("10 to 15 minutes"));
    assert!(svg.contains("15 min"));
}

#[test]
fn the_transit_plan_labels_its_stops_and_lists_their_lines() {
    let svg = svg_for(&walk_scene(), "transit");

    assert!(svg.contains("tram at 600"));
    assert!(svg.contains("Tram 13 · 17"));
    // The bus stop is 14 minutes away, outside the 10 minute walk.
    assert!(!svg.contains("bus at -900"));
}

#[test]
fn walk_diagrams_without_a_walk_stay_at_the_site() {
    let svg = svg_for(&scene(), "walkability");
    assert!(svg.contains("1:2500"));
}

#[test]
fn the_neighbourhood_counts_what_lies_within_each_band() {
    let neighbourhood = neighbourhood(&walk_scene()).expect("a walk");
    let education = |band: usize| neighbourhood.bands[band].facilities[0].count;
    let trams = |band: usize| neighbourhood.bands[band].transit_stops[2].count;

    assert_eq!((education(0), education(1)), (0, 1));
    assert_eq!((trams(0), trams(1), trams(2)), (0, 1, 1));
    // Bicycle parking is counted as parking, never as a facility.
    let band = &neighbourhood.bands[0];
    assert_eq!(band.parking[1].group, "bicycle_parking");
    assert_eq!(band.parking[1].count, 1);
    assert!(band
        .facilities
        .iter()
        .all(|group| group.group != "bicycle_parking"));
}
