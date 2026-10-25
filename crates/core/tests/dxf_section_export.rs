//! The DXF of a section sheet read back: in metres along the line and metres
//! NAP, with the cut, what stands beyond, the ground, water and the datum.

mod dxf_support;
mod render_support;

use dxf_support::{dxf_of, through_the_row};
use geo::{polygon, Geometry, LineString};
use openarchi_core::project::Look;
use openarchi_core::scene::{Attributes, Feature, Layer, Scene, WaterLevel};
use openarchi_core::sources::raster::Grid;
use openarchi_core::view::ViewMode;
use render_support::{look, scene};

fn section_look() -> Look {
    let mut look = look();
    look.view.mode = ViewMode::Section;
    look.section = Some(through_the_row());
    look
}

/// Ground rising 1 m every 10 m eastwards, from NAP 0 at the west edge of the test site.
fn sloping_terrain() -> Grid {
    let size = 60;
    let mut values = Vec::with_capacity(size * size);
    for _ in 0..size {
        for column in 0..size {
            values.push(column as f32);
        }
    }
    Grid {
        west: -300.0,
        north: 300.0,
        cell_m: 10.0,
        width: size,
        height: size,
        values,
    }
}

#[test]
fn a_section_sheet_writes_its_section_in_metres_along_the_line_and_nap() {
    let scene = Scene {
        terrain: Some(sloping_terrain()),
        ..scene()
    };
    let dxf = dxf_of(&scene, "context", &section_look(), &[], &[]);

    // Five buildings cut, the first 50 to 70 m from A on ground near NAP +15.
    let cuts = dxf.on("OA-SECTION-CUT");
    assert_eq!(cuts.len(), 5);
    let first = &cuts[0].points;
    let xs: Vec<f64> = first.iter().map(|point| point.0).collect();
    assert_eq!((xs[0], xs[1]), (50.0, 70.0));
    let (bottom, top) = (first[0].1, first[2].1);
    assert!((14.0..=22.0).contains(&bottom), "bottom {bottom}");
    assert!(
        (top - bottom - 14.0).abs() < 2.0,
        "14 m tall, got {}",
        top - bottom
    );

    let ground = &dxf.on("OA-SECTION-GROUND")[0].points;
    assert!(ground.first().unwrap().0.abs() < 0.01);
    assert!((ground.last().unwrap().0 - 300.0).abs() < 0.01);
    assert!(
        ground.iter().all(|&(_, y)| (10.0..=50.0).contains(&y)),
        "{ground:?}"
    );
    assert!(dxf.on("OA-SECTION-ELEVATION").len() >= 5);
    // The two trees stand behind the viewer, who looks north.
    assert!(dxf.on("OA-SECTION-TREE").is_empty());
    let texts: Vec<&str> = dxf
        .on("OA-SECTION-TEXT")
        .iter()
        .map(|entity| entity.text.as_str())
        .collect();
    assert!(texts.contains(&"NAP 0.00 m"), "{texts:?}");
    assert_eq!(dxf.layer("OA-SECTION-NAP").unwrap().2, "DASHED");
    // Nothing of the plan comes along.
    assert!(dxf.on("OA-SITE").is_empty() && dxf.on("OA-BUILDING").is_empty());
}

#[test]
fn a_canal_in_the_section_is_written_at_its_target_level() {
    let mut scene = Scene {
        terrain: Some(sloping_terrain()),
        ..scene()
    };
    scene.features.push(Feature {
        layer: Layer::Water,
        geometry: Geometry::Polygon(polygon![
            (x: -140.0, y: 60.0),
            (x: -125.0, y: 60.0),
            (x: -125.0, y: 120.0),
            (x: -140.0, y: 120.0),
        ]),
        attributes: Attributes::default(),
    });
    scene.water_levels.push(WaterLevel {
        name: "Test boezem".to_string(),
        level_m: -0.4,
        line: LineString::from(vec![(-132.0, 0.0), (-132.0, 200.0)]),
    });
    let dxf = dxf_of(&scene, "context", &section_look(), &[], &[]);

    let surface = &dxf.on("OA-SECTION-WATER")[0].points;
    assert_eq!(surface, &[(10.0, -0.4), (25.0, -0.4)]);
    let texts: Vec<&str> = dxf
        .on("OA-SECTION-TEXT")
        .iter()
        .map(|entity| entity.text.as_str())
        .collect();
    assert!(texts.contains(&"Test boezem NAP -0.40 m"), "{texts:?}");
}

#[test]
fn a_section_without_terrain_says_its_zero_is_not_nap() {
    let dxf = dxf_of(&scene(), "context", &section_look(), &[], &[]);

    let texts: Vec<&str> = dxf
        .on("OA-SECTION-TEXT")
        .iter()
        .map(|entity| entity.text.as_str())
        .collect();
    assert!(!texts.contains(&"NAP 0.00 m"));
    assert!(dxf
        .on("OA-SECTION-CUT")
        .iter()
        .all(|cut| cut.points[0].1 == 0.0));
}

#[test]
fn trees_in_front_of_a_section_are_a_trunk_and_a_crown() {
    let mut look = section_look();
    // Looking south now, towards the two trees 40 m away.
    look.section = Some(through_the_row().flipped());
    let dxf = dxf_of(&scene(), "context", &look, &[], &[]);

    let tree = dxf.on("OA-SECTION-TREE");
    let kinds: Vec<&str> = tree.iter().map(|entity| entity.kind.as_str()).collect();
    assert_eq!(kinds, ["POLYLINE", "CIRCLE", "POLYLINE", "CIRCLE"]);
}
