//! The DXF of a plan sheet read back: which layers it declares and which
//! entities sit on them where, with diagrams, highlights, the section line,
//! pins and sheet areas.

mod dxf_support;
mod render_support;

use dxf_support::{dxf_of, through_the_row};
use openarchi_core::catalog::builtin_diagrams;
use openarchi_core::highlight::{Highlight, HighlightShape, Outline, DEFAULT_COLOUR};
use openarchi_core::pin::{Pin, PinIcon, PinSize};
use openarchi_core::project::Look;
use render_support::{look, scene, ORIGIN, SITE_RADIUS_M};

fn plot(label: &str, outline: Outline, x: f64, y: f64, side: f64) -> Highlight {
    Highlight {
        id: format!("highlight-{label}"),
        label: label.to_string(),
        shape: HighlightShape::Rectangle {
            min: [ORIGIN.0 + x, ORIGIN.1 + y],
            max: [ORIGIN.0 + x + side, ORIGIN.1 + y + side],
        },
        outline,
        colour: DEFAULT_COLOUR.to_string(),
        dims_rest: false,
        shows_area: true,
    }
}

#[test]
fn every_entity_of_every_built_in_diagram_sits_on_a_declared_layer() {
    let scene = scene();
    for built_in in builtin_diagrams() {
        for (variant, _) in built_in.drawings() {
            let look = Look {
                diagram_id: built_in.id.clone(),
                variant: variant.clone(),
                ..look()
            };
            let dxf = dxf_of(&scene, &built_in.id, &look, &[], &[]);
            for entity in &dxf.entities {
                assert!(
                    dxf.layer(&entity.layer).is_some(),
                    "{} {variant:?}: {} on undeclared layer {}",
                    built_in.id,
                    entity.kind,
                    entity.layer
                );
            }
        }
    }
}

#[test]
fn a_classifying_diagram_puts_each_legend_class_on_its_own_layer() {
    let look = Look {
        diagram_id: "building-height".to_string(),
        ..look()
    };
    let dxf = dxf_of(&scene(), "building-height", &look, &[], &[]);

    // 14, 18.5, 32 and 4 m, and one building without a height.
    assert_eq!(dxf.on("OA-BUILDING-10-15-M").len(), 1);
    assert_eq!(dxf.on("OA-BUILDING-15-20-M").len(), 1);
    assert_eq!(dxf.on("OA-BUILDING-30-M-AND-TALLER").len(), 1);
    assert_eq!(dxf.on("OA-BUILDING-UNDER-6-M").len(), 1);
    assert_eq!(dxf.on("OA-BUILDING-UNKNOWN").len(), 1);
    assert!(dxf.on("OA-BUILDING").is_empty());
}

#[test]
fn computed_layers_land_on_their_own_layers_when_the_diagram_draws_them() {
    let blocks = Look {
        diagram_id: "city-block".to_string(),
        ..look()
    };
    let sun = Look {
        diagram_id: "sun-hours".to_string(),
        ..look()
    };

    assert!(!dxf_of(&scene(), "city-block", &blocks, &[], &[])
        .on("OA-BLOCK")
        .is_empty());
    // Hours of sun are classed, so the bands land on one layer per legend class.
    let sun_dxf = dxf_of(&scene(), "sun-hours", &sun, &[], &[]);
    assert!(sun_dxf
        .entities
        .iter()
        .any(|entity| entity.layer.starts_with("OA-SUN-HOURS-")));
    assert!(dxf_of(&scene(), "context", &look(), &[], &[])
        .on("OA-BLOCK")
        .is_empty());
}

#[test]
fn highlights_get_a_layer_per_line_and_their_captions_in_rd_new() {
    let highlights = [
        plot("Plot", Outline::Solid, -20.0, -20.0, 40.0),
        plot("New house", Outline::Dashed, 100.0, 100.0, 20.0),
        plot("Study", Outline::Dotted, -150.0, 100.0, 30.0),
    ];
    let dxf = dxf_of(&scene(), "context", &look(), &[], &highlights);

    assert_eq!(dxf.layer("OA-HIGHLIGHT-ACCENT").unwrap().2, "CONTINUOUS");
    assert_eq!(dxf.layer("OA-HIGHLIGHT-DASHED-ACCENT").unwrap().2, "DASHED");
    assert_eq!(dxf.layer("OA-HIGHLIGHT-DOTTED-ACCENT").unwrap().2, "DOTTED");
    let plot = dxf.on("OA-HIGHLIGHT-ACCENT");
    assert_eq!(plot.len(), 1);
    assert_eq!(plot[0].points[0], (ORIGIN.0 - 20.0, ORIGIN.1 - 20.0));
    let captions: Vec<&str> = dxf
        .on("OA-HIGHLIGHT-LABEL")
        .iter()
        .map(|entity| entity.text.as_str())
        .collect();
    assert_eq!(
        captions,
        [
            "Plot \\U+00B7 1,600 m\\U+00B2",
            "New house \\U+00B7 400 m\\U+00B2",
            "Study \\U+00B7 900 m\\U+00B2"
        ]
    );

    let hidden = Look {
        hide_highlights: true,
        ..look()
    };
    assert!(dxf_of(&scene(), "context", &hidden, &[], &highlights)
        .on("OA-HIGHLIGHT-ACCENT")
        .is_empty());
}

#[test]
fn a_sheet_opened_from_a_highlight_writes_only_its_area() {
    let house = plot("House", Outline::Solid, -100.0, 70.0, 40.0);
    let framed = Look {
        selection: Some(house.shape.to_area()),
        ..look()
    };
    let dxf = dxf_of(
        &scene(),
        "context",
        &framed,
        &[],
        std::slice::from_ref(&house),
    );

    // The first test building, -100 to -80 east, stands inside; the next only touches its edge.
    assert_eq!(dxf.on("OA-BUILDING").len(), 1);
    for entity in dxf.on("OA-BUILDING") {
        for &(x, y) in &entity.points {
            assert!(x >= ORIGIN.0 - 100.01 && x <= ORIGIN.0 - 59.99, "x {x}");
            assert!(y >= ORIGIN.1 + 69.99 && y <= ORIGIN.1 + 110.01, "y {y}");
        }
    }
}

#[test]
fn the_section_line_is_marked_on_the_plan_with_its_ends() {
    let mut look = look();
    look.section = Some(through_the_row());
    let dxf = dxf_of(&scene(), "context", &look, &[], &[]);

    assert_eq!(dxf.layer("OA-SECTION-LINE").unwrap().2, "DASHDOT");
    let line = dxf.on("OA-SECTION-LINE");
    assert_eq!(
        line[0].points,
        [
            (ORIGIN.0 - 150.0, ORIGIN.1 + 90.0),
            (ORIGIN.0 + 150.0, ORIGIN.1 + 90.0)
        ]
    );
    let labels: Vec<&str> = dxf
        .on("OA-SECTION-MARK")
        .iter()
        .filter(|entity| entity.kind == "TEXT")
        .map(|entity| entity.text.as_str())
        .collect();
    assert_eq!(labels, ["A", "A'"]);
}

#[test]
fn pins_on_the_site_are_points_with_their_labels() {
    let pins = [Pin {
        id: "pin-1".to_string(),
        x: ORIGIN.0 + 10.0,
        y: ORIGIN.1 + 10.0,
        label: "Café".to_string(),
        icon: PinIcon::Dot,
        size: PinSize::Medium,
    }];
    let dxf = dxf_of(&scene(), "context", &look(), &pins, &[]);

    let on_pins = dxf.on("OA-PINS");
    assert_eq!(on_pins[0].kind, "POINT");
    assert_eq!(on_pins[0].points, [(ORIGIN.0 + 10.0, ORIGIN.1 + 10.0)]);
    assert_eq!(on_pins[1].text, "Caf\\U+00E9");
}

#[test]
fn plan_coordinates_stay_near_the_site_in_rd_new() {
    let dxf = dxf_of(&scene(), "context", &look(), &[], &[]);

    for entity in dxf
        .entities
        .iter()
        .filter(|entity| entity.layer != "OA-SITE")
    {
        for &(x, _) in &entity.points {
            assert!(
                (x - ORIGIN.0).abs() <= SITE_RADIUS_M + 1.0,
                "{} x {x}",
                entity.layer
            );
        }
    }
}
