mod render_support;

use openarchi_core::project::Look;
use openarchi_core::render::{render_svg, RenderInput};
use openarchi_core::view::ViewMode;
use render_support::{diagram, look, scene, theme, ORIGIN};

fn svg_with(look: &Look) -> String {
    let scene = scene();
    let (diagram, theme) = (diagram(&look.diagram_id), theme("paper"));
    render_svg(&RenderInput {
        scene: &scene,
        diagram: &diagram,
        theme: &theme,
        look,
        pins: &[],
        highlights: &[],
    })
    .expect("render")
}

fn view_look() -> Look {
    Look {
        diagram_id: "view".to_string(),
        ..look()
    }
}

/// The paper position the core wrote on the eye the app drags.
fn eye_on_paper(svg: &str) -> (f64, f64) {
    let start = svg
        .find(r#"id="view-point""#)
        .expect("an eye with a handle");
    let group = &svg[start..];
    let number = |name: &str| {
        let key = format!(r#"{name}=""#);
        let from = group.find(&key).unwrap() + key.len();
        let to = from + group[from..].find('"').unwrap();
        group[from..to].parse::<f64>().unwrap()
    };
    (number("data-x"), number("data-y"))
}

#[test]
fn the_view_tints_the_ground_in_view_and_says_how() {
    let svg = svg_with(&view_look());

    assert!(svg.contains(r#"id="layer-viewshed""#));
    assert!(svg.contains("Franklin &amp; Ray 1994"));
    assert!(svg.contains("Viewpoint, eye 1.5 m above the ground"));
}

#[test]
fn moving_the_viewpoint_moves_the_eye() {
    let centre = eye_on_paper(&svg_with(&view_look()));
    let moved = Look {
        view_point: Some((ORIGIN.0 + 100.0, ORIGIN.1 + 100.0)),
        ..view_look()
    };
    let placed = eye_on_paper(&svg_with(&moved));

    // North-east on the ground is right and up on a plan.
    assert!(placed.0 > centre.0 + 10.0, "{centre:?} {placed:?}");
    assert!(placed.1 < centre.1 - 10.0, "{centre:?} {placed:?}");
}

#[test]
fn other_diagrams_draw_no_eye() {
    let svg = svg_with(&Look {
        view_point: Some(ORIGIN),
        ..look()
    });

    assert!(!svg.contains("view-point"));
}

#[test]
fn a_tilted_view_draws_the_view_too() {
    let mut tilted = view_look();
    tilted.view.mode = ViewMode::Axonometric;
    let svg = svg_with(&tilted);

    assert!(svg.contains(r#"id="layer-viewshed""#));
    assert!(svg.contains(r#"id="view-point""#));
}

#[test]
fn a_look_saved_before_viewpoints_opens_at_the_site_centre() {
    let saved = serde_json::to_value(look()).unwrap();
    assert!(saved.get("view_point").is_none());
    let read: Look = serde_json::from_value(saved).unwrap();

    assert_eq!(read.view_point, None);
}
