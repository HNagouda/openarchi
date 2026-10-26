//! Protocol tests for the project tools: the handshake, the tool list, and
//! creating, reading, deleting and restoring projects.

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

use serde_json::{json, Value};
use tempfile::TempDir;

mod common;

use common::{Server, PROTOCOL_VERSION};

/// Every tool the server must offer, in one place.
const TOOLS: [&str; 49] = [
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
    "export_laser",
    "list_reports",
    "export_report",
    "create_theme",
    "import_palette",
    "list_user_palettes",
    "delete_project",
    "restore_project",
    "list_sheets",
    "add_sheet",
    "list_pins",
    "add_pin",
    "move_pin",
    "remove_pin",
    "set_section",
    "set_view_point",
    "set_wind_season",
    "list_highlights",
    "add_highlight",
    "edit_highlight",
    "remove_highlight",
    "open_highlight_as_sheet",
    "show_on_sheet",
    "import_model",
    "place_model",
    "set_model_units",
    "set_replaced",
    "remove_model",
    "pick_on_site",
    "list_site_edits",
    "remove_from_site",
    "restore_on_site",
    "move_tree",
    "plant_tree",
];

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
#[ignore = "needs the network (PDOK)"]
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
#[ignore = "needs the network (PDOK)"]
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
