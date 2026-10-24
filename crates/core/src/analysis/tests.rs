//! The analysis of the recorded Westermarkt scene, offline: a temporary tile
//! cache is filled with the answers `examples/analysis_samples.rs --record`
//! saved, so every part runs without asking a service.

use std::fs;
use std::path::Path;

use tempfile::TempDir;

use super::*;
use crate::catalog::builtin_themes;
use crate::export::{svg_pages_to_pdf, svg_to_png};
use comfort::{yearly_hours, Exposure};

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
    let summary = summary::summarise(&scene, &ratios, false);
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
    let summary = summary::summarise(&scene, &ratios, false);
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
    let summary = summary::summarise(&scene, &ratios, false);
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
fn on_a_sheet_area_the_site_part_sums_up_that_area_and_says_so() {
    use crate::area::{Area, LonLat};
    let mut scene = westermarkt();
    // An empty walk is enough for the walk counts, and the note about where they start, to appear.
    scene.walk = Some(crate::scene::Walk {
        bands: Vec::new(),
        paths: Vec::new(),
        places: Vec::new(),
    });
    let selection = Area::Circle {
        center: LonLat {
            lon: 4.8840,
            lat: 52.3745,
        },
        radius_m: 60.0,
    };
    let sheet_area = crate::selection::framed(&scene, &selection).unwrap();
    let (_folder, cache) = recorded_cache();
    let theme = theme("paper");

    let whole = part(Part::Site, &scene, None, &theme, &cache).unwrap();
    let framed = part(Part::Site, &scene, Some(&sheet_area), &theme, &cache).unwrap();
    assert_eq!(whole[0].facts[0].label, "Site area");
    assert_eq!(framed[0].facts[0].label, "Sheet area");
    assert_ne!(whole[0].facts[0].value, framed[0].facts[0].value);
    assert!(!whole[0].source.contains("sheet's area"));
    assert!(framed[0].source.starts_with("Of this sheet's area only"));
    // The walk starts at the site centre, which a sheet area's line says when it counts places.
    let walk_note = "counted from the site's centre";
    assert!(!whole[0].source.contains(walk_note));
    assert!(framed[0].source.contains(walk_note), "{}", framed[0].source);

    let report = report(&scene, Some(&sheet_area), &theme, &cache).unwrap();
    let area_ratios = ratios::compute(&sheet_area, &theme).unwrap();
    assert!(report.summary.is_sheet_area);
    assert_eq!(report.summary.building_count, area_ratios.building_count);
}

#[test]
fn the_climate_is_a_mild_dutch_year_with_south_westerly_winter_wind() {
    let (_folder, cache) = recorded_cache();
    let (climate, warnings) = climate(&westermarkt(), &cache).unwrap();
    assert!(warnings.is_empty());

    let annual = climate.months.iter().map(|month| month.mean_c).sum::<f64>() / 12.0;
    assert!((9.0..13.0).contains(&annual), "{annual}");
    // Every hour from 8:00 to 22:00 of a 365 day year lands in one comfort
    // category, once in the sun and once in the shade.
    for exposure in [Exposure::Sun, Exposure::Shade] {
        let daytime: u32 = yearly_hours(&climate, exposure).iter().sum();
        assert_eq!(
            daytime,
            365 * (climate::DAY_END_HOUR - climate::DAY_START_HOUR)
        );
    }

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
fn outdoor_comfort_in_amsterdam_is_mild_summers_and_cold_windy_winters() {
    let (_folder, cache) = recorded_cache();
    let (climate, _) = climate(&westermarkt(), &cache).unwrap();
    let share = |months: &[u32], exposure: Exposure, categories: std::ops::Range<usize>| {
        let mut total = 0;
        let mut inside = 0;
        for month in climate
            .months
            .iter()
            .filter(|month| months.contains(&month.month))
        {
            let hours = exposure.hours(month);
            total += hours.iter().sum::<u32>();
            inside += hours[categories.clone()].iter().sum::<u32>();
        }
        inside as f64 / total as f64
    };

    // Summer days are mostly free of thermal stress, sun or shade; winter
    // days in the shade are mostly moderate or strong cold stress, wind included.
    let summer = [6, 7, 8];
    let winter = [12, 1, 2];
    let no_stress = utci::NO_STRESS..utci::NO_STRESS + 1;
    let moderate_or_strong_cold = utci::NO_STRESS - 3..utci::NO_STRESS - 1;
    assert!(share(&summer, Exposure::Sun, no_stress.clone()) > 0.6);
    assert!(share(&summer, Exposure::Shade, no_stress) > 0.6);
    assert!(share(&winter, Exposure::Shade, moderate_or_strong_cold) > 0.5);
    assert_eq!(climate.comfort_outside_range_hours, 0);
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
fn the_whole_analysis_lays_out_on_numbered_a4_pages_with_credits_last() {
    let (_folder, cache) = recorded_cache();
    let scene = westermarkt();
    let theme = theme("paper");
    let mut sections = Vec::new();
    for part in [Part::Location, Part::Site, Part::Climate] {
        sections.extend(super::part(part, &scene, None, &theme, &cache).unwrap());
    }
    let failures = vec![("Rain".to_string(), "offline".to_string())];

    let pages = report_pages(&sections, &failures, &theme, "Westermarkt").unwrap();
    assert!(pages.len() >= 3, "{}", pages.len());
    let count = pages.len();
    for (index, page) in pages.iter().enumerate() {
        assert!(page.contains(r#"width="210mm" height="297mm""#));
        assert!(page.contains(&format!(">{} / {count}<", index + 1)));
        // Every page after the first names the project and the site in its header.
        if index > 0 {
            assert!(page.contains("Westermarkt 68, Amsterdam"));
        }
    }
    let last = pages.last().unwrap();
    assert!(last.contains("Data credits") && last.contains("PVGIS"));
    assert!(pages
        .iter()
        .any(|page| page.contains("Rain could not be fetched: offline")));
    // A section is never cut across two pages.
    for section in &sections {
        let layer = format!(r#"id="{}""#, section.id);
        let holding = pages.iter().filter(|page| page.contains(&layer)).count();
        assert_eq!(holding, 1, "{}", section.id);
    }

    let pdf = svg_pages_to_pdf(&pages, "Westermarkt").unwrap();
    let text = String::from_utf8_lossy(&pdf);
    assert!(text.starts_with("%PDF"));
    assert!(text.contains(&format!("/Count {count}")));
}

#[test]
fn every_section_draws_in_every_theme_and_converts() {
    let (_folder, cache) = recorded_cache();
    let scene = westermarkt();
    for theme in builtin_themes() {
        let mut sections = Vec::new();
        for part in [Part::Location, Part::Site, Part::Climate] {
            sections.extend(super::part(part, &scene, None, &theme, &cache).unwrap());
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
