//! Protocol tests for the sheet tab and pin tools.

use openarchi_core::library::Library;
use openarchi_core::project::Look;
use serde_json::json;

mod common;

use common::{fetched_westermarkt, Server};

#[test]
fn a_sheet_opened_from_a_highlight_zooms_in_and_its_ratios_describe_that_area() {
    let mut server = Server::start();
    let id = fetched_westermarkt(&mut server);

    let added = server.open_circle_as_sheet(&id, "Westerkerk", 60.0);

    let sheets = added["sheets"].as_array().unwrap();
    assert_eq!(sheets.len(), 2);
    assert_eq!(sheets[0]["name"], json!("Sheet 1"));
    assert_eq!(sheets[1]["name"], json!("Westerkerk"));
    assert!(sheets[1]["scale"].as_u64().unwrap() < sheets[0]["scale"].as_u64().unwrap());
    assert_eq!(server.call_ok("list_sheets", json!({ "id": id })), added);

    let whole = server.call_ok("get_ratios", json!({ "id": id }));
    let part = server.call_ok("get_ratios", json!({ "id": id, "tab": 2 }));
    let circle = std::f64::consts::PI * 60.0 * 60.0;
    let area = part["site_area_m2"].as_f64().unwrap();
    assert!((area - circle).abs() / circle < 0.01, "{area}");
    assert!(part["building_count"].as_u64() < whole["building_count"].as_u64());

    let styled = server.call_ok(
        "set_look",
        json!({ "id": id, "tab": 2, "diagram_id": "figure-ground" }),
    );
    assert_eq!(
        styled["name"],
        json!("Westerkerk"),
        "set_look keeps the tab's name"
    );
    let message = server.call_err("set_look", json!({ "id": id, "tab": 3 }));
    assert!(message.contains("1 to 2"), "{message}");
}

#[test]
fn add_sheet_names_a_new_tab_after_the_first_free_number() {
    let mut server = Server::start();
    let id = fetched_westermarkt(&mut server);
    // As after deleting the second of three tabs in the app.
    let library = Library::at(server.library.path()).unwrap();
    let mut project = library.load(&id).unwrap();
    let third = Look {
        name: "Sheet 3".to_string(),
        ..project.look.clone()
    };
    project.more_sheets.push(third);
    library.save(&mut project).unwrap();

    let added = server.call_ok("add_sheet", json!({ "id": id }));

    assert_eq!(added["sheets"][2]["name"], json!("Sheet 4"));
}

#[test]
fn add_sheet_shows_the_whole_site_and_refuses_an_area_of_its_own() {
    let mut server = Server::start();
    let id = fetched_westermarkt(&mut server);

    let added = server.call_ok("add_sheet", json!({ "id": id, "name": "Context" }));
    assert_eq!(added["sheets"][1]["selection"], json!(null));

    // Like the app, an assistant gets a sheet of part of the site only from a highlight.
    let message = server.call_err(
        "add_sheet",
        json!({
            "id": id,
            "selection": { "kind": "circle", "center": { "lon": 4.8840, "lat": 52.3745 }, "radius_m": 50.0 },
        }),
    );
    assert!(message.contains("selection"), "{message}");
}

#[test]
fn pins_are_added_moved_relabelled_and_removed() {
    let mut server = Server::start();
    let id = fetched_westermarkt(&mut server);

    let added = server.call_ok(
        "add_pin",
        json!({
            "id": id,
            "location": { "lon": 4.8840, "lat": 52.3745 },
            "label": "Westerkerk",
            "icon": "star",
            "size": "large",
        }),
    );
    assert_eq!(added["pins"][0]["id"], json!("pin-1"));
    assert_eq!(added["pins"][0]["icon"], json!("star"));
    assert_eq!(added["pins"][0]["size"], json!("large"));
    server.call_ok(
        "add_pin",
        json!({ "id": id, "location": { "lon": 4.8830, "lat": 52.3750 }, "icon": "dot" }),
    );
    // The app no longer offers a numbered disc, so neither does the assistant.
    let refused = server.call_err(
        "add_pin",
        json!({ "id": id, "location": { "lon": 4.8830, "lat": 52.3750 }, "icon": "number" }),
    );
    assert!(refused.contains("number"), "{refused}");

    let moved = server.call_ok(
        "move_pin",
        json!({
            "id": id,
            "pin_id": "pin-1",
            "location": { "lon": 4.8845, "lat": 52.3748 },
            "label": "Tower",
        }),
    );
    let first = &moved["pins"][0];
    assert_eq!(first["label"], json!("Tower"));
    assert_eq!(first["size"], json!("large"), "a move keeps the size");
    let resized = server.call_ok(
        "move_pin",
        json!({ "id": id, "pin_id": "pin-1", "size": "extra_large" }),
    );
    assert_eq!(resized["pins"][0]["size"], json!("extra_large"));
    assert!((first["location"]["lon"].as_f64().unwrap() - 4.8845).abs() < 1e-6);
    assert_eq!(moved["pins"][1]["icon"], json!("dot"));

    let left = server.call_ok("remove_pin", json!({ "id": id, "pin_id": "pin-1" }));
    assert_eq!(left["pins"].as_array().unwrap().len(), 1);
    assert_eq!(server.call_ok("list_pins", json!({ "id": id })), left);

    let message = server.call_err("remove_pin", json!({ "id": id, "pin_id": "pin-9" }));
    assert!(message.contains("pin-2"), "{message}");
}

#[test]
fn a_pin_outside_the_site_or_before_a_fetch_is_refused() {
    let mut server = Server::start();
    let unfetched = server.create_westermarkt();
    let message = server.call_err(
        "add_pin",
        json!({ "id": unfetched, "location": { "lon": 4.8840, "lat": 52.3745 } }),
    );
    assert!(message.contains("fetch_site"), "{message}");

    let id = fetched_westermarkt(&mut server);
    let message = server.call_err(
        "add_pin",
        json!({ "id": id, "location": { "lon": 4.95, "lat": 52.37 } }),
    );
    assert!(message.contains("outside the fetched site"), "{message}");
}
