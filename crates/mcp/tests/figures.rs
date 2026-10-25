//! Protocol tests for the figure tools: ratios, the walk, the analysis and the environment.

use geo::{Geometry, Point};
use openarchi_core::library::Library;
use openarchi_core::scene::{Attributes, Feature, Layer, Scene, Walk};
use serde_json::json;

mod common;

use common::{Server, WESTERMARKT_SCENE};

/// The analysis answers the core recorded for the Westermarkt, as tile cache files.
const RECORDED_ANALYSIS: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../core/tests/fixtures/analysis-cache/tiles-v1"
);

/// A small recorded scene with noise, air, flood, slope, population and dwellings.
const WESTERKERK_ENVIRONMENT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../core/tests/fixtures/westerkerk-environment.scene.json"
);

#[test]
fn ratios_before_a_fetch_say_to_fetch_first() {
    let mut server = Server::start();
    let id = server.create_westermarkt();

    let message = server.call_err("get_ratios", json!({ "id": id }));
    assert!(message.contains("fetch_site"), "{message}");
}

#[test]
fn get_ratios_returns_groups_that_add_up_in_the_requested_theme() {
    let mut server = Server::start();
    let id = server.create_westermarkt();
    let library = Library::at(server.library.path()).unwrap();
    let mut project = library.load(&id).unwrap();
    let scene: Scene =
        serde_json::from_str(&std::fs::read_to_string(WESTERMARKT_SCENE).unwrap()).unwrap();
    project.scene = Some(scene);
    library.save(&mut project).unwrap();

    let ratios = server.call_ok("get_ratios", json!({ "id": id, "theme_id": "midnight" }));

    assert!(ratios["building_count"].as_u64().unwrap() > 1000);
    let groups = ratios["groups"].as_array().unwrap();
    let land_use = groups
        .iter()
        .find(|group| group["id"] == json!("land_use"))
        .unwrap();
    assert_eq!(land_use["unit"], json!("square_metres"));
    let shares: f64 = land_use["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["share"].as_f64().unwrap())
        .sum();
    assert!((shares - 1.0).abs() < 1e-9, "{shares}");
    assert_eq!(
        land_use["source"],
        json!("Buildings from BAG, the ground from BGT.")
    );
    let message = server.call_err("get_ratios", json!({ "id": id, "theme_id": "nope" }));
    assert!(message.contains("list_themes"), "{message}");
}

#[test]
fn get_neighbourhood_counts_places_per_walk_band_and_explains_a_missing_walk() {
    let mut server = Server::start();
    let id = server.create_westermarkt();
    let library = Library::at(server.library.path()).unwrap();
    let mut project = library.load(&id).unwrap();
    let mut scene: Scene =
        serde_json::from_str(&std::fs::read_to_string(WESTERMARKT_SCENE).unwrap()).unwrap();
    project.scene = Some(scene.clone());
    library.save(&mut project).unwrap();

    let message = server.call_err("get_neighbourhood", json!({ "id": id }));
    assert!(message.contains("fetch_site"), "{message}");

    let cafe = Feature {
        layer: Layer::Business,
        geometry: Geometry::Point(Point::new(50.0, 0.0)),
        attributes: Attributes {
            group: Some("food_drink".to_string()),
            walk_min: Some(7.5),
            ..Attributes::default()
        },
    };
    scene.walk = Some(Walk {
        bands: vec![],
        paths: vec![],
        places: vec![cafe],
    });
    project.scene = Some(scene);
    library.save(&mut project).unwrap();

    let neighbourhood = server.call_ok("get_neighbourhood", json!({ "id": id }));
    let bands = neighbourhood["bands"].as_array().unwrap();
    let food = |band: usize| bands[band]["businesses"][0]["count"].as_u64().unwrap();
    assert_eq!(bands.len(), 3);
    assert_eq!(bands[1]["minutes"], json!(10));
    assert_eq!(bands[1]["businesses"][0]["group"], json!("food_drink"));
    assert_eq!((food(0), food(1), food(2)), (0, 1, 1));
}

#[test]
fn get_analysis_returns_location_climate_and_notes_from_the_cache() {
    let mut server = Server::start();
    let id = server.create_westermarkt();
    let library = Library::at(server.library.path()).unwrap();
    let mut project = library.load(&id).unwrap();
    let scene: Scene =
        serde_json::from_str(&std::fs::read_to_string(WESTERMARKT_SCENE).unwrap()).unwrap();
    project.scene = Some(scene);
    library.save(&mut project).unwrap();
    seed_analysis_cache(&server.library.path().join("cache"));

    let report = server.call_ok("get_analysis", json!({ "id": id }));
    assert_eq!(report["location"]["municipality"], json!("Amsterdam"));
    assert_eq!(report["climate"]["months"].as_array().unwrap().len(), 12);
    // Outdoor comfort is the UTCI, hours per stress category in the sun and in the shade.
    let categories = report["climate"]["comfort_categories"].as_array().unwrap();
    assert_eq!(categories.len(), 10);
    assert_eq!(categories[5], json!("No thermal stress"));
    assert!(report["climate"]["comfort_method"]
        .as_str()
        .unwrap()
        .contains("UTCI"));
    let july = &report["climate"]["months"][6];
    for exposure in ["comfort_sun_hours", "comfort_shade_hours"] {
        let hours: Vec<u64> = july[exposure]
            .as_array()
            .unwrap()
            .iter()
            .map(|hours| hours.as_u64().unwrap())
            .collect();
        assert_eq!(hours.len(), 10);
        // 31 days from 8:00 to 22:00, mostly free of thermal stress.
        assert_eq!(hours.iter().sum::<u64>(), 31 * 14);
        assert!(hours[5] * 2 > 31 * 14, "{exposure}: {hours:?}");
    }
    assert!(report["summary"]["coverage"].as_f64().unwrap() > 0.0);
    let sections: Vec<&str> = report["notes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["section"].as_str().unwrap())
        .collect();
    assert!(
        sections.contains(&"wind") && sections.contains(&"sun"),
        "{sections:?}"
    );
    for entry in report["notes"].as_array().unwrap() {
        let source = entry["source"].as_str().unwrap();
        assert!(!source.is_empty(), "{} has no source", entry["section"]);
    }
    assert_eq!(report["summary"]["is_sheet_area"], json!(false));

    // On a sheet with a sub-selection only the site summary follows the area.
    server.open_circle_as_sheet(&id, "Westerkerk", 60.0);
    let framed = server.call_ok("get_analysis", json!({ "id": id, "tab": 2 }));
    assert_eq!(framed["summary"]["is_sheet_area"], json!(true));
    assert!(
        framed["summary"]["site_area_m2"].as_f64() < report["summary"]["site_area_m2"].as_f64()
    );
    assert_eq!(framed["location"], report["location"]);
    assert_eq!(framed["climate"], report["climate"]);
    let site_source = framed["notes"][1]["source"].as_str().unwrap();
    assert!(
        site_source.starts_with("Of this sheet's area"),
        "{site_source}"
    );
}

/// Copies the answers the core recorded for the Westermarkt into a library's
/// tile cache, dated now so none counts as stale, so get_analysis runs offline.
/// Rain is recorded up to 2025; from 2027 on the server asks KNMI for newer
/// years, and the test does not depend on that answer.
fn seed_analysis_cache(cache: &std::path::Path) {
    let recorded = std::path::Path::new(RECORDED_ANALYSIS);
    for dataset in std::fs::read_dir(recorded).unwrap() {
        let dataset = dataset.unwrap().path();
        let target = cache.join("tiles-v1").join(dataset.file_name().unwrap());
        std::fs::create_dir_all(&target).unwrap();
        for file in std::fs::read_dir(&dataset).unwrap() {
            let file = file.unwrap().path();
            let copy = target.join(file.file_name().unwrap());
            std::fs::copy(&file, &copy).unwrap();
            let handle = std::fs::File::options().write(true).open(&copy).unwrap();
            handle.set_modified(std::time::SystemTime::now()).unwrap();
        }
    }
}

#[test]
fn get_environment_gives_noise_flood_and_people_with_their_years() {
    let mut server = Server::start();
    let id = server.create_westermarkt();
    let library = Library::at(server.library.path()).unwrap();
    let mut project = library.load(&id).unwrap();

    let message = server.call_err("get_environment", json!({ "id": id }));
    assert!(message.contains("fetch_site"), "{message}");

    let scene: Scene =
        serde_json::from_str(&std::fs::read_to_string(WESTERKERK_ENVIRONMENT).unwrap()).unwrap();
    project.scene = Some(scene);
    library.save(&mut project).unwrap();

    let environment = server.call_ok("get_environment", json!({ "id": id }));
    assert!(environment["road_noise_mean_db"].as_f64().unwrap() > 50.0);
    assert!(environment["flood_max_depth_m"].as_f64().unwrap() > 0.0);
    assert!(environment["residents"].as_f64().unwrap() > 0.0);
    assert_eq!(environment["noise_year"], json!(2022));
    assert_eq!(environment["whole_site"], json!([]));

    // The fixture was fetched before rasters were kept, so a sheet area keeps
    // the whole site's figures and says which ones they are.
    server.open_circle_as_sheet(&id, "Westerkerk", 40.0);
    let part = server.call_ok("get_environment", json!({ "id": id, "tab": 2 }));
    assert_eq!(
        part["road_noise_mean_db"],
        environment["road_noise_mean_db"]
    );
    let whole_site = part["whole_site"].as_array().unwrap();
    assert!(whole_site.contains(&json!("road_noise")), "{whole_site:?}");
    assert!(whole_site.contains(&json!("slope")), "{whole_site:?}");
}

#[test]
fn get_environment_gives_tree_canopy_and_heat_island_and_the_diagrams_are_listed() {
    let mut server = Server::start();
    let id = server.create_westermarkt();
    let library = Library::at(server.library.path()).unwrap();
    let mut project = library.load(&id).unwrap();
    let mut scene: Scene =
        serde_json::from_str(&std::fs::read_to_string(WESTERKERK_ENVIRONMENT).unwrap()).unwrap();

    // Fetched before trees and heat were read: no figure, not zero.
    project.scene = Some(scene.clone());
    library.save(&mut project).unwrap();
    let before = server.call_ok("get_environment", json!({ "id": id }));
    assert_eq!(before["tree_canopy_share"], json!(null));
    assert_eq!(before["heat_island_mean_c"], json!(null));

    // The tree map was read and met no tree; the heat island was measured.
    let mut tree_credit = scene.credits[0].clone();
    tree_credit.layers = vec![Layer::TreeCanopy, Layer::GreenCover];
    scene.credits.push(tree_credit);
    scene.measured.heat_island_c = Some(openarchi_core::sources::raster::Stats {
        min: 1.8,
        mean: 2.2,
        max: 2.6,
        coverage: 1.0,
    });
    scene.editions.insert(Layer::HeatIsland, 2022);
    project.scene = Some(scene);
    library.save(&mut project).unwrap();
    let after = server.call_ok("get_environment", json!({ "id": id }));
    assert_eq!(after["tree_canopy_share"], json!(0.0));
    assert_eq!(after["heat_island_mean_c"], json!(2.2));
    assert_eq!(after["heat_island_year"], json!(2022));

    let diagrams = server.call_ok("list_diagrams", json!({}));
    let ids: Vec<&str> = diagrams
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["id"].as_str().unwrap())
        .collect();
    assert!(
        ids.contains(&"greenery") && ids.contains(&"heat-island"),
        "{ids:?}"
    );
}
