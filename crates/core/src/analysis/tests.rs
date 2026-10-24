//! The analysis of the recorded Westermarkt scene, offline: a temporary tile
//! cache is filled with the answers `examples/analysis_samples.rs --record`
//! saved, so every part runs without asking a service.

use std::fs;
use std::path::Path;

use tempfile::TempDir;

use super::*;
use crate::catalog::builtin_themes;
use crate::export::svg_to_png;

const SCENE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/westermarkt.scene.json"
);
const RECORDED: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/analysis-cache/tiles-v1"
);
/// The year the recorded rain ends with; the clock moves on, the fixture does not.
const RECORDED_LAST_YEAR: i32 = 2025;
const PREVIEW_WIDTH_PX: u32 = 300;

fn westermarkt() -> Scene {
    let json = fs::read_to_string(SCENE).expect("fixture is checked in");
    serde_json::from_str(&json).expect("fixture is a serialised Scene")
}

fn theme(id: &str) -> Theme {
    builtin_themes()
        .into_iter()
        .find(|theme| theme.id == id)
        .unwrap()
}

/// A cache holding every recorded answer, written fresh so none counts as stale.
fn recorded_cache() -> (TempDir, TileCache) {
    let folder = TempDir::new().unwrap();
    let cache = TileCache::at(folder.path());
    for dataset in fs::read_dir(RECORDED).unwrap() {
        let dataset = dataset.unwrap().path();
        let name = dataset.file_name().unwrap().to_string_lossy().to_string();
        for file in fs::read_dir(&dataset).unwrap() {
            let path = file.unwrap().path();
            let bytes = zstd::decode_all(fs::read(&path).unwrap().as_slice()).unwrap();
            cache.write(&name, tile_of(&path), &bytes).unwrap();
        }
    }

    (folder, cache)
}

/// "24_97.zst" -> column 24, row 97.
fn tile_of(path: &Path) -> Tile {
    let stem = path.file_stem().unwrap().to_string_lossy().to_string();
    let (column, row) = stem.split_once('_').unwrap();
    Tile {
        column: column.parse().unwrap(),
        row: row.parse().unwrap(),
    }
}

#[test]
fn the_location_is_amsterdam_at_the_westermarkt() {
    let (_folder, cache) = recorded_cache();
    let location = location::locate(&westermarkt(), &cache);
    assert_eq!(location.municipality.as_deref(), Some("Amsterdam"));
    assert_eq!(location.province.as_deref(), Some("Noord-Holland"));
    assert!(location
        .address
        .as_deref()
        .unwrap()
        .starts_with("Westermarkt"));
    assert!(location.warnings.is_empty(), "{:?}", location.warnings);
}

#[test]
fn the_summary_agrees_with_the_ratios() {
    let scene = westermarkt();
    let ratios = ratios::compute(&scene, &theme("paper")).unwrap();
    let summary = summary::summarise(&scene, &ratios);
    assert_eq!(summary.coverage, ratios.coverage);
    assert_eq!(summary.building_count, ratios.building_count);
    // Zero would mean a land use label changed in ratios.rs and the summary lost it.
    assert!(summary.green_share > 0.0 && summary.water_share > 0.0 && summary.paved_share > 0.0);
    assert!(summary.dominant_use.is_some() && summary.median_year_built.is_some());
    assert!(summary.tallest_m.unwrap() >= summary.mean_height_m.unwrap());
}

#[test]
fn the_summary_counts_the_walk_and_names_what_is_missing() {
    use crate::scene::{Attributes, Feature, Layer, Walk};
    let place = |layer: Layer, group: &str, walk_min: f64| Feature {
        layer,
        geometry: geo::Geometry::Point(geo::Point::new(0.0, 0.0)),
        attributes: Attributes {
            group: Some(group.to_string()),
            walk_min: Some(walk_min),
            ..Attributes::default()
        },
    };
    let mut scene = westermarkt();
    scene.walk = Some(Walk {
        bands: vec![],
        paths: vec![],
        places: vec![
            place(Layer::Facility, "education", 6.0),
            place(Layer::Facility, "health", 14.0),
            place(Layer::Business, "shop", 3.0),
            place(Layer::TransitStop, "tram", 8.0),
        ],
    });
    let ratios = ratios::compute(&scene, &theme("paper")).unwrap();
    let summary = summary::summarise(&scene, &ratios);
    let surroundings = &summary.surroundings;

    assert_eq!(surroundings.facilities_within_walk, Some(2));
    assert_eq!(surroundings.businesses_within_walk, Some(1));
    // The tram stop is 8 minutes away, outside the 5 minute walk.
    assert_eq!(surroundings.transit_stops_within_walk, Some(0));
    assert!(surroundings
        .facility_groups_missing
        .contains(&"Culture".to_string()));
    let notes = notes::site_notes(&summary);
    assert!(notes.iter().any(|note| note.starts_with("No transit stop")));
}

#[test]
fn the_site_section_shows_noise_air_and_people_from_the_environment() {
    const ENVIRONMENT_SCENE: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/westerkerk-environment.scene.json"
    );
    let json = fs::read_to_string(ENVIRONMENT_SCENE).expect("fixture is checked in");
    let scene: Scene = serde_json::from_str(&json).expect("fixture is a serialised Scene");
    let theme = theme("paper");
    let ratios = ratios::compute(&scene, &theme).unwrap();
    let summary = summary::summarise(&scene, &ratios);
    assert!(summary
        .surroundings
        .environment
        .road_noise_mean_db
        .is_some());

    let section = sections::site(&summary, &ratios, scene.credits.clone(), &theme).unwrap();
    let labels: Vec<&str> = section
        .facts
        .iter()
        .map(|fact| fact.label.as_str())
        .collect();
    assert!(
        labels.iter().any(|label| label.starts_with("Road noise")),
        "{labels:?}"
    );
    assert!(
        labels.iter().any(|label| label.starts_with("NO₂")),
        "{labels:?}"
    );
}

#[test]
fn the_climate_is_a_mild_dutch_year_with_south_westerly_winter_wind() {
    let (_folder, cache) = recorded_cache();
    let (climate, warnings) = climate(&westermarkt(), &cache).unwrap();
    assert!(warnings.is_empty());

    let annual = climate.months.iter().map(|month| month.mean_c).sum::<f64>() / 12.0;
    assert!((9.0..13.0).contains(&annual), "{annual}");
    // Every hour from 8:00 to 22:00 of a 365 day year lands in one comfort band.
    let daytime: u32 = climate
        .months
        .iter()
        .flat_map(|month| &month.comfort_hours)
        .sum();
    assert_eq!(
        daytime,
        365 * (climate::DAY_END_HOUR - climate::DAY_START_HOUR)
    );

    let winter = &climate.wind[0];
    let prevailing = winter
        .sectors
        .iter()
        .max_by(|a, b| a.total().total_cmp(&b.total()))
        .unwrap();
    assert!(
        ["S", "SSW", "SW", "WSW", "W"].contains(&prevailing.from.as_str()),
        "{}",
        prevailing.from
    );
    let shares: f64 = winter.calm_share
        + winter
            .sectors
            .iter()
            .map(|sector| sector.total())
            .sum::<f64>();
    assert!((shares - 1.0).abs() < 1e-9);

    // In the Netherlands no direct sun comes from the north.
    assert_eq!(
        climate.sun[0].winter_kwh_m2 + climate.sun[0].summer_kwh_m2,
        0.0
    );
}

#[test]
fn rain_comes_from_schiphol_over_thirty_full_years() {
    let (_folder, cache) = recorded_cache();
    let (rain, _) = rain(&westermarkt(), &cache, RECORDED_LAST_YEAR).unwrap();
    assert_eq!(rain.station.id, 240);
    assert_eq!((rain.first_year, rain.last_year), (1996, 2025));
    assert!(rain.complete_years >= 28, "{}", rain.complete_years);
    assert!(
        (700.0..1000.0).contains(&rain.annual_mm),
        "{}",
        rain.annual_mm
    );
}

#[test]
fn every_section_draws_in_every_theme_and_converts() {
    let (_folder, cache) = recorded_cache();
    let scene = westermarkt();
    for theme in builtin_themes() {
        let mut sections = Vec::new();
        for part in [Part::Location, Part::Site, Part::Climate] {
            sections.extend(super::part(part, &scene, &theme, &cache).unwrap());
        }
        let (rain, warnings) = rain(&scene, &cache, RECORDED_LAST_YEAR).unwrap();
        sections.push(sections::rain(&rain, warnings, &theme).unwrap());

        assert_eq!(sections.len(), 8);
        for section in &sections {
            assert!(!section.figures.is_empty(), "{} has no figure", section.id);
            assert!(section
                .figures
                .iter()
                .all(|figure| figure.svg.contains(&theme.palette.paper)));
        }
        let page = report_svg(&sections, &[], &theme, "Westermarkt").unwrap();
        svg_to_png(&page, PREVIEW_WIDTH_PX).unwrap_or_else(|error| panic!("{}: {error}", theme.id));
    }
}

#[test]
fn every_climate_section_has_design_notes() {
    let (_folder, cache) = recorded_cache();
    let (climate, _) = climate(&westermarkt(), &cache).unwrap();
    for section in sections::climate_notes(&climate) {
        assert!(
            !section.notes.is_empty(),
            "{} has no notes",
            section.section
        );
    }
}

#[test]
fn a_part_that_cannot_be_fetched_is_an_error_and_nothing_is_cached() {
    let folder = TempDir::new().unwrap();
    let cache = TileCache::at(folder.path());
    let key = Tile { column: 1, row: 1 };
    let result = cached(&cache, "test", key, || {
        Err(Error::Network {
            service: "test".to_string(),
            detail: "offline".to_string(),
        })
    });
    assert!(result.is_err());
    assert_eq!(cache.read("test", key).unwrap(), None);
}
