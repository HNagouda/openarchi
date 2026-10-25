mod render_support;

use openarchi_core::project::Look;
use openarchi_core::render::{missing_data, render_svg, RenderInput};
use openarchi_core::scene::{Layer, Scene};
use openarchi_core::sources::raster::Stats;
use render_support::{diagram, look, scene, scene_with_neighbourhood, theme};

fn notes(scene: &Scene, diagram_id: &str, look: &Look) -> Vec<String> {
    let (diagram, theme) = (diagram(diagram_id), theme("paper"));
    missing_data(&RenderInput {
        scene,
        diagram: &diagram,
        theme: &theme,
        look,
        pins: &[],
        highlights: &[],
    })
    .expect("notes")
}

fn quiet_look() -> Look {
    let mut look = look();
    look.sheet.show_context = false;
    look
}

#[test]
fn a_site_fetched_without_openstreetmap_says_its_transit_is_missing() {
    let found = notes(&scene(), "transit", &quiet_look());
    assert_eq!(found.len(), 1);
    assert!(
        found[0].contains("transit stops") && found[0].contains("Fetch the site again"),
        "{found:?}"
    );
}

#[test]
fn a_site_fetched_before_the_rasters_says_its_noise_is_missing() {
    assert_eq!(notes(&scene(), "noise", &quiet_look()).len(), 1);
}

#[test]
fn a_site_without_flood_risk_is_not_taken_for_a_missing_source() {
    let mut measured = scene();
    measured.measured.set(
        Layer::RoadNoise,
        Some(Stats {
            min: 50.0,
            mean: 55.0,
            max: 60.0,
            coverage: 1.0,
        }),
    );
    assert!(notes(&measured, "flood", &quiet_look()).is_empty());
}

#[test]
fn the_neighbourhood_toggle_on_a_site_without_one_says_so_and_the_svg_carries_it() {
    let found = notes(&scene(), "context", &look());
    assert_eq!(found.len(), 1);
    assert!(notes(&scene_with_neighbourhood(), "context", &look()).is_empty());

    let (diagram, theme) = (diagram("context"), theme("paper"));
    let svg = render_svg(&RenderInput {
        scene: &scene(),
        diagram: &diagram,
        theme: &theme,
        look: &look(),
        pins: &[],
        highlights: &[],
    })
    .unwrap();
    assert!(svg
        .contains("data-missing=\"This site was fetched before openarchi kept the neighbourhood"));
}

#[test]
fn the_note_follows_the_variant_of_the_look() {
    let mut read = scene();
    let mut credit = read.credits[0].clone();
    credit.layers = vec![Layer::TreeCanopy, Layer::GreenCover];
    read.credits.push(credit);
    // The diagram as the catalog holds it, with its variants, as the MCP server passes it.
    let greenery = openarchi_core::catalog::builtin_diagrams()
        .into_iter()
        .find(|diagram| diagram.id == "greenery")
        .unwrap();
    let mut look = quiet_look();
    look.variant = Some("vegetation".to_string());

    let found = missing_data(&RenderInput {
        scene: &read,
        diagram: &greenery,
        theme: &theme("paper"),
        look: &look,
        pins: &[],
        highlights: &[],
    })
    .unwrap();
    assert_eq!(found.len(), 1);
    assert!(found[0].contains("vegetation index"), "{found:?}");
}

#[test]
fn a_site_fetched_before_trees_and_heat_says_they_are_missing() {
    let found = notes(&scene(), "greenery:canopy", &quiet_look());
    assert_eq!(found.len(), 1);
    assert!(found[0].contains("tree canopy"), "{found:?}");
    assert_eq!(notes(&scene(), "heat-island", &quiet_look()).len(), 1);
}

#[test]
fn a_site_that_read_the_tree_map_and_has_no_trees_is_not_missing_them() {
    let mut read = scene();
    let mut credit = read.credits[0].clone();
    credit.layers = vec![Layer::TreeCanopy, Layer::GreenCover];
    read.credits.push(credit);

    let canopy = notes(&read, "greenery:canopy", &quiet_look());
    assert!(canopy.is_empty(), "{canopy:?}");
    assert!(notes(&read, "greenery:cover", &quiet_look()).is_empty());
    assert_eq!(notes(&read, "greenery:vegetation", &quiet_look()).len(), 1);
}
