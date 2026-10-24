//! Drives the real `openarchi-mcp` binary over stdio, the way Claude does,
//! against a throwaway library.
//!
//! Tests marked `#[ignore]` need the data or render layer, which are still
//! `todo!()`. Run them with `cargo test -p openarchi-mcp -- --ignored` once
//! those land, then drop the `#[ignore]`.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use geo::{Geometry, Point};
use openarchi_core::library::Library;
use openarchi_core::scene::{Attributes, Feature, Layer, Scene, Walk};
use serde_json::{json, Value};
use tempfile::TempDir;

/// The newest protocol version that still starts with an initialize handshake.
const PROTOCOL_VERSION: &str = "2025-11-25";

/// The core's recorded Westermarkt scene, so a test can have a fetched site offline.
const WESTERMARKT_SCENE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../core/tests/fixtures/westermarkt.scene.json"
);

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

/// Every tool the server must offer, in one place.
const TOOLS: [&str; 18] = [
    "search_places",
    "list_projects",
    "create_project",
    "get_project",
    "fetch_site",
    "list_diagrams",
    "list_themes",
    "set_look",
    "edit_text",
    "render_preview",
    "get_ratios",
    "get_neighbourhood",
    "get_analysis",
    "get_environment",
    "export",
    "create_theme",
    "delete_project",
    "restore_project",
];

/// A running server, initialized, with its own empty library.
struct Server {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
    library: TempDir,
}

impl Server {
    fn start() -> Server {
        let library = TempDir::new().unwrap();
        let mut child = Command::new(env!("CARGO_BIN_EXE_openarchi-mcp"))
            .env("OPENARCHI_LIBRARY_DIR", library.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());

        let mut server = Server {
            child,
            stdin,
            stdout,
            next_id: 1,
            library,
        };
        server.request(
            "initialize",
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": { "name": "protocol-test", "version": "0" },
            }),
        );
        server.send(json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }));
        server
    }

    fn send(&mut self, message: Value) {
        writeln!(self.stdin, "{message}").unwrap();
        self.stdin.flush().unwrap();
    }

    /// Sends a request and returns its `result`, skipping any notifications.
    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        self.send(json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));

        loop {
            let mut line = String::new();
            assert!(
                self.stdout.read_line(&mut line).unwrap() > 0,
                "server closed"
            );
            let message: Value = serde_json::from_str(&line).unwrap();
            if message["id"] == json!(id) {
                assert!(message.get("error").is_none(), "{message}");
                return message["result"].clone();
            }
        }
    }

    /// Calls a tool and returns its result as is, error or not.
    fn call(&mut self, tool: &str, arguments: Value) -> Value {
        self.request(
            "tools/call",
            json!({ "name": tool, "arguments": arguments }),
        )
    }

    /// Calls a tool that must succeed and parses its JSON text.
    fn call_ok(&mut self, tool: &str, arguments: Value) -> Value {
        let result = self.call(tool, arguments);
        assert_eq!(result["isError"], json!(false), "{tool} failed: {result}");
        serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap()
    }

    /// Calls a tool that must fail and returns its message.
    fn call_err(&mut self, tool: &str, arguments: Value) -> String {
        let result = self.call(tool, arguments);
        assert_eq!(result["isError"], json!(true), "{tool} succeeded: {result}");
        result["content"][0]["text"].as_str().unwrap().to_string()
    }

    fn create_westermarkt(&mut self) -> String {
        let created = self.call_ok(
            "create_project",
            json!({
                "name": "Westermarkt",
                "area": {
                    "kind": "circle",
                    "center": { "lon": 4.8837, "lat": 52.3747 },
                    "radius_m": 150.0,
                },
            }),
        );
        created["id"].as_str().unwrap().to_string()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn tools_list_returns_every_tool_with_an_object_schema() {
    let mut server = Server::start();
    let listed = server.request("tools/list", json!({}));

    let mut names: Vec<&str> = listed["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect();
    names.sort();
    let mut expected = TOOLS.to_vec();
    expected.sort();
    assert_eq!(names, expected);

    for tool in listed["tools"].as_array().unwrap() {
        assert_eq!(tool["inputSchema"]["type"], json!("object"), "{tool}");
        assert!(!tool["description"].as_str().unwrap().is_empty(), "{tool}");
    }
}

#[test]
fn initialize_names_the_server_and_gives_instructions() {
    let library = TempDir::new().unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_openarchi-mcp"))
        .env("OPENARCHI_LIBRARY_DIR", library.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    let initialize = json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": {},
            "clientInfo": { "name": "protocol-test", "version": "0" },
        },
    });
    writeln!(stdin, "{initialize}").unwrap();

    let mut line = String::new();
    stdout.read_line(&mut line).unwrap();
    let reply: Value = serde_json::from_str(&line).unwrap();
    child.kill().unwrap();
    child.wait().unwrap();

    assert_eq!(reply["result"]["serverInfo"]["name"], json!("openarchi"));
    assert!(reply["result"]["instructions"]
        .as_str()
        .unwrap()
        .contains("fetch_site"));
}

#[test]
fn created_project_is_listed_and_written_to_the_library_folder() {
    let mut server = Server::start();
    let id = server.create_westermarkt();

    let projects = server.call_ok("list_projects", json!({}));
    assert_eq!(projects[0]["id"], json!(id));
    assert_eq!(projects[0]["has_scene"], json!(false));
    let file = server
        .library
        .path()
        .join(format!("projects/{id}.openarchi"));
    assert!(file.exists());
}

#[test]
fn get_project_shows_the_look_and_no_site_before_a_fetch() {
    let mut server = Server::start();
    let id = server.create_westermarkt();

    let project = server.call_ok("get_project", json!({ "id": id }));

    assert_eq!(project["name"], json!("Westermarkt"));
    assert_eq!(project["area"]["kind"], json!("circle"));
    assert_eq!(project["look"]["sheet"]["paper"], json!("a3"));
    assert_eq!(project["site"], Value::Null);
}

#[test]
fn set_look_changes_only_the_passed_sheet_fields() {
    let mut server = Server::start();
    let id = server.create_westermarkt();

    let look = server.call_ok(
        "set_look",
        json!({
            "id": id,
            "sheet": { "paper": "a1", "title": "Westermarkt" },
            "overrides": { "water": { "fill": "#3366aa" } },
        }),
    );

    assert_eq!(look["sheet"]["paper"], json!("a1"));
    assert_eq!(look["sheet"]["title"], json!("Westermarkt"));
    assert_eq!(look["sheet"]["frame"], json!("circle"));
    assert_eq!(look["overrides"]["water"]["fill"], json!("#3366aa"));
    let saved = server.call_ok("get_project", json!({ "id": id }));
    assert_eq!(saved["look"], look);
}

#[test]
fn edit_text_adds_a_box_and_moves_the_title_and_both_are_saved() {
    let mut server = Server::start();
    let id = server.create_westermarkt();

    let added = server.call_ok(
        "edit_text",
        json!({
            "id": id,
            "text_id": "new",
            "text": "Westerkerk",
            "style": { "family": "Source Serif 4", "italic": true, "size_pt": 18, "align": "centre", "colour": "@accent" },
            "position": { "x_mm": 120, "y_mm": 90 },
        }),
    );
    server.call_ok(
        "edit_text",
        json!({ "id": id, "text_id": "title", "position": { "x_mm": 20, "y_mm": 20 } }),
    );

    assert_eq!(added["text_id"], json!("text-1"));
    assert_eq!(added["paper_mm"]["width"], json!(420.0));
    let sheet = &server.call_ok("get_project", json!({ "id": id }))["look"]["sheet"];
    assert_eq!(sheet["texts"][0]["style"]["align"], json!("centre"));
    assert_eq!(sheet["texts"][0]["position"]["x_mm"], json!(120.0));
    assert_eq!(sheet["title_position"]["y_mm"], json!(20.0));
}

#[test]
fn edit_text_with_an_unknown_font_names_the_bundled_ones() {
    let mut server = Server::start();
    let id = server.create_westermarkt();

    let message = server.call_err(
        "edit_text",
        json!({ "id": id, "text_id": "title", "style": { "family": "Helvetica" } }),
    );
    assert!(message.contains("IBM Plex Mono"), "{message}");
}

#[test]
fn set_look_picks_a_variant_and_keeps_it_while_the_diagram_stays() {
    let mut server = Server::start();
    let id = server.create_westermarkt();

    let chosen = server.call_ok(
        "set_look",
        json!({ "id": id, "diagram_id": "air-quality", "variant": "pm25" }),
    );
    let themed = server.call_ok("set_look", json!({ "id": id, "theme_id": "ink" }));
    let moved_on = server.call_ok("set_look", json!({ "id": id, "diagram_id": "noise" }));

    assert_eq!(chosen["variant"], json!("pm25"));
    assert_eq!(themed["variant"], json!("pm25"));
    assert_eq!(moved_on["diagram_id"], json!("noise"));
    assert_eq!(moved_on["variant"], Value::Null);
}

#[test]
fn set_look_with_an_old_diagram_id_lands_on_its_variant() {
    let mut server = Server::start();
    let id = server.create_westermarkt();

    let look = server.call_ok(
        "set_look",
        json!({ "id": id, "diagram_id": "noise-all-sources" }),
    );

    assert_eq!(look["diagram_id"], json!("noise"));
    assert_eq!(look["variant"], json!("all"));
}

#[test]
fn set_look_with_an_unknown_variant_names_the_ones_there_are() {
    let mut server = Server::start();
    let id = server.create_westermarkt();

    let message = server.call_err(
        "set_look",
        json!({ "id": id, "diagram_id": "air-quality", "variant": "ozone" }),
    );
    let plain = server.call_err(
        "set_look",
        json!({ "id": id, "diagram_id": "context", "variant": "pm25" }),
    );

    assert!(message.contains("no2, pm10, pm25"), "{message}");
    assert!(plain.contains("no variants"), "{plain}");
}

#[test]
fn set_look_takes_a_plate_for_one_variant() {
    let mut server = Server::start();
    let id = server.create_westermarkt();

    let look = server.call_ok(
        "set_look",
        json!({ "id": id, "view": { "mode": "explode", "plates": ["air-quality:pm10", "context"] } }),
    );
    let message = server.call_err(
        "set_look",
        json!({ "id": id, "view": { "plates": ["air-quality:ozone"] } }),
    );

    assert_eq!(look["view"]["plates"][0], json!("air-quality:pm10"));
    assert!(message.contains("id:variant"), "{message}");
}

#[test]
fn set_look_with_an_unknown_layer_is_a_readable_error() {
    let mut server = Server::start();
    let id = server.create_westermarkt();

    let message = server.call_err(
        "set_look",
        json!({ "id": id, "overrides": { "buildings": { "hidden": true } } }),
    );
    assert!(message.contains("unknown layer"), "{message}");
}

#[test]
fn deleted_project_leaves_the_list_and_restores() {
    let mut server = Server::start();
    let id = server.create_westermarkt();

    let deleted = server.call_ok("delete_project", json!({ "id": id }));
    assert!(deleted["message"]
        .as_str()
        .unwrap()
        .contains("Recently Deleted"));
    assert_eq!(server.call_ok("list_projects", json!({})), json!([]));

    server.call_ok("restore_project", json!({ "id": id }));
    assert_eq!(
        server.call_ok("list_projects", json!({}))[0]["id"],
        json!(id)
    );
}

#[test]
fn unknown_project_is_a_readable_error_and_the_server_keeps_going() {
    let mut server = Server::start();

    let message = server.call_err("get_project", json!({ "id": "2026-01-01-nowhere-0000" }));
    assert!(message.contains("no project"), "{message}");
    server.call_ok("list_projects", json!({}));
}

#[test]
fn preview_before_a_fetch_says_to_fetch_first() {
    let mut server = Server::start();
    let id = server.create_westermarkt();

    let message = server.call_err("render_preview", json!({ "id": id }));
    assert!(message.contains("fetch_site"), "{message}");
}

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
}

#[test]
fn export_refuses_a_relative_path() {
    let mut server = Server::start();
    let id = server.create_westermarkt();

    let message = server.call_err(
        "export",
        json!({ "id": id, "format": "svg", "path": "site.svg" }),
    );
    assert!(message.contains("absolute"), "{message}");
}

#[test]
#[ignore = "needs geocode::search from the data layer"]
fn search_places_finds_westermarkt() {
    let mut server = Server::start();
    let places = server.call_ok(
        "search_places",
        json!({ "query": "Westermarkt 20 Amsterdam", "limit": 3 }),
    );

    let first = &places[0];
    assert!(first["label"].as_str().unwrap().contains("Westermarkt"));
    assert!((first["location"]["lon"].as_f64().unwrap() - 4.88).abs() < 0.01);
}

#[test]
#[ignore = "needs site::build_scene from the data layer, and the network"]
fn fetch_site_saves_a_scene_and_get_project_summarizes_it() {
    let mut server = Server::start();
    let id = server.create_westermarkt();

    let summary = server.call_ok("fetch_site", json!({ "id": id }));
    assert!(summary["buildings"]["count"].as_u64().unwrap() > 0);

    let project = server.call_ok("get_project", json!({ "id": id }));
    assert!(
        project["site"]["features_per_layer"]["building"]
            .as_u64()
            .unwrap()
            > 0
    );
}

#[test]
fn list_diagrams_includes_the_default_diagram() {
    let mut server = Server::start();
    let diagrams = server.call_ok("list_diagrams", json!({}));
    assert!(diagrams
        .as_array()
        .unwrap()
        .iter()
        .any(|d| d["id"] == json!("context")));
}

#[test]
fn list_diagrams_shows_groups_and_the_variants_of_air_quality() {
    let mut server = Server::start();
    let diagrams = server.call_ok("list_diagrams", json!({}));
    let air = diagrams
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["id"] == json!("air-quality"))
        .unwrap()
        .clone();

    let variants: Vec<&str> = air["variants"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["id"].as_str().unwrap())
        .collect();
    assert_eq!(variants, ["no2", "pm10", "pm25"]);
    assert_eq!(air["group"], json!("Environment"));
    assert_eq!(air["variants"][2]["classify"]["layer"], json!("pm25"));
}

#[test]
#[ignore = "needs catalog::all_themes from the render layer"]
fn list_themes_includes_the_default_theme_with_its_palette() {
    let mut server = Server::start();
    let listed = server.call_ok("list_themes", json!({}));
    let paper = listed["themes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|theme| theme["id"] == json!("paper"))
        .unwrap()
        .clone();
    assert!(paper["palette"]["figure"]
        .as_str()
        .unwrap()
        .starts_with('#'));
}

#[test]
#[ignore = "needs catalog from the render layer"]
fn set_look_refuses_an_unknown_theme() {
    let mut server = Server::start();
    let id = server.create_westermarkt();
    let message = server.call_err("set_look", json!({ "id": id, "theme_id": "no-such-theme" }));
    assert!(message.contains("list_themes"), "{message}");
}

#[test]
#[ignore = "needs catalog::read_theme from the render layer"]
fn created_theme_is_saved_and_listed() {
    let mut server = Server::start();
    let listed = server.call_ok("list_themes", json!({ "full": true }));
    let mut theme = listed["themes"][0].clone();
    theme["id"] = json!("assistant-test");
    theme["name"] = json!("Assistant test");

    server.call_ok("create_theme", json!({ "theme_json": theme }));

    assert!(server
        .library
        .path()
        .join("themes/assistant-test.json")
        .exists());
    let listed = server.call_ok("list_themes", json!({}));
    assert!(listed["themes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|t| t["id"] == json!("assistant-test")));
}

#[test]
#[ignore = "needs the data and render layers, and the network"]
fn render_preview_returns_a_png_image() {
    let mut server = Server::start();
    let id = server.create_westermarkt();
    server.call_ok("fetch_site", json!({ "id": id }));

    let result = server.call("render_preview", json!({ "id": id, "width_px": 400 }));

    assert_eq!(result["isError"], json!(false), "{result}");
    assert_eq!(result["content"][0]["type"], json!("image"));
    assert_eq!(result["content"][0]["mimeType"], json!("image/png"));
}

#[test]
#[ignore = "needs the data and render layers, and the network"]
fn export_writes_every_format() {
    let mut server = Server::start();
    let id = server.create_westermarkt();
    server.call_ok("fetch_site", json!({ "id": id }));
    let out = TempDir::new().unwrap();

    for format in ["svg", "pdf", "png", "dxf"] {
        let path = out.path().join(format!("westermarkt.{format}"));
        server.call_ok(
            "export",
            json!({ "id": id, "format": format, "path": path }),
        );
        assert!(path.metadata().unwrap().len() > 0, "{format} is empty");
    }
}
