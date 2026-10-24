mod render_support;

use openarchi_core::catalog::builtin_themes;
use openarchi_core::export::{export, ExportFormat};
use openarchi_core::project::Look;
use openarchi_core::render::{render_svg, RenderInput};
use openarchi_core::scene::Scene;
use openarchi_core::sheet::Sheet;
use openarchi_core::view::ViewMode;
use render_support::{diagram, look, scene, scene_with_neighbourhood, theme};

const FURNITURE_GROUP: &str = r#"<g id="furniture""#;

fn svg_of(scene: &Scene, look: &Look, theme_id: &str) -> String {
    let (diagram, theme) = (diagram("context"), theme(theme_id));
    render_svg(&RenderInput {
        scene,
        diagram: &diagram,
        theme: &theme,
        look,
        pins: &[],
    })
    .expect("render")
}

fn look_in(mode: ViewMode) -> Look {
    let mut look = look();
    look.view.mode = mode;
    look
}

/// Everything from the furniture group on: title, legend, scale bar and credits.
fn furniture(svg: &str) -> &str {
    let start = svg.find(FURNITURE_GROUP).expect("a furniture group");
    &svg[start..]
}

#[test]
fn a_new_sheet_shows_the_neighbourhood_and_an_old_one_does_not() {
    assert!(Sheet::default().show_context);

    let mut saved = serde_json::to_value(Sheet::default()).unwrap();
    saved.as_object_mut().unwrap().remove("show_context");
    let old: Sheet = serde_json::from_value(saved).unwrap();
    assert!(!old.show_context);
}

#[test]
fn the_neighbourhood_is_drawn_clipped_to_the_drawing_zone_in_every_view() {
    let scene = scene_with_neighbourhood();
    for mode in [ViewMode::Plan, ViewMode::Axonometric, ViewMode::Explode] {
        let svg = svg_of(&scene, &look_in(mode), "paper");
        assert!(
            svg.contains(r#"context-clip"><rect"#),
            "{mode:?} draws no neighbourhood"
        );
        assert!(svg.contains(r#"inkscape:label="Neighbourhood""#));
    }
}

#[test]
fn the_neighbourhood_leaves_the_furniture_column_exactly_as_it_was() {
    let with = svg_of(&scene_with_neighbourhood(), &look(), "paper");
    let without = svg_of(&scene(), &look(), "paper");
    assert_eq!(furniture(&with), furniture(&without));
}

#[test]
fn a_hidden_neighbourhood_is_not_drawn() {
    let mut look = look();
    look.sheet.show_context = false;
    let svg = svg_of(&scene_with_neighbourhood(), &look, "paper");
    assert!(!svg.contains("context-clip"));
}

#[test]
fn standing_neighbours_are_clipped_one_by_one_among_the_site_buildings() {
    let svg = svg_of(
        &scene_with_neighbourhood(),
        &look_in(ViewMode::Axonometric),
        "paper",
    );
    let solids = &svg[svg.find(r#"id="solids""#).expect("a solids group")..];
    // Three neighbouring buildings, each in its own clipped group.
    assert_eq!(
        solids
            .matches(r#"<g clip-path="url(#context-clip)">"#)
            .count(),
        3
    );
}

#[test]
fn only_the_bottom_plate_of_an_explode_view_stands_in_the_neighbourhood() {
    let svg = svg_of(
        &scene_with_neighbourhood(),
        &look_in(ViewMode::Explode),
        "paper",
    );
    let plates = look().view.plates.len();
    assert_eq!(svg.matches(r#"<clipPath id="plate-"#).count(), plates + 2);
    assert!(svg.contains(&format!("plate-{plates}-context-clip-reach")));
    assert_eq!(svg.matches("context-clip\"><rect").count(), 1);
}

#[test]
fn the_neighbourhood_takes_its_colours_from_every_palette() {
    let scene = scene_with_neighbourhood();
    let mut fills = Vec::new();
    for theme in builtin_themes() {
        let svg = svg_of(&scene, &look(), &theme.id);
        let start = svg
            .find(r#"id="context-building""#)
            .expect("neighbouring buildings");
        let fill = svg[start..]
            .split("fill=\"")
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .unwrap()
            .to_string();
        fills.push(fill);
    }
    fills.sort();
    fills.dedup();
    assert_eq!(fills.len(), builtin_themes().len());
}

#[test]
fn the_dxf_puts_the_neighbourhood_on_its_own_layers_only_when_shown() {
    let scene = scene_with_neighbourhood();
    let (diagram, theme) = (diagram("context"), theme("paper"));
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("site.dxf");
    let mut look = look();
    for is_shown in [true, false] {
        look.sheet.show_context = is_shown;
        let input = RenderInput {
            scene: &scene,
            diagram: &diagram,
            theme: &theme,
            look: &look,
            pins: &[],
        };
        export(&input, ExportFormat::Dxf, 300, &path).unwrap();
        let dxf = std::fs::read_to_string(&path).unwrap();
        assert_eq!(dxf.contains("OA-CONTEXT-BUILDING"), is_shown);
        assert_eq!(dxf.contains("OA-CONTEXT-GREEN"), is_shown);
    }
}
