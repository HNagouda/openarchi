mod render_support;

use openarchi_core::highlight::{Highlight, HighlightShape, Outline, UNLABELLED};
use openarchi_core::project::Look;
use openarchi_core::render::{fitting_scale, render_svg, RenderInput};
use openarchi_core::view::ViewMode;
use render_support::{diagram, look, scene, theme, ORIGIN};

/// A `width` by `height` m rectangle whose south-west corner is `x`, `y` metres from the site centre.
fn plot(id: &str, label: &str, x: f64, y: f64, width: f64, height: f64) -> Highlight {
    Highlight {
        id: id.to_string(),
        label: label.to_string(),
        shape: HighlightShape::Rectangle {
            min: [ORIGIN.0 + x, ORIGIN.1 + y],
            max: [ORIGIN.0 + x + width, ORIGIN.1 + y + height],
        },
        outline: Outline::Solid,
        dims_rest: false,
        shows_area: false,
    }
}

fn svg_with(look: &Look, highlights: &[Highlight]) -> String {
    let scene = scene();
    let (diagram, theme) = (diagram("context"), theme("paper"));
    render_svg(&RenderInput {
        scene: &scene,
        diagram: &diagram,
        theme: &theme,
        look,
        pins: &[],
        highlights,
    })
    .expect("render")
}

#[test]
fn a_highlight_is_drawn_with_its_id_label_and_legend_row() {
    let svg = svg_with(
        &look(),
        &[plot("highlight-1", "Plot", -20.0, -20.0, 40.0, 30.0)],
    );

    assert!(svg.contains(r#"inkscape:label="Highlights""#));
    assert!(svg.contains(r#"data-highlight="highlight-1""#));
    // Once beside the shape and once in the legend.
    assert_eq!(svg.matches(">Plot</text>").count(), 2, "{svg}");
}

#[test]
fn an_unlabelled_highlight_shows_in_the_legend_as_a_highlighted_area() {
    let svg = svg_with(&look(), &[plot("highlight-1", "", 0.0, 0.0, 20.0, 20.0)]);

    assert!(svg.contains(&format!(">{UNLABELLED}</text>")));
}

#[test]
fn the_area_joins_the_label_when_asked() {
    let mut house = plot("highlight-1", "New house", 0.0, 0.0, 12.0, 10.0);
    house.shows_area = true;

    assert!(svg_with(&look(), &[house]).contains(">New house · 120 m²</text>"));
}

#[test]
fn a_dashed_outline_and_dim_the_rest_are_drawn_as_asked() {
    let mut proposed = plot("highlight-1", "Proposed", 0.0, 0.0, 30.0, 30.0);
    proposed.outline = Outline::Dashed;
    proposed.dims_rest = true;

    let svg = svg_with(&look(), &[proposed]);

    assert!(svg.contains("stroke-dasharray"));
    assert!(
        svg.contains(r#"fill-rule="evenodd""#),
        "the veil has the highlight cut out"
    );
}

#[test]
fn a_sheet_that_hides_highlights_draws_none_and_lists_none() {
    let hidden = Look {
        hide_highlights: true,
        ..look()
    };

    let svg = svg_with(
        &hidden,
        &[plot("highlight-1", "Plot", 0.0, 0.0, 20.0, 20.0)],
    );

    assert!(!svg.contains("data-highlight="));
    assert!(!svg.contains(">Plot</text>"));
}

#[test]
fn highlights_lie_on_the_ground_in_the_axonometric_and_the_explode_bottom_plate() {
    let highlights = [plot("highlight-1", "Plot", -20.0, -20.0, 40.0, 30.0)];
    for mode in [ViewMode::Axonometric, ViewMode::Explode] {
        let mut tilted = look();
        tilted.view.mode = mode;
        tilted.view.plates = vec!["context".to_string(), "figure-ground".to_string()];

        let svg = svg_with(&tilted, &highlights);

        assert_eq!(svg.matches("data-highlight=").count(), 1, "{mode:?}");
    }

    let mut explode = look();
    explode.view.mode = ViewMode::Explode;
    explode.view.plates = vec!["context".to_string(), "figure-ground".to_string()];
    let svg = svg_with(&explode, &highlights);
    let bottom = svg.find(r#"data-plate="figure-ground""#).unwrap();
    assert!(
        svg.find("data-highlight=").unwrap() > bottom,
        "on the bottom plate"
    );
}

#[test]
fn the_buildings_of_an_axonometric_stand_over_the_highlight() {
    let mut axo = look();
    axo.view.mode = ViewMode::Axonometric;

    let svg = svg_with(
        &axo,
        &[plot("highlight-1", "Plot", -20.0, -20.0, 40.0, 30.0)],
    );

    let shape = svg.find("data-highlight=").unwrap();
    let solids = svg.find(r#"id="solids""#).unwrap();
    let label = svg.find(">Plot</text>").unwrap();
    assert!(
        shape < solids && solids < label,
        "shape under, label over the buildings"
    );
}

#[test]
fn a_section_draws_no_highlight() {
    let mut section = look();
    section.section = Some(openarchi_core::section::SectionLine {
        start: (ORIGIN.0 - 100.0, ORIGIN.1),
        end: (ORIGIN.0 + 100.0, ORIGIN.1),
        depth_m: 60.0,
    });
    section.view.mode = ViewMode::Section;

    let svg = svg_with(
        &section,
        &[plot("highlight-1", "Plot", 0.0, 0.0, 20.0, 20.0)],
    );

    assert!(!svg.contains("data-highlight="));
}

#[test]
fn a_sheet_opened_from_a_highlight_frames_exactly_it() {
    let house = plot("highlight-1", "New house", -10.0, -10.0, 24.0, 16.0);
    let mut framed = Look {
        selection: Some(house.shape.to_area()),
        ..look()
    };
    framed.sheet.scale = fitting_scale(&scene(), &framed).unwrap();

    let svg = svg_with(&framed, std::slice::from_ref(&house));

    // 1:500 is the closest round scale a sheet fits to; a 24 m house would fit closer still.
    assert_eq!(framed.sheet.scale, 500);
    assert!(svg.contains(r#"data-highlight="highlight-1""#));
}

#[test]
fn a_highlight_outside_a_sub_selection_is_left_off_with_its_legend_row() {
    let far = plot("highlight-1", "Far plot", 200.0, 0.0, 20.0, 20.0);
    let near = plot("highlight-2", "Near plot", -10.0, -10.0, 20.0, 20.0);
    let framed = Look {
        selection: Some(near.shape.to_area()),
        ..look()
    };

    let svg = svg_with(&framed, &[far, near]);

    assert!(!svg.contains("Far plot"));
    assert!(svg.contains("Near plot"));
}
