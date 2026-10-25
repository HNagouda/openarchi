//! Protocol tests for the Wind diagram: its season, and that it stays in plan
//! the way the app keeps it there.

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
fn the_wind_refuses_tilted_views_and_plates() {
    let mut server = Server::start();
    let id = server.create_westermarkt();

    let tilted = server.call_err(
        "set_look",
        json!({ "id": id, "diagram_id": "wind", "view": { "mode": "axonometric" } }),
    );
    assert!(tilted.contains("plan only"), "{tilted}");
    let plate = server.call_err(
        "set_look",
        json!({ "id": id, "view": { "mode": "explode", "plates": ["context", "wind"] } }),
    );
    assert!(plate.contains("plan only"), "{plate}");
}

#[test]
fn choosing_the_wind_on_a_tilted_sheet_turns_it_to_its_plan() {
    let mut server = Server::start();
    let id = server.create_westermarkt();
    server.call_ok(
        "set_look",
        json!({ "id": id, "view": { "mode": "axonometric" } }),
    );

    let look = server.call_ok("set_look", json!({ "id": id, "diagram_id": "wind" }));
    assert_eq!(look["view"]["mode"], json!("plan"));
}
