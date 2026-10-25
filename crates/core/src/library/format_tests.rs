use std::fs;

use serde_json::Value;

use super::tests::{library, westermarkt};
use super::*;
use crate::scene::Scene;

#[test]
fn newer_format_version_is_refused_with_a_reason() {
    let (folder, library) = library();
    let project = library.create("Westermarkt", westermarkt()).unwrap();
    let path = folder
        .path()
        .join(format!("projects/{}.openarchi", project.id));
    let mut value: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    value["format_version"] = Value::from(PROJECT_FORMAT_VERSION + 1);
    fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();

    let error = library.load(&project.id).unwrap_err().to_string();
    assert!(error.contains("newer openarchi"), "{error}");
}

/// A project exactly as openarchi 0.1.0 wrote it, before sheet text could be styled or moved.
const VERSION_1_PROJECT: &str = r#"{
  "format_version": 1,
  "id": "2026-10-01-westermarkt-3f9a",
  "name": "Westermarkt",
  "created": "2026-10-01T09:00:00Z",
  "modified": "2026-10-01T09:30:00Z",
  "area": { "kind": "circle", "center": { "lon": 4.8837, "lat": 52.3747 }, "radius_m": 150.0 },
  "scene": null,
  "look": {
    "diagram_id": "building-age",
    "theme_id": "ink",
    "sheet": {
      "paper": "a2", "landscape": false, "scale": 2000, "frame": "square",
      "title": "Westermarkt", "subtitle": "Amsterdam",
      "show_legend": true, "show_north_arrow": false, "show_scale_bar": true, "show_credits": true
    },
    "overrides": {}
  }
}"#;

#[test]
fn version_1_project_opens_with_its_title_in_the_automatic_layout() {
    let (folder, library) = library();
    let id = "2026-10-01-westermarkt-3f9a";
    fs::write(
        folder.path().join(format!("projects/{id}.openarchi")),
        VERSION_1_PROJECT,
    )
    .unwrap();

    let project = library.load(id).unwrap();

    let sheet = &project.look.sheet;
    assert_eq!(sheet.title, "Westermarkt");
    assert_eq!(sheet.paper, crate::sheet::Paper::A2);
    assert_eq!(sheet.title_position, None);
    assert_eq!(sheet.title_style, crate::sheet::TextStyle::default());
    assert!(sheet.texts.is_empty());
}

#[test]
fn version_1_project_is_saved_back_as_the_current_version() {
    let (folder, library) = library();
    let id = "2026-10-01-westermarkt-3f9a";
    let path = folder.path().join(format!("projects/{id}.openarchi"));
    fs::write(&path, VERSION_1_PROJECT).unwrap();
    let mut project = library.load(id).unwrap();

    library.save(&mut project).unwrap();

    let saved: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    assert_eq!(saved["format_version"], Value::from(PROJECT_FORMAT_VERSION));
    assert_eq!(saved["look"]["sheet"]["texts"], serde_json::json!([]));
}

#[test]
fn text_boxes_and_moved_titles_survive_a_save_and_load() {
    use crate::sheet::{Align, Position, TextBox, TextStyle};

    let (_folder, library) = library();
    let mut project = library.create("Westermarkt", westermarkt()).unwrap();
    project.look.sheet.title_position = Some(Position {
        x_mm: 20.5,
        y_mm: 18.0,
    });
    project.look.sheet.title_style.italic = Some(true);
    project.look.sheet.texts.push(TextBox {
        id: "text-1".to_string(),
        text: "Westerkerk\n1631".to_string(),
        style: TextStyle {
            family: Some("IBM Plex Mono".to_string()),
            bold: Some(true),
            size_pt: Some(18.0),
            align: Some(Align::Right),
            colour: Some("@accent".to_string()),
            opacity: Some(0.8),
            ..TextStyle::default()
        },
        position: Some(Position {
            x_mm: 200.0,
            y_mm: 120.0,
        }),
    });
    library.save(&mut project).unwrap();

    let loaded = library.load(&project.id).unwrap();

    assert_eq!(loaded.look, project.look);
}

/// The Westermarkt scene the render tests use, about 7 MB of JSON.
fn fixture_scene() -> Scene {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/westermarkt.scene.json"
    );
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

#[test]
fn a_saved_scene_is_stored_packed_and_loads_back_whole() {
    let (folder, library) = library();
    let mut project = library.create("Westermarkt", westermarkt()).unwrap();
    let scene = fixture_scene();
    let feature_count = scene.features.len();
    project.scene = Some(scene);
    library.save(&mut project).unwrap();

    let path = folder
        .path()
        .join(format!("projects/{}.openarchi", project.id));
    let value: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let loaded = library.load(&project.id).unwrap();

    assert!(value["scene"].is_string());
    assert_eq!(loaded.scene.unwrap().features.len(), feature_count);
    assert!(library.list().unwrap()[0].has_scene);
}

#[test]
fn a_version_1_project_with_a_plain_scene_still_opens() {
    let (folder, library) = library();
    let project = library.create("Westermarkt", westermarkt()).unwrap();
    let path = folder
        .path()
        .join(format!("projects/{}.openarchi", project.id));
    let mut value: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    value["format_version"] = Value::from(1);
    value["scene"] = serde_json::to_value(fixture_scene()).unwrap();
    fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();

    let loaded = library.load(&project.id).unwrap();

    assert_eq!(loaded.format_version, PROJECT_FORMAT_VERSION);
    assert_eq!(
        loaded.scene.unwrap().features.len(),
        fixture_scene().features.len()
    );
}

#[test]
fn a_damaged_packed_scene_is_refused_with_a_reason() {
    let (folder, library) = library();
    let project = library.create("Westermarkt", westermarkt()).unwrap();
    let path = folder
        .path()
        .join(format!("projects/{}.openarchi", project.id));
    let mut value: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    value["scene"] = Value::from("not base64 at all!");
    fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();

    let error = library.load(&project.id).unwrap_err().to_string();
    assert!(error.contains("stored scene is damaged"), "{error}");
}

#[test]
fn a_project_saved_on_a_folded_diagram_opens_on_its_variant() {
    let (folder, library) = library();
    let project = library.create("Westermarkt", westermarkt()).unwrap();
    let path = folder
        .path()
        .join(format!("projects/{}.openarchi", project.id));
    let mut value: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    value["look"]["diagram_id"] = Value::from("traffic-noise");
    value["look"].as_object_mut().unwrap().remove("variant");
    fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();

    let loaded = library.load(&project.id).unwrap();

    assert_eq!(loaded.look.diagram_id, "noise");
    assert_eq!(loaded.look.variant.as_deref(), Some("road"));
}
