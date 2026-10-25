//! Protocol tests for the model tools and the model in get_project.

use serde_json::{json, Value};

mod common;

use common::{fetched_westermarkt, Server};

fn fixture(name: &str) -> String {
    format!(
        "{}/../core/tests/fixtures/models/{name}",
        env!("CARGO_MANIFEST_DIR")
    )
}

fn size(model: &Value) -> Vec<f64> {
    model["size_m"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| (value.as_f64().unwrap() * 100.0).round() / 100.0)
        .collect()
}

#[test]
fn a_model_is_imported_placed_and_shown_in_the_project() {
    let mut server = Server::start();
    let id = fetched_westermarkt(&mut server);

    let imported = server.call_ok(
        "import_model",
        json!({ "id": id, "path": fixture("gable-house.glb") }),
    );
    let model = &imported["model"];
    assert_eq!(size(model), vec![8.0, 10.0, 9.0]);
    assert_eq!(model["unit"], json!("metre"));
    assert_eq!(model["up"], json!("y"));
    assert_eq!(model["placement"]["rotation_deg"], json!(0.0));
    assert!(model["covered"].is_array(), "{model}");

    let start = model["placement"].clone();
    let placed = server.call_ok(
        "place_model",
        json!({ "id": id, "rd": [start["x"].as_f64().unwrap() + 12.0, start["y"].as_f64().unwrap() + 62.0], "rotation_deg": 380.0 }),
    );
    let placement = &placed["model"]["placement"];
    assert_eq!(placement["rotation_deg"], json!(20.0));
    let moved = placement["x"].as_f64().unwrap() - start["x"].as_f64().unwrap();
    assert!((moved - 12.0).abs() < 1e-6, "{placement}");
    // Placing picks the replaced buildings by the app's rule: more than half covered.
    for id in placed["model"]["replaced"].as_array().unwrap() {
        let covered = placed["model"]["covered"]
            .as_array()
            .unwrap()
            .iter()
            .find(|building| &building["id"] == id)
            .unwrap();
        assert!(covered["share"].as_f64().unwrap() > 0.5, "{covered}");
    }

    let project = server.call_ok("get_project", json!({ "id": id }));
    assert_eq!(project["model"]["placement"], *placement);
    let preview = server.call("render_preview", json!({ "id": id, "width_px": 400 }));
    assert_eq!(preview["isError"], json!(false), "{preview}");
}

#[test]
fn a_model_read_in_another_unit_keeps_its_place() {
    let mut server = Server::start();
    let id = fetched_westermarkt(&mut server);
    let imported = server.call_ok(
        "import_model",
        json!({ "id": id, "path": fixture("gable-house-mm.obj") }),
    );
    assert_eq!(imported["model"]["unit"], json!("millimetre"));

    let reread = server.call_ok(
        "set_model_units",
        json!({ "id": id, "unit": "centimetre", "up": "z" }),
    );

    assert_eq!(size(&reread["model"]), vec![80.0, 100.0, 90.0]);
    assert_eq!(reread["model"]["placement"], imported["model"]["placement"]);
}

#[test]
fn only_a_building_under_the_model_can_be_replaced() {
    let mut server = Server::start();
    let id = fetched_westermarkt(&mut server);
    let imported = server.call_ok(
        "import_model",
        json!({ "id": id, "path": fixture("l-block-y-up.obj"), "up": "y" }),
    );
    let covered = imported["model"]["covered"].as_array().unwrap().clone();
    assert!(!covered.is_empty(), "{imported}");

    let stray = server.call_err(
        "set_replaced_buildings",
        json!({ "id": id, "replaced": ["0000000000000000"] }),
    );
    assert!(stray.contains("not under the model"), "{stray}");

    let chosen = server.call_ok(
        "set_replaced_buildings",
        json!({ "id": id, "replaced": [covered[0]["id"]] }),
    );
    assert_eq!(chosen["model"]["replaced"], json!([covered[0]["id"]]));
}

#[test]
fn a_removed_model_is_gone_and_ifc_is_refused_with_the_way_round() {
    let mut server = Server::start();
    let id = fetched_westermarkt(&mut server);
    server.call_ok(
        "import_model",
        json!({ "id": id, "path": fixture("gable-house.glb") }),
    );

    server.call_ok("remove_model", json!({ "id": id }));

    let project = server.call_ok("get_project", json!({ "id": id }));
    assert_eq!(project["model"], Value::Null);
    let again = server.call_err("remove_model", json!({ "id": id }));
    assert!(again.contains("import_model"), "{again}");
    let ifc = server.call_err(
        "import_model",
        json!({ "id": id, "path": "/tmp/house.ifc" }),
    );
    assert!(ifc.contains("Save As"), "{ifc}");
}
