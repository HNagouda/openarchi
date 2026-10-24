//! Ratios of the recorded Westermarkt scene: every group adds up, and the
//! numbers agree with what the scene and the diagrams say.

use openarchi_core::catalog;
use openarchi_core::ratios::{self, ChartStyle, Group, Ratios, Unit};
use openarchi_core::scene::{Layer, Scene};
use openarchi_core::theme::Theme;

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/westermarkt.scene.json"
);
const SHARE_TOLERANCE: f64 = 1e-9;

fn westermarkt() -> Scene {
    let json = std::fs::read_to_string(FIXTURE).expect("fixture is checked in");
    serde_json::from_str(&json).expect("fixture is a serialised Scene")
}

fn theme(id: &str) -> Theme {
    catalog::builtin_themes()
        .into_iter()
        .find(|theme| theme.id == id)
        .unwrap()
}

fn ratios_in(theme_id: &str) -> Ratios {
    ratios::compute(&westermarkt(), &theme(theme_id)).unwrap()
}

fn group<'a>(ratios: &'a Ratios, id: &str) -> &'a Group {
    ratios
        .groups
        .iter()
        .find(|group| group.id == id)
        .unwrap_or_else(|| panic!("no {id} group"))
}

fn value_of(group: &Group, label: &str) -> f64 {
    group
        .rows
        .iter()
        .find(|row| row.label == label)
        .map(|row| row.value)
        .unwrap_or(0.0)
}

#[test]
fn every_group_has_shares_that_add_up_to_one_hundred_percent() {
    for group in ratios_in("paper").groups {
        let shares: f64 = group.rows.iter().map(|row| row.share).sum();
        let values: f64 = group.rows.iter().map(|row| row.value).sum();
        assert!(
            (shares - 1.0).abs() < SHARE_TOLERANCE,
            "{} adds up to {shares}",
            group.id
        );
        assert!((values - group.total).abs() < 1e-6, "{} total", group.id);
        assert!(
            group.rows.iter().all(|row| row.value > 0.0),
            "{} has an empty row",
            group.id
        );
    }
}

#[test]
fn westermarkt_has_every_group_but_households() {
    let ids: Vec<String> = ratios_in("paper")
        .groups
        .into_iter()
        .map(|group| group.id)
        .collect();
    assert_eq!(
        ids,
        [
            "land_use",
            "transport",
            "building_use",
            "floors",
            "height",
            "year_built",
            "footprint",
            "coverage",
            "floor_area_ratio",
            "green",
            "water",
        ]
    );
}

#[test]
fn building_groups_count_every_building_once() {
    let ratios = ratios_in("paper");
    let buildings = westermarkt()
        .features
        .iter()
        .filter(|feature| feature.layer == Layer::Building)
        .count();
    assert_eq!(ratios.building_count, buildings);
    for id in [
        "building_use",
        "floors",
        "height",
        "year_built",
        "footprint",
    ] {
        let group = group(&ratios, id);
        assert_eq!(group.unit, Unit::Buildings);
        assert_eq!(group.total, buildings as f64, "{id}");
    }
}

#[test]
fn land_use_covers_the_site_and_is_mostly_buildings_and_water() {
    let ratios = ratios_in("paper");
    let land_use = group(&ratios, "land_use");
    // BAG and BGT overlap by a few hundred square metres, never by much.
    assert!((land_use.total - ratios.site_area_m2).abs() < ratios.site_area_m2 * 0.01);
    let buildings = value_of(land_use, "Buildings") / land_use.total;
    let water = value_of(land_use, "Water") / land_use.total;
    assert!((0.40..0.50).contains(&buildings), "buildings {buildings}");
    assert!((0.12..0.20).contains(&water), "water {water}");
    assert!((ratios.coverage - buildings).abs() < 0.01);
}

#[test]
fn canals_are_the_water_of_westermarkt() {
    let ratios = ratios_in("paper");
    let water = group(&ratios, "water");
    assert_eq!(water.rows[0].label, "Canals and streams");
    assert!(water.rows[0].share > 0.99);
}

#[test]
fn parcel_groups_count_the_same_parcels() {
    let ratios = ratios_in("paper");
    let coverage = group(&ratios, "coverage");
    let floor_area = group(&ratios, "floor_area_ratio");
    assert_eq!(coverage.total, floor_area.total);
    assert_eq!(
        value_of(coverage, "No building"),
        value_of(floor_area, "No building")
    );
    // The canal ring is built up to the street: most built parcels are over half covered.
    let dense = value_of(coverage, "50–75 %") + value_of(coverage, "75–100 %");
    assert!(
        dense > coverage.total * 0.4,
        "dense {dense} of {}",
        coverage.total
    );
}

#[test]
fn building_use_takes_the_theme_categories_like_the_diagram() {
    let paper = theme("paper");
    let ratios = ratios_in("paper");
    let residential = &group(&ratios, "building_use").rows[0];
    assert_eq!(residential.label, "Residential");
    assert_eq!(residential.fill, paper.categories[0]);
}

#[test]
fn colours_follow_the_theme() {
    let paper = ratios_in("paper");
    let midnight = ratios_in("midnight");
    let fills = |ratios: &Ratios| group(ratios, "land_use").rows[0].fill.clone();
    assert_ne!(fills(&paper), fills(&midnight));
    assert_eq!(
        group(&paper, "land_use").rows[0].value,
        group(&midnight, "land_use").rows[0].value
    );
}

#[test]
fn csv_has_a_line_per_row_and_the_site_totals() {
    let ratios = ratios_in("paper");
    let csv = ratios::to_csv(&ratios);
    let rows: usize = ratios.groups.iter().map(|group| group.rows.len()).sum();
    assert_eq!(csv.lines().count(), 1 + 4 + rows);
    assert!(csv.starts_with("group,class,value,unit,share_percent,source\nSite,Site area,"));
}

#[test]
fn both_chart_styles_render_parseable_svg_with_a_layer_per_group() {
    let ratios = ratios_in("paper");
    for style in [ChartStyle::Bar, ChartStyle::Pie] {
        let svg = ratios::chart_svg(&ratios, &theme("paper"), style, "Westermarkt");
        usvg::Tree::from_str(&svg, &usvg::Options::default()).expect("chart is valid SVG");
        for group in &ratios.groups {
            assert!(
                svg.contains(&format!(r#"id="{}""#, group.id)),
                "{style:?} {}",
                group.id
            );
        }
    }
}

#[test]
fn every_group_names_its_source() {
    let ratios = ratios_in("paper");
    for group in &ratios.groups {
        assert!(!group.source.is_empty(), "{} has no source line", group.id);
    }
    assert_eq!(
        group(&ratios, "land_use").source,
        "Buildings from BAG, the ground from BGT."
    );
}

/// Prints the numbers, for checking them by eye: `cargo test ratios -- --nocapture`.
#[test]
fn print_westermarkt_ratios() {
    let ratios = ratios_in("paper");
    println!(
        "site {:.0} m², {} buildings, coverage {:.3}, floor area ratio {:.2}",
        ratios.site_area_m2, ratios.building_count, ratios.coverage, ratios.floor_area_ratio
    );
    for group in &ratios.groups {
        println!(
            "{} ({:?}, total {:.0})",
            group.title, group.unit, group.total
        );
        for row in &group.rows {
            println!(
                "  {:<22} {:>6.1} %  {:>10.0}",
                row.label,
                row.share * 100.0,
                row.value
            );
        }
    }
}
