mod render_support;

use openarchi_core::catalog::{builtin_diagrams, builtin_themes};
use openarchi_core::render::{render_svg, RenderInput};
use openarchi_core::scene::Layer;
use openarchi_core::sheet::{Frame, Paper};
use openarchi_core::theme::LayerOverride;
use render_support::{diagram, look, scene, theme};

fn svg_for(diagram_id: &str, theme_id: &str, look: &openarchi_core::project::Look) -> String {
    let scene = scene();
    let (diagram, theme) = (diagram(diagram_id), theme(theme_id));
    render_svg(&RenderInput {
        scene: &scene,
        diagram: &diagram,
        theme: &theme,
        look,
    })
    .expect("render")
}

/// Parsed with the bundled fonts, so text has real extents in the bounding boxes.
fn parse(svg: &str) -> usvg::Tree {
    let mut options = usvg::Options::default();
    options
        .fontdb_mut()
        .load_fonts_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/fonts"));
    usvg::Tree::from_str(svg, &options).expect("valid SVG")
}

#[test]
fn every_diagram_renders_in_every_theme_as_valid_svg() {
    let scene = scene();
    let look = look();
    for diagram in builtin_diagrams() {
        for theme in builtin_themes() {
            let svg = render_svg(&RenderInput {
                scene: &scene,
                diagram: &diagram,
                theme: &theme,
                look: &look,
            })
            .unwrap_or_else(|error| panic!("{} in {}: {error}", diagram.id, theme.id));
            parse(&svg);
        }
    }
}

#[test]
fn sheet_is_sized_in_millimetres_with_a_millimetre_viewbox() {
    let svg = svg_for("context", "paper", &look());

    assert!(svg.contains(r#"width="420mm" height="297mm" viewBox="0 0 420 297""#));
}

#[test]
fn portrait_sheet_swaps_width_and_height() {
    let mut look = look();
    look.sheet.landscape = false;

    let svg = svg_for("context", "paper", &look);

    assert!(svg.contains(r#"width="297mm" height="420mm""#));
}

#[test]
fn layers_are_named_groups_drawing_programs_list() {
    let svg = svg_for("context", "paper", &look());

    assert!(svg
        .contains(r#"id="layer-building" inkscape:groupmode="layer" inkscape:label="Buildings""#));
    assert!(
        svg.contains(r#"id="layer-rail-track" inkscape:groupmode="layer" inkscape:label="Tracks""#)
    );
    assert!(svg.contains(r#"inkscape:label="Furniture""#));
}

#[test]
fn coordinates_are_rounded_to_a_hundredth_of_a_millimetre() {
    let svg = svg_for("context", "paper", &look());

    for number in svg
        .split(|c: char| !(c.is_ascii_digit() || c == '.'))
        .filter(|part| part.contains('.'))
    {
        let decimals = number.split('.').nth(1).unwrap_or("");
        assert!(
            decimals.len() <= 2 || number.matches('.').count() > 1,
            "{number} has more than two decimals"
        );
    }
}

#[test]
fn one_ground_metre_is_a_thousandth_of_the_scale_in_millimetres() {
    // At 1:2500 a 20 m building is 8 mm wide on paper.
    let svg = svg_for("figure-ground", "paper", &look());

    assert!(
        svg.contains("l8 0 0 -8 -8 0z"),
        "a 20 m square should be drawn 8 mm wide"
    );
}

#[test]
fn canal_island_survives_as_a_hole() {
    let svg = svg_for("context", "paper", &look());
    let water = svg
        .split(r#"id="layer-water""#)
        .nth(1)
        .expect("water layer");
    let first_path = water.split("<path d=\"").nth(1).expect("water path");
    let path_data = first_path.split('"').next().unwrap_or_default();

    assert_eq!(path_data.matches('M').count(), 2, "outer ring and island");
    assert!(water.starts_with(' ') && svg.contains(r#"fill-rule="evenodd""#));
}

#[test]
fn classified_legend_lists_present_classes_then_unknown() {
    let svg = svg_for("building-age", "paper", &look());

    for label in [
        "Before 1900",
        "1900–1945",
        "1970–1990",
        "2010 and later",
        "Unknown",
    ] {
        assert!(
            svg.contains(&format!(">{label}</text>")),
            "legend misses {label}"
        );
    }
    assert!(
        !svg.contains(">1945–1970</text>"),
        "a class with no buildings is not listed"
    );
    assert!(svg.find(">2010 and later<").unwrap() < svg.find(">Unknown<").unwrap());
}

#[test]
fn unknown_buildings_take_the_unknown_colour() {
    let paper = theme("paper");
    let svg = svg_for("building-use", "paper", &look());
    let unknown_group = format!(
        r#"id="building-unknown" inkscape:label="Unknown" fill="{}""#,
        paper.palette.unknown
    );

    assert!(svg.contains(&unknown_group));
}

#[test]
fn mixed_use_takes_the_first_value_that_matches() {
    // "winkelfunctie, woonfunctie" is retail; an unlisted value falls in Other.
    let svg = svg_for("building-use", "paper", &look());

    assert!(svg.contains(">Retail</text>"));
    assert!(svg.contains(">Other</text>"));
}

#[test]
fn muted_layers_share_one_legend_row() {
    let svg = svg_for("landscape", "paper", &look());

    assert!(svg.contains(">Buildings</text>") || svg.contains(">Context</text>"));
    assert!(!svg.contains(">Roads</text>"));
}

#[test]
fn hidden_override_leaves_the_layer_out() {
    let mut look = look();
    look.overrides.insert(
        Layer::Water,
        LayerOverride {
            hidden: Some(true),
            ..LayerOverride::default()
        },
    );

    let svg = svg_for("context", "paper", &look);

    assert!(!svg.contains(r#"id="layer-water""#));
}

#[test]
fn fill_override_wins_over_the_classes() {
    let mut look = look();
    look.overrides.insert(
        Layer::Building,
        LayerOverride {
            fill: Some("#123456".to_string()),
            ..LayerOverride::default()
        },
    );

    let svg = svg_for("building-age", "paper", &look);

    assert!(svg.contains(r##"fill="#123456""##));
    assert!(!svg.contains(">Unknown</text>"));
}

#[test]
fn bad_override_colour_is_an_error_not_a_broken_svg() {
    let scene = scene();
    let (diagram, theme) = (diagram("context"), theme("paper"));
    let mut look = look();
    look.overrides.insert(
        Layer::Road,
        LayerOverride {
            fill: Some("@nope".to_string()),
            ..LayerOverride::default()
        },
    );

    let result = render_svg(&RenderInput {
        scene: &scene,
        diagram: &diagram,
        theme: &theme,
        look: &look,
    });

    assert!(result.is_err());
}

#[test]
fn site_boundary_is_drawn_in_the_accent_colour() {
    let paper = theme("paper");
    let svg = svg_for("context", "paper", &look());

    assert!(svg.contains(r#"id="site-boundary""#));
    assert!(svg.contains(&format!(
        r#"stroke="{}" stroke-width="0.5""#,
        paper.palette.accent
    )));
}

#[test]
fn shadows_follow_the_theme() {
    let with_shadow = svg_for("context", "paper", &look());
    let without_shadow = svg_for("context", "ink", &look());

    assert!(with_shadow.contains(r#"id="building-shadow""#));
    assert!(!without_shadow.contains(r#"id="building-shadow""#));
}

#[test]
fn patterned_layers_get_a_pattern_definition() {
    let svg = svg_for("context", "mono", &look());

    assert!(svg.contains(r#"<pattern id="pattern-water""#));
    assert!(svg.contains("fill=\"url(#pattern-water)\""));
}

#[test]
fn furniture_never_overlaps_the_drawing() {
    let papers = [
        Paper::A4,
        Paper::A3,
        Paper::A2,
        Paper::A1,
        Paper::A0,
        Paper::Square,
    ];
    for paper in papers {
        for landscape in [true, false] {
            for frame in [Frame::Circle, Frame::Square, Frame::Area] {
                let mut look = look();
                look.sheet.paper = paper;
                look.sheet.landscape = landscape;
                look.sheet.frame = frame;
                // A scale that overflows small sheets, so cropping is exercised too.
                look.sheet.scale = 1500;
                look.sheet.title =
                    "A long title that has to wrap over more than one line".to_string();

                let tree = parse(&svg_for("building-use", "paper", &look));
                let ground = tree
                    .node_by_id("ground")
                    .expect("ground")
                    .abs_layer_bounding_box()
                    .expect("ground box");
                let furniture = tree
                    .node_by_id("furniture")
                    .expect("furniture")
                    .abs_layer_bounding_box()
                    .expect("furniture box");
                let overlaps = ground.left() < furniture.right()
                    && furniture.left() < ground.right()
                    && ground.top() < furniture.bottom()
                    && furniture.top() < ground.bottom();
                assert!(
                    !overlaps,
                    "{paper:?} landscape={landscape} {frame:?}: {ground:?} overlaps {furniture:?}"
                );
                let page = tree.size();
                assert!(
                    furniture.right() <= page.width() && furniture.bottom() <= page.height(),
                    "{paper:?} furniture leaves the page"
                );
            }
        }
    }
}

#[test]
fn zero_scale_is_refused() {
    let scene = scene();
    let (diagram, theme) = (diagram("context"), theme("paper"));
    let mut look = look();
    look.sheet.scale = 0;

    assert!(render_svg(&RenderInput {
        scene: &scene,
        diagram: &diagram,
        theme: &theme,
        look: &look
    })
    .is_err());
}
