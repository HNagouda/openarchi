mod render_support;

use std::fs;

use openarchi_core::render::{missing_data, render_svg, RenderInput};
use openarchi_core::scene::Scene;
use openarchi_core::sources::pvgis;
use openarchi_core::view::ViewMode;
use openarchi_core::wind::{self, WindSeason, WindYear};
use render_support::{diagram, look, scene, theme};

/// The PVGIS year recorded for the Westermarkt.
const RECORDED_YEAR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/analysis-cache/tiles-v1/pvgis-tmy-5km/24_97.zst"
);

/// The scene as fetched today: with its typical year's wind and the credit for it.
fn scene_with_wind() -> Scene {
    let bytes = zstd::decode_all(fs::read(RECORDED_YEAR).unwrap().as_slice()).unwrap();
    let tmy = pvgis::parse(&bytes).unwrap();
    let mut scene = scene();
    scene.wind = Some(WindYear::from_tmy(&tmy).unwrap());
    scene.credits.push(wind::credit());
    scene
}

fn input<'a>(
    scene: &'a Scene,
    diagram: &'a openarchi_core::diagram::Diagram,
    theme: &'a openarchi_core::theme::Theme,
    look: &'a openarchi_core::project::Look,
) -> RenderInput<'a> {
    RenderInput {
        scene,
        diagram,
        theme,
        look,
        pins: &[],
        highlights: &[],
    }
}

#[test]
fn a_site_fetched_before_the_wind_was_kept_says_its_wind_is_missing() {
    let (scene, diagram, theme, look) = (scene(), diagram("wind"), theme("paper"), look());
    let notes = missing_data(&input(&scene, &diagram, &theme, &look)).unwrap();
    let svg = render_svg(&input(&scene, &diagram, &theme, &look)).unwrap();

    assert!(notes.iter().any(|note| note.contains("wind")), "{notes:?}");
    assert!(!svg.contains(r#"id="wind-class-"#));
}

#[test]
fn the_wind_is_drawn_in_bands_with_flow_lines_its_direction_and_method() {
    let scene = scene_with_wind();
    let (diagram, theme, look) = (diagram("wind"), theme("paper"), look());
    let svg = render_svg(&input(&scene, &diagram, &theme, &look)).unwrap();

    assert!(svg.contains(r#"id="wind-class-"#));
    assert!(svg.contains(r#"id="layer-wind-line""#));
    assert!(svg.contains("Qian"));
    assert!(svg.contains("most common all year"));
    assert!(svg.contains("PVGIS"));
}

#[test]
fn a_season_names_its_own_months_in_the_legend() {
    let scene = scene_with_wind();
    let (diagram, theme, mut look) = (diagram("wind"), theme("paper"), look());
    look.wind_season = WindSeason::Winter;
    let svg = render_svg(&input(&scene, &diagram, &theme, &look)).unwrap();

    assert!(svg.contains("in winter (Dec–Feb)"));
}

#[test]
fn a_tilted_look_draws_the_wind_as_its_plan() {
    let scene = scene_with_wind();
    let (diagram, theme, mut look) = (diagram("wind"), theme("paper"), look());
    let plan = render_svg(&input(&scene, &diagram, &theme, &look)).unwrap();
    look.view.mode = ViewMode::Axonometric;
    let tilted = render_svg(&input(&scene, &diagram, &theme, &look)).unwrap();

    assert_eq!(plan, tilted);
}
