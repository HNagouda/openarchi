//! Protocol tests for the Wind diagram: its season, and that it stays flat
//! the way the app keeps it: in plan or on an explode plate.

use serde_json::json;

mod common;

use common::{fetched_westermarkt, Server};

#[test]
fn the_wind_season_is_saved_with_the_sheet() {
    let mut server = Server::start();
    let id = fetched_westermarkt(&mut server);

    let winter = server.call_ok("set_wind_season", json!({ "id": id, "season": "winter" }));
    assert_eq!(winter["wind_season"], json!("winter"));
    let look = server.call_ok("set_look", json!({ "id": id, "diagram_id": "wind" }));
    assert_eq!(look["wind_season"], json!("winter"));

    let year = server.call_ok("set_wind_season", json!({ "id": id, "season": "year" }));
    assert_eq!(year["wind_season"], json!("year"));
    let bad = server.call_err("set_wind_season", json!({ "id": id, "season": "monsoon" }));
    assert!(bad.contains("monsoon"), "{bad}");
}

#[test]
fn the_wind_refuses_the_axonometric_view() {
    let mut server = Server::start();
    let id = server.create_westermarkt();

    let tilted = server.call_err(
        "set_look",
        json!({ "id": id, "diagram_id": "wind", "view": { "mode": "axonometric" } }),
    );
    assert!(tilted.contains("plan only"), "{tilted}");
}

#[test]
fn the_wind_can_be_an_explode_plate() {
    let mut server = Server::start();
    let id = server.create_westermarkt();

    let look = server.call_ok(
        "set_look",
        json!({ "id": id, "view": { "mode": "explode", "plates": ["context", "wind"] } }),
    );
    assert_eq!(look["view"]["plates"], json!(["context", "wind"]));
    let own = server.call_ok("set_look", json!({ "id": id, "diagram_id": "wind" }));
    assert_eq!(own["view"]["mode"], json!("explode"));
}

#[test]
fn choosing_the_wind_on_an_axonometric_sheet_turns_it_to_its_plan() {
    let mut server = Server::start();
    let id = server.create_westermarkt();
    server.call_ok(
        "set_look",
        json!({ "id": id, "view": { "mode": "axonometric" } }),
    );

    let look = server.call_ok("set_look", json!({ "id": id, "diagram_id": "wind" }));
    assert_eq!(look["view"]["mode"], json!("plan"));
}
