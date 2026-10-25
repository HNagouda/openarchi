//! Protocol tests for the style tools: set_look, edit_text, the diagram and
//! theme lists, create_theme and the palettes.

use serde_json::{json, Value};

mod common;

use common::Server;

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
fn set_look_fades_the_neighbourhood_and_refuses_an_opacity_past_one() {
    let mut server = Server::start();
    let id = server.create_westermarkt();

    let faded = server.call_ok(
        "set_look",
        json!({ "id": id, "sheet": { "context_opacity": 0.4 } }),
    );
    let refused = server.call(
        "set_look",
        json!({ "id": id, "sheet": { "context_opacity": 40 } }),
    );

    assert_eq!(faded["sheet"]["context_opacity"], json!(0.4));
    assert_eq!(refused["isError"], json!(true));
}

#[test]
fn set_look_turns_the_neighbourhood_off_and_on() {
    let mut server = Server::start();
    let id = server.create_westermarkt();

    let off = server.call_ok(
        "set_look",
        json!({ "id": id, "sheet": { "show_context": false } }),
    );
    let on = server.call_ok(
        "set_look",
        json!({ "id": id, "sheet": { "show_context": true } }),
    );

    assert_eq!(off["sheet"]["show_context"], json!(false));
    assert_eq!(on["sheet"]["show_context"], json!(true));
    assert_eq!(on["sheet"]["show_legend"], json!(true));
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
fn import_palette_saves_a_theme_set_look_can_use_and_lists_it() {
    let mut server = Server::start();
    let imported = server.call_ok(
        "import_palette",
        json!({
            "text": "https://palettes.example/f79f79-f7d08a-e3f09b-87b6a7-5b5941",
            "name": "Sea glass"
        }),
    );
    assert_eq!(imported["id"], json!("sea-glass"));
    assert_eq!(imported["palette"]["ink"], json!("#5B5941"));
    assert_eq!(imported["roles"].as_array().unwrap().len(), 6);
    assert!(server.library.path().join("themes/sea-glass.json").exists());

    let id = server.create_westermarkt();
    server.call_ok("set_look", json!({ "id": id, "theme_id": "sea-glass" }));

    let listed = server.call_ok("list_user_palettes", json!({}));
    let palettes = listed["palettes"].as_array().unwrap();
    assert_eq!(palettes.len(), 1);
    assert_eq!(palettes[0]["swatches"].as_array().unwrap().len(), 5);
}

#[test]
fn import_palette_without_saving_only_checks_and_junk_names_the_problem() {
    let mut server = Server::start();
    let checked = server.call_ok(
        "import_palette",
        json!({ "text": "f7d08a, f79f79", "save": false }),
    );
    assert_eq!(checked["id"], json!(null));
    assert!(checked["warnings"][0].as_str().unwrap().contains("4.5:1"));
    assert!(!server
        .library
        .path()
        .join("themes/imported-palette.json")
        .exists());

    // Saving the same unreadable colours is refused, as the app's editor refuses it.
    let refused = server.call_err("import_palette", json!({ "text": "f7d08a, f79f79" }));
    assert!(refused.contains("hard to read"), "{refused}");
    assert!(!server
        .library
        .path()
        .join("themes/imported-palette.json")
        .exists());

    let message = server.call_err("import_palette", json!({ "text": "hello there" }));
    assert!(
        message.contains("\"hello\" is not a hex colour"),
        "{message}"
    );
}
