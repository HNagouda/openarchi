//! Protocol tests for the section and viewpoint tools.

use serde_json::json;

mod common;

use common::{fetched_westermarkt, Server};

#[test]
fn a_section_line_is_drawn_flipped_shown_and_cleared_as_in_the_app() {
    let mut server = Server::start();
    let id = fetched_westermarkt(&mut server);

    // Like the greyed-out Section button, the section view waits for a line.
    let message = server.call_err(
        "set_look",
        json!({ "id": id, "view": { "mode": "section" } }),
    );
    assert!(message.contains("set_section"), "{message}");

    let drawn = server.call_ok(
        "set_section",
        json!({
            "id": id,
            "from": { "lon": 4.8825, "lat": 52.3745 },
            "to": { "lon": 4.8855, "lat": 52.3745 },
        }),
    );
    assert_eq!(drawn["section"]["depth_m"], json!(60.0));
    let from_lon = drawn["section"]["from"]["lon"].as_f64().unwrap();
    assert!((from_lon - 4.8825).abs() < 1e-6, "{drawn}");

    let flipped = server.call_ok(
        "set_section",
        json!({ "id": id, "flip": true, "depth_m": 80 }),
    );
    assert_eq!(flipped["section"]["from"], drawn["section"]["to"]);
    assert_eq!(flipped["section"]["depth_m"], json!(80.0));
    assert_eq!(flipped["section"]["scale"], json!(null));

    // Like the scale and height menus beside the depth field.
    let scaled = server.call_ok(
        "set_section",
        json!({ "id": id, "scale": 500, "exaggeration": 2 }),
    );
    assert_eq!(scaled["section"]["scale"], json!(500));
    assert_eq!(scaled["section"]["exaggeration"], json!(2));
    let odd = server.call_err("set_section", json!({ "id": id, "scale": 300 }));
    assert!(odd.contains("scale must be one of"), "{odd}");
    let fitted = server.call_ok(
        "set_section",
        json!({ "id": id, "scale": 0, "exaggeration": 1 }),
    );
    assert_eq!(fitted["section"]["scale"], json!(null));
    assert_eq!(fitted["section"]["exaggeration"], json!(1));

    let shown = server.call_ok(
        "set_look",
        json!({ "id": id, "view": { "mode": "section" } }),
    );
    assert_eq!(shown["view"]["mode"], json!("section"));
    let sheets = server.call_ok("list_sheets", json!({ "id": id }));
    assert_eq!(sheets["sheets"][0]["section"], flipped["section"]);

    // Removing the line takes the sheet back to the plan it can still draw.
    let cleared = server.call_ok("set_section", json!({ "id": id, "clear": true }));
    assert_eq!(cleared, json!({ "section": null, "view": "plan" }));
}

#[test]
fn a_section_line_too_short_or_off_the_site_is_refused() {
    let mut server = Server::start();
    let id = fetched_westermarkt(&mut server);
    let short = server.call_err(
        "set_section",
        json!({ "id": id, "from": { "lon": 4.8840, "lat": 52.3745 }, "to": { "lon": 4.88401, "lat": 52.3745 } }),
    );
    assert!(short.contains("at least 5 m"), "{short}");
    let far = server.call_err(
        "set_section",
        json!({ "id": id, "from": { "lon": 4.95, "lat": 52.37 }, "to": { "lon": 4.96, "lat": 52.37 } }),
    );
    assert!(far.contains("inside the area the sheet draws"), "{far}");
    let half = server.call_err(
        "set_section",
        json!({ "id": id, "to": { "lon": 4.8850, "lat": 52.3745 } }),
    );
    assert!(half.contains("both from and to"), "{half}");
}

#[test]
fn the_view_point_stands_at_a_place_on_a_pin_or_back_at_the_centre() {
    let mut server = Server::start();
    let id = fetched_westermarkt(&mut server);

    let placed = server.call_ok(
        "set_view_point",
        json!({ "id": id, "at": { "lon": 4.8830, "lat": 52.3745 } }),
    );
    let lon = placed["view_point"]["at"]["lon"].as_f64().unwrap();
    assert!((lon - 4.8830).abs() < 1e-6, "{placed}");

    let pin = server.call_ok(
        "add_pin",
        json!({ "id": id, "location": { "lon": 4.8845, "lat": 52.3740 }, "icon": "dot" }),
    );
    let pin_id = pin["pins"][0]["id"].as_str().unwrap().to_string();
    let on_pin = server.call_ok("set_view_point", json!({ "id": id, "pin": pin_id }));
    let pin_lat = on_pin["view_point"]["at"]["lat"].as_f64().unwrap();
    assert!((pin_lat - 52.3740).abs() < 1e-6, "{on_pin}");
    let sheets = server.call_ok("list_sheets", json!({ "id": id }));
    assert_eq!(sheets["sheets"][0]["view_point"], on_pin["view_point"]);

    let centre = server.call_ok("set_view_point", json!({ "id": id, "centre": true }));
    assert_eq!(centre["view_point"], json!(null));
}

#[test]
fn an_eye_on_a_pin_follows_it_and_goes_back_to_the_centre_when_it_is_removed() {
    let mut server = Server::start();
    let id = fetched_westermarkt(&mut server);
    let pin = server.call_ok(
        "add_pin",
        json!({ "id": id, "location": { "lon": 4.8845, "lat": 52.3740 }, "icon": "dot" }),
    );
    let pin_id = pin["pins"][0]["id"].as_str().unwrap().to_string();
    let on_pin = server.call_ok("set_view_point", json!({ "id": id, "pin": pin_id }));
    assert_eq!(on_pin["view_point"]["pin"], json!(pin_id));

    server.call_ok(
        "move_pin",
        json!({ "id": id, "pin_id": pin_id, "location": { "lon": 4.8835, "lat": 52.3742 } }),
    );
    let moved = server.call_ok("list_sheets", json!({ "id": id }));
    let lon = moved["sheets"][0]["view_point"]["at"]["lon"]
        .as_f64()
        .unwrap();
    assert!((lon - 4.8835).abs() < 1e-6, "{moved}");

    server.call_ok("remove_pin", json!({ "id": id, "pin_id": pin_id }));
    let removed = server.call_ok("list_sheets", json!({ "id": id }));
    assert_eq!(removed["sheets"][0]["view_point"], json!(null));
}

#[test]
fn a_view_point_off_the_site_or_asked_two_ways_is_refused() {
    let mut server = Server::start();
    let id = fetched_westermarkt(&mut server);

    let far = server.call_err(
        "set_view_point",
        json!({ "id": id, "at": { "lon": 4.95, "lat": 52.37 } }),
    );
    assert!(far.contains("inside the area the sheet draws"), "{far}");
    let both = server.call_err(
        "set_view_point",
        json!({ "id": id, "at": { "lon": 4.8830, "lat": 52.3745 }, "centre": true }),
    );
    assert!(both.contains("exactly one"), "{both}");
    let missing = server.call_err("set_view_point", json!({ "id": id, "pin": "pin-9" }));
    assert!(missing.contains("no pin"), "{missing}");
}
