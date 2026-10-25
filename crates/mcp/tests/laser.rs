//! Protocol tests for the laser cutting tool.

use openarchi_core::library::Library;
use openarchi_core::scene::Scene;
use serde_json::json;

mod common;

use common::{fetched_westermarkt, Server, WESTERMARKT_SCENE};

#[test]
fn export_laser_checks_then_writes_one_svg_per_bed_for_a_sheet_area() {
    let mut server = Server::start();
    let id = server.create_westermarkt();
    let library = Library::at(server.library.path()).unwrap();
    let mut project = library.load(&id).unwrap();
    let scene: Scene =
        serde_json::from_str(&std::fs::read_to_string(WESTERMARKT_SCENE).unwrap()).unwrap();
    project.scene = Some(scene);
    library.save(&mut project).unwrap();
    // A second tab framing a small block, so the model is quick to make.
    server.open_circle_as_sheet(&id, "Block", 50.0);
    let folder = server.library.path().join("laser");
    std::fs::create_dir(&folder).unwrap();

    // Only checking needs no folder, as the app's dialog checks before asking for one.
    let checked = server.call_ok(
        "export_laser",
        json!({ "id": id, "tab": 2, "scale": 500, "check_only": true }),
    );
    let sheets = checked["summary"]["sheet_count"].as_u64().unwrap();
    assert!(sheets >= 1, "{checked}");
    assert_eq!(checked["summary"]["plate_tiles"], json!(1));
    assert!(
        std::fs::read_dir(&folder).unwrap().next().is_none(),
        "check_only writes nothing"
    );

    let written = server.call_ok(
        "export_laser",
        json!({ "id": id, "tab": 2, "folder": folder, "scale": 500 }),
    );
    let files = written["files"].as_array().unwrap();
    assert_eq!(files.len() as u64, sheets);
    let first = std::fs::read_to_string(files[0].as_str().unwrap()).unwrap();
    assert!(first.contains("stroke=\"#FF0000\"") && first.contains("stroke=\"#0000FF\""));
    assert!(
        files[0].as_str().unwrap().ends_with("-laser-1.svg"),
        "{written}"
    );

    let again = server.call_err(
        "export_laser",
        json!({ "id": id, "tab": 2, "folder": folder, "scale": 500 }),
    );
    assert!(again.contains("overwrite"), "{again}");
    let tiny = server.call_err(
        "export_laser",
        json!({ "id": id, "folder": folder, "thickness_mm": 0.1, "overwrite": true }),
    );
    assert!(tiny.contains("thick"), "{tiny}");
    let relative = server.call_err("export_laser", json!({ "id": id, "folder": "laser" }));
    assert!(relative.contains("absolute"), "{relative}");
}

#[test]
fn export_laser_needs_a_folder_once_it_writes() {
    let mut server = Server::start();
    let id = fetched_westermarkt(&mut server);
    server.open_circle_as_sheet(&id, "Block", 30.0);

    let message = server.call_err("export_laser", json!({ "id": id, "tab": 2, "scale": 500 }));
    assert!(message.contains("folder is needed"), "{message}");
}
