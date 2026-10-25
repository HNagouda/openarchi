mod render_support;

use std::fs;

use openarchi_core::radiation::{self, SolarYear};
use openarchi_core::render::{missing_data, render_svg, RenderInput};
use openarchi_core::scene::Scene;
use openarchi_core::sources::pvgis;
use render_support::{diagram, look, scene, theme};

/// The PVGIS year recorded for the Westermarkt.
const RECORDED_YEAR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/analysis-cache/tiles-v1/pvgis-tmy-5km/24_97.zst"
);

/// The scene as fetched today: with its typical year and the credit for it.
fn scene_with_solar_year() -> Scene {
    let bytes = zstd::decode_all(fs::read(RECORDED_YEAR).unwrap().as_slice()).unwrap();
    let tmy = pvgis::parse(&bytes).unwrap();
    let mut scene = scene();
    scene.solar = Some(SolarYear::from_tmy(&tmy).unwrap());
    scene.credits.push(radiation::credit());
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
fn the_sky_view_is_drawn_in_bands_with_its_method_in_the_legend() {
    let (scene, diagram, theme, look) = (scene(), diagram("sky-view"), theme("paper"), look());
    let svg = render_svg(&input(&scene, &diagram, &theme, &look)).unwrap();

    assert!(svg.contains(r#"id="layer-sky-view""#));
    assert!(svg.contains(r#"id="sky-view-class-"#));
    assert!(svg.contains("Johnson"));
}

#[test]
fn a_site_fetched_before_the_typical_year_was_kept_says_its_radiation_is_missing() {
    let (scene, diagram, theme, look) =
        (scene(), diagram("solar-radiation"), theme("paper"), look());
    let notes = missing_data(&input(&scene, &diagram, &theme, &look)).unwrap();
    let svg = render_svg(&input(&scene, &diagram, &theme, &look)).unwrap();

    assert!(
        notes.iter().any(|note| note.contains("solar radiation")),
        "{notes:?}"
    );
    assert!(!svg.contains(r#"id="solar-radiation-class-"#));
}

#[test]
fn the_radiation_is_drawn_from_the_kept_year_and_credits_pvgis() {
    let scene = scene_with_solar_year();
    let (diagram, theme, look) = (diagram("solar-radiation"), theme("paper"), look());
    let notes = missing_data(&input(&scene, &diagram, &theme, &look)).unwrap();
    let svg = render_svg(&input(&scene, &diagram, &theme, &look)).unwrap();
    let sky = render_svg(&input(&scene, &self::diagram("sky-view"), &theme, &look)).unwrap();

    assert!(
        !notes.iter().any(|note| note.contains("solar radiation")),
        "{notes:?}"
    );
    assert!(svg.contains(r#"id="solar-radiation-class-"#));
    assert!(svg.contains("PVGIS"));
    // The credit belongs to the radiation, not to every sheet.
    assert!(!sky.contains("PVGIS"));
}

#[test]
fn the_radiation_legend_gives_this_sites_kwh_as_shares_of_open_ground() {
    let scene = scene_with_solar_year();
    let (diagram, theme, look) = (diagram("solar-radiation"), theme("paper"), look());
    let svg = render_svg(&input(&scene, &diagram, &theme, &look)).unwrap();

    assert!(svg.contains("open ground receives here"));
    assert!(svg.contains("(95 % or more)"));
    assert!(svg.contains("FAO forest"));
}
