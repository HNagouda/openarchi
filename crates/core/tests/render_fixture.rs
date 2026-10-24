//! The real Westermarkt scene through every built-in diagram and theme.

use openarchi_core::catalog::{builtin_diagrams, builtin_themes};
use openarchi_core::project::Look;
use openarchi_core::render::{render_svg, RenderInput};
use openarchi_core::scene::Scene;

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/westermarkt.scene.json"
);

fn westermarkt() -> Scene {
    let text = std::fs::read_to_string(FIXTURE).expect("read the fixture");
    serde_json::from_str(&text).expect("parse the fixture")
}

#[test]
fn westermarkt_renders_in_every_diagram_and_theme() {
    let scene = westermarkt();
    let look = Look::default();
    let options = usvg::Options::default();
    for (_, diagram) in builtin_diagrams().iter().flat_map(|d| d.drawings()) {
        for theme in builtin_themes() {
            let svg = render_svg(&RenderInput {
                scene: &scene,
                diagram: &diagram,
                theme: &theme,
                look: &look,
            })
            .unwrap_or_else(|error| panic!("{} in {}: {error}", diagram.id, theme.id));
            usvg::Tree::from_str(&svg, &options)
                .unwrap_or_else(|error| panic!("{} in {}: {error}", diagram.id, theme.id));
        }
    }
}

#[test]
fn westermarkt_building_use_legend_names_the_buildings_bag_cannot_place() {
    let scene = westermarkt();
    let diagram = builtin_diagrams()
        .into_iter()
        .find(|d| d.id == "building-use")
        .expect("building-use");
    let theme = builtin_themes().into_iter().next().expect("paper");
    let svg = render_svg(&RenderInput {
        scene: &scene,
        diagram: &diagram,
        theme: &theme,
        look: &Look::default(),
    })
    .expect("render");

    assert!(svg.contains(">Residential</text>"));
    assert!(svg.contains(">Unknown</text>"));
}
