//! A small recorded scene around the Westerkerk with the environment layers:
//! noise, air, flood depth, slope, population and BAG dwellings. Checks that
//! the bands, legends, credits, ratios and summary agree with each other.

use openarchi_core::catalog::{builtin_diagrams, builtin_themes};
use openarchi_core::environment::summarise;
use openarchi_core::project::Look;
use openarchi_core::ratios;
use openarchi_core::render::{render_svg, RenderInput};
use openarchi_core::scene::{Layer, Scene};
use openarchi_core::sources::bands::visible_areas;

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/westerkerk-environment.scene.json"
);
/// The diagrams that draw the environment layers.
const ENVIRONMENT_DIAGRAMS: [&str; 9] = [
    "traffic-noise",
    "noise-all-sources",
    "air-quality",
    "air-quality-pm10",
    "air-quality-pm25",
    "flood",
    "slope",
    "population",
    "households",
];

fn westerkerk() -> Scene {
    let text = std::fs::read_to_string(FIXTURE).expect("read the fixture");
    serde_json::from_str(&text).expect("parse the fixture")
}

fn svg_of(scene: &Scene, diagram_id: &str, theme_id: &str) -> String {
    let diagrams = builtin_diagrams();
    let themes = builtin_themes();
    let diagram = diagrams.iter().find(|d| d.id == diagram_id).unwrap();
    let theme = themes.iter().find(|t| t.id == theme_id).unwrap();
    render_svg(&RenderInput {
        scene,
        diagram,
        theme,
        look: &Look::default(),
    })
    .unwrap_or_else(|error| panic!("{diagram_id} in {theme_id}: {error}"))
}

#[test]
fn every_environment_diagram_renders_in_every_theme() {
    let scene = westerkerk();
    let options = usvg::Options::default();
    for diagram_id in ENVIRONMENT_DIAGRAMS {
        for theme in builtin_themes() {
            let svg = svg_of(&scene, diagram_id, &theme.id);
            usvg::Tree::from_str(&svg, &options)
                .unwrap_or_else(|error| panic!("{diagram_id} in {}: {error}", theme.id));
        }
    }
}

#[test]
fn the_noise_legend_gives_unit_and_year_and_the_sheet_credits_rivm() {
    let svg = svg_of(&westerkerk(), "traffic-noise", "paper");
    assert!(svg.contains("RIVM, 2022"));
    assert!(svg.contains("55–60 dB"));
    assert!(svg.contains("RIVM Atlas Leefomgeving"));
    assert!(
        !svg.contains("LIWO"),
        "a noise sheet does not credit the flood map"
    );
}

#[test]
fn legends_and_ratios_quote_the_year_of_the_edition_the_scene_was_fetched_in() {
    let mut scene = westerkerk();
    scene.editions.insert(Layer::RoadNoise, 2023);
    scene.editions.insert(Layer::Population, 2025);

    assert!(svg_of(&scene, "traffic-noise", "paper").contains("RIVM, 2023."));
    assert!(svg_of(&scene, "population", "paper").contains("CBS, 2025."));
    for diagram_id in ENVIRONMENT_DIAGRAMS {
        assert!(!svg_of(&scene, diagram_id, "paper").contains("{year}"));
    }
    let theme = builtin_themes().into_iter().next().unwrap();
    let ratios = ratios::compute(&scene, &theme).unwrap();
    let noise = ratios
        .groups
        .iter()
        .find(|group| group.id == "road_noise")
        .unwrap();
    assert!(noise.note.as_deref().unwrap().ends_with("RIVM, 2023."));
}

#[test]
fn a_sheet_without_environment_layers_leaves_their_sources_out_of_the_credits() {
    let svg = svg_of(&westerkerk(), "figure-ground", "paper");
    assert!(!svg.contains("RIVM"));
    assert!(!svg.contains("CBS"));
    assert!(svg.contains("BAG"));
}

#[test]
fn noise_bands_stack_from_quiet_to_loud_and_cover_the_site_once() {
    let scene = westerkerk();
    let bands: Vec<_> = scene
        .features
        .iter()
        .filter(|feature| feature.layer == Layer::RoadNoise)
        .collect();
    let site = geo::Area::unsigned_area(&scene.boundary);
    let shown: f64 = visible_areas(&bands).iter().map(|(_, area)| area).sum();

    assert!(bands.len() >= 4, "{} noise bands", bands.len());
    assert!(
        (shown - site).abs() / site < 0.02,
        "bands show {shown} of {site} m²"
    );
}

#[test]
fn the_summary_matches_what_the_services_said_at_the_westerkerk() {
    let summary = summarise(&westerkerk());

    // RIVM put 40 x 40 cells here between 39 and 74 dB, mean 56.6.
    let noise = summary.road_noise_mean_db.unwrap();
    assert!((50.0..70.0).contains(&noise), "{noise} dB");
    // LIWO answered 0.1807 m at the Westerkerk itself in the extreme scenario.
    let depth = summary.flood_max_depth_m.unwrap();
    assert!((0.1..1.0).contains(&depth), "{depth} m");
    assert!(summary.no2_mean_ug_m3.unwrap() > 10.0);
    assert!(summary.residents.unwrap() > 100.0);
    assert!(summary.dwellings.unwrap() > 50);
    // Fetched before scenes kept their editions, so it gets the years pinned then.
    assert_eq!(summary.noise_year, Some(2022));
    assert_eq!(summary.population_year, Some(2024));
}

#[test]
fn ratios_add_dwellings_bag_floor_area_and_noise_shares() {
    let theme = builtin_themes().into_iter().next().unwrap();
    let ratios = ratios::compute(&westerkerk(), &theme).unwrap();

    assert!(ratios.dwellings.unwrap() > 50);
    assert!(ratios.residents.unwrap() > 100.0);
    let bag_ratio = ratios.bag_floor_area_ratio.unwrap();
    assert!(
        (0.5..6.0).contains(&bag_ratio),
        "BAG floor area ratio {bag_ratio}"
    );

    let noise = ratios
        .groups
        .iter()
        .find(|group| group.id == "road_noise")
        .expect("a road noise group");
    assert!((noise.total - ratios.site_area_m2).abs() / ratios.site_area_m2 < 0.02);
    assert!(ratios.groups.iter().any(|group| group.id == "dwellings"));
    assert!(ratios.groups.iter().any(|group| group.id == "floor_area"));
}

#[test]
fn every_band_edge_is_a_break_of_its_diagram_so_legend_and_map_agree() {
    let scene = westerkerk();
    for diagram in builtin_diagrams() {
        let Some(classify) = diagram.classify else {
            continue;
        };
        let openarchi_core::diagram::Scheme::Sequential { breaks, .. } = &classify.scheme else {
            continue;
        };
        let edges = scene
            .features
            .iter()
            .filter(|feature| feature.layer == classify.layer && feature.layer != Layer::Population)
            .filter_map(|feature| feature.attributes.value);
        for edge in edges {
            let is_lowest = edge < breaks[0];
            assert!(
                is_lowest || breaks.contains(&edge),
                "{} band at {edge} is not a break",
                diagram.id
            );
        }
    }
}
