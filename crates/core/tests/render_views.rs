mod render_support;

use openarchi_core::catalog::{builtin_diagrams, builtin_themes};
use openarchi_core::project::Look;
use openarchi_core::render::{render_svg, RenderInput};
use openarchi_core::view::{SunMoment, ViewMode};
use render_support::{diagram, look, scene, theme};

fn look_in(mode: ViewMode) -> Look {
    let mut look = look();
    look.view.mode = mode;
    look
}

fn svg_for(diagram_id: &str, theme_id: &str, look: &Look) -> String {
    try_svg(diagram_id, theme_id, look).expect("render")
}

fn try_svg(diagram_id: &str, theme_id: &str, look: &Look) -> openarchi_core::Result<String> {
    let scene = scene();
    let (diagram, theme) = (diagram(diagram_id), theme(theme_id));
    render_svg(&RenderInput {
        scene: &scene,
        diagram: &diagram,
        theme: &theme,
        look,
        pins: &[],
        highlights: &[],
        model: None,
    })
}

fn parse(svg: &str) -> usvg::Tree {
    usvg::Tree::from_str(svg, &usvg::Options::default()).expect("valid SVG")
}

#[test]
fn every_diagram_renders_in_every_theme_in_the_tilted_views() {
    let scene = scene();
    for mode in [ViewMode::Axonometric, ViewMode::Explode] {
        let look = look_in(mode);
        for (_, diagram) in builtin_diagrams().iter().flat_map(|d| d.drawings()) {
            for theme in builtin_themes() {
                let svg = render_svg(&RenderInput {
                    scene: &scene,
                    diagram: &diagram,
                    theme: &theme,
                    look: &look,
                    pins: &[],
                    highlights: &[],
                    model: None,
                })
                .unwrap_or_else(|error| panic!("{} in {} {mode:?}: {error}", diagram.id, theme.id));
                parse(&svg);
            }
        }
    }
}

#[test]
fn axonometric_stands_buildings_and_trees_up_in_one_layer() {
    let svg = svg_for("context", "paper", &look_in(ViewMode::Axonometric));

    assert!(svg.contains(
        r#"id="solids" inkscape:groupmode="layer" inkscape:label="Buildings and trees""#
    ));
    // The flat building layer is replaced by the solids, not drawn as well.
    assert!(!svg.contains(r#"id="layer-building""#));
    // A circle frame turns into an ellipse when tilted.
    assert!(svg.contains("<ellipse"));
}

#[test]
fn axonometric_turns_the_north_arrow_with_the_view() {
    let mut look = look_in(ViewMode::Axonometric);
    look.view.rotation_deg = 30.0;

    let svg = svg_for("context", "paper", &look);

    assert!(svg.contains(r#"id="north-arrow""#));
    assert!(svg.contains("transform=\"rotate(-"));
}

#[test]
fn a_plan_keeps_its_upright_north_arrow() {
    let svg = svg_for("context", "paper", &look());
    assert!(!svg.contains("rotate("));
}

#[test]
fn explode_draws_one_labelled_plate_per_chosen_diagram() {
    let mut look = look_in(ViewMode::Explode);
    look.view.plates = vec!["building-use".to_string(), "context".to_string()];

    let svg = svg_for("context", "paper", &look);

    assert!(svg.contains(r#"inkscape:label="Plate 1: Building use""#));
    assert!(svg.contains(r#"inkscape:label="Plate 2: Context""#));
    assert!(!svg.contains("Plate 3"));
    assert!(svg.contains(">Building use</text>"));
    // Each plate clips to its own frame.
    assert!(svg.contains(r#"<clipPath id="plate-1-frame-clip">"#));
    assert!(svg.contains(r#"<clipPath id="plate-2-frame-clip">"#));
}

#[test]
fn explode_names_each_plate_and_its_place_for_the_app() {
    let mut look = look_in(ViewMode::Explode);
    look.view.plates = vec!["building-use".to_string(), "context".to_string()];

    let svg = svg_for("context", "paper", &look);

    assert!(svg.contains(r#"data-plate="building-use" data-index="0" data-centre-y=""#));
    assert!(svg.contains(r#"data-plate="context" data-index="1" data-centre-y=""#));
}

#[test]
fn explode_draws_a_scale_bar_that_holds_left_to_right() {
    let mut look = look_in(ViewMode::Explode);
    look.view.plates = vec!["building-use".to_string(), "context".to_string()];

    let svg = svg_for("context", "paper", &look);

    assert!(svg.contains(r#"id="scale-bar""#));
    assert!(svg.contains(" left to right, on A3</text>"));
}

#[test]
fn axonometric_says_its_scale_holds_left_to_right() {
    let svg = svg_for("context", "paper", &look_in(ViewMode::Axonometric));

    assert!(svg.contains(r#"id="scale-bar""#));
    assert!(svg.contains(" left to right, on A3</text>"));
}

#[test]
fn a_plan_scale_note_has_no_direction() {
    let svg = svg_for("context", "paper", &look());
    assert!(!svg.contains("left to right"));
}

#[test]
fn explode_refuses_a_plate_that_is_not_a_diagram() {
    let mut look = look_in(ViewMode::Explode);
    look.view.plates = vec!["no-such-diagram".to_string()];

    let error = try_svg("context", "paper", &look).unwrap_err();

    assert!(error.to_string().contains("no-such-diagram"));
}

#[test]
fn sun_path_draws_the_three_arcs_with_legend_rows() {
    let svg = svg_for("sun-path", "paper", &look());

    assert!(svg.contains(r#"id="sun-path""#));
    assert!(svg.contains("Summer solstice, 21 June"));
    assert!(svg.contains("Winter solstice, 21 December"));
    assert!(svg.contains("Sun on 21 March, 13:00"));
    // The horizon is the frame here, so the accent site line gives way to the arcs.
    assert!(!svg.contains("Site boundary"));
}

#[test]
fn shadow_diagram_names_its_moment_and_draws_shadows() {
    let svg = svg_for("shadow", "paper", &look());

    assert!(svg.contains(r#"id="shadows""#));
    assert!(svg.contains("Shadow on 21 March, 13:00"));
}

#[test]
fn shadow_at_night_says_there_is_no_sun() {
    let mut look = look();
    look.sun = SunMoment {
        month: 12,
        day: 21,
        hour: 22,
        minute: 0,
    };

    let svg = svg_for("shadow", "paper", &look);

    assert!(svg.contains("No sun on 21 December, 22:00"));
}

#[test]
fn an_impossible_date_is_refused() {
    let mut look = look();
    look.sun.month = 13;

    assert!(try_svg("shadow", "paper", &look).is_err());
}
