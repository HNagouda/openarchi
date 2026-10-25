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
    })
    .unwrap();
    assert!(svg
        .contains("data-missing=\"This site was fetched before openarchi kept the neighbourhood"));
}
