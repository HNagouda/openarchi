//! Protocol tests for the highlight tools and show_on_sheet.

use serde_json::{json, Value};

mod common;

use common::{fetched_westermarkt, Server};

/// A 30 by 20 m plot just south-west of the Westerkerk, in degrees.
fn plot_shape() -> Value {
    json!({
        "kind": "rectangle",
        "south_west": { "lon": 4.8836, "lat": 52.3743 },
        "north_east": { "lon": 4.88404, "lat": 52.37448 },
    })
}

#[test]
fn highlights_are_added_edited_listed_and_removed() {
    let mut server = Server::start();
    let id = fetched_westermarkt(&mut server);

    let added = server.call_ok(
        "add_highlight",
        json!({ "id": id, "shape": plot_shape(), "label": "Plot", "shows_area": true }),
    );
    let plot = &added["highlights"][0];
    assert_eq!(plot["id"], json!("highlight-1"));
    assert_eq!(plot["outline"], json!("solid"));
    let area = plot["area_m2"].as_f64().unwrap();
    assert!((500.0..700.0).contains(&area), "{area}");
    let south_west = plot["shape"]["south_west"]["lon"].as_f64().unwrap();
    assert!((south_west - 4.8836).abs() < 1e-5, "{plot}");

    server.call_ok(
        "add_highlight",
        json!({
            "id": id,
            "shape": { "kind": "circle", "center": { "lon": 4.8845, "lat": 52.3748 }, "radius_m": 12.0 },
        }),
    );
    let edited = server.call_ok(
        "edit_highlight",
        json!({ "id": id, "highlight_id": "highlight-2", "label": "New house", "outline": "dotted", "colour": "#1F7A8C", "dims_rest": true }),
    );
    let house = &edited["highlights"][1];
    assert_eq!(house["label"], json!("New house"));
    assert_eq!(house["outline"], json!("dotted"));
    assert_eq!(house["colour"], json!("#1F7A8C"));
    assert_eq!(edited["highlights"][0]["colour"], json!("@accent"));
    let odd = server.call_err(
        "edit_highlight",
        json!({ "id": id, "highlight_id": "highlight-2", "colour": "teal" }),
    );
    assert!(odd.contains("#RRGGBB"), "{odd}");
    assert_eq!(house["dims_rest"], json!(true));
    assert_eq!(house["shape"]["kind"], json!("circle"));
    assert_eq!(
        server.call_ok("list_highlights", json!({ "id": id })),
        edited
    );

    // The preview draws the highlights, one dimming the rest, without an error.
    let preview = server.call("render_preview", json!({ "id": id, "width_px": 400 }));
    assert_eq!(preview["isError"], json!(false), "{preview}");
    assert_eq!(preview["content"][0]["type"], json!("image"));

    let left = server.call_ok(
        "remove_highlight",
        json!({ "id": id, "highlight_id": "highlight-1" }),
    );
    assert_eq!(left["highlights"].as_array().unwrap().len(), 1);
    let message = server.call_err(
        "remove_highlight",
        json!({ "id": id, "highlight_id": "highlight-9" }),
    );
    assert!(message.contains("highlight-2"), "{message}");
}

#[test]
fn a_highlight_outside_the_site_too_small_or_before_a_fetch_is_refused() {
    let mut server = Server::start();
    let unfetched = server.create_westermarkt();
    let message = server.call_err(
        "add_highlight",
        json!({ "id": unfetched, "shape": plot_shape() }),
    );
    assert!(message.contains("fetch_site"), "{message}");

    let id = fetched_westermarkt(&mut server);
    let far = server.call_err(
        "add_highlight",
        json!({
            "id": id,
            "shape": { "kind": "circle", "center": { "lon": 4.95, "lat": 52.37 }, "radius_m": 20.0 },
        }),
    );
    assert!(far.contains("outside the fetched site"), "{far}");
    let tiny = server.call_err(
        "add_highlight",
        json!({
            "id": id,
            "shape": { "kind": "circle", "center": { "lon": 4.8840, "lat": 52.3745 }, "radius_m": 0.5 },
        }),
    );
    assert!(tiny.contains("at least 4 m²"), "{tiny}");
}

#[test]
fn a_highlight_opens_as_a_sheet_whose_ratios_describe_it() {
    let mut server = Server::start();
    let id = fetched_westermarkt(&mut server);
    let added = server.call_ok(
        "add_highlight",
        json!({ "id": id, "shape": plot_shape(), "label": "Plot" }),
    );
    let area = added["highlights"][0]["area_m2"].as_f64().unwrap();

    let opened = server.call_ok(
        "open_highlight_as_sheet",
        json!({ "id": id, "highlight_id": "highlight-1" }),
    );

    let sheets = opened["sheets"].as_array().unwrap();
    assert_eq!(sheets.len(), 2);
    assert_eq!(sheets[1]["name"], json!("Plot"));
    let part = server.call_ok("get_ratios", json!({ "id": id, "tab": 2 }));
    let measured = part["site_area_m2"].as_f64().unwrap();
    assert!(
        (measured - area).abs() / area < 0.02,
        "{measured} vs {area}"
    );
}

#[test]
fn show_on_sheet_hides_and_shows_pins_and_highlights_per_tab() {
    let mut server = Server::start();
    let id = fetched_westermarkt(&mut server);

    let hidden = server.call_ok("show_on_sheet", json!({ "id": id, "highlights": false }));
    assert_eq!(hidden["hide_highlights"], json!(true));
    assert_eq!(hidden["hide_pins"], json!(false));

    let shown = server.call_ok(
        "show_on_sheet",
        json!({ "id": id, "highlights": true, "pins": false }),
    );
    assert_eq!(shown["hide_highlights"], json!(false));
    assert_eq!(shown["hide_pins"], json!(true));
}
