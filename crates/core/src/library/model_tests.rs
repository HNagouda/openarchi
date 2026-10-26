use std::fs;
use std::path::PathBuf;

use tempfile::TempDir;

use super::*;
use crate::area::LonLat;
use crate::edits::SiteEdits;
use crate::model;
use crate::project::packed_scene;
use crate::scene::Scene;

fn small_scene() -> Scene {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/westerkerk-environment.scene.json"
    );
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

fn house(scene: &Scene) -> Model {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/models/gable-house.glb");
    model::import(&path, None, None, scene, None).unwrap()
}

fn westermarkt() -> Area {
    Area::Circle {
        center: LonLat {
            lon: 4.8837,
            lat: 52.3747,
        },
        radius_m: 150.0,
    }
}

fn saved_text(folder: &TempDir, id: &str) -> String {
    fs::read_to_string(folder.path().join(format!("projects/{id}.openarchi"))).unwrap()
}

#[test]
fn a_model_is_saved_after_the_scene_and_loads_back_in_place() {
    let folder = TempDir::new().unwrap();
    let library = Library::at(folder.path()).unwrap();
    let mut project = library.create("Westermarkt", westermarkt()).unwrap();
    let scene = small_scene();
    let mut model = house(&scene);
    model.placement.rotation_deg = 30.0;
    project.model = Some(model.clone());
    project.edits = SiteEdits::default().with_model_picks(&["0363100012164990".to_string()]);
    project.scene = Some(scene);

    library.save(&mut project).unwrap();
    let reloaded = library.load(&project.id).unwrap();
    let loaded = reloaded.model.unwrap();

    let text = saved_text(&folder, &project.id);
    assert!(text.find("\"scene\":").unwrap() < text.find("\"model\":").unwrap());
    assert_eq!(loaded.placement, model.placement);
    assert_eq!(reloaded.edits, project.edits);
    assert_eq!(loaded.mesh.triangles, model.mesh.triangles);
}

#[test]
fn a_model_saved_beside_a_packed_scene_also_comes_after_it() {
    let folder = TempDir::new().unwrap();
    let library = Library::at(folder.path()).unwrap();
    let mut project = library.create("Westermarkt", westermarkt()).unwrap();
    let scene = small_scene();
    project.model = Some(house(&scene));
    let packed = packed_scene::pack(&scene).unwrap();

    library
        .save_with_packed_scene(&mut project, &packed)
        .unwrap();
    let summary = library.list().unwrap().remove(0);

    let text = saved_text(&folder, &project.id);
    assert!(text.find("\"scene\":").unwrap() < text.find("\"model\":").unwrap());
    assert!(
        project.model.is_some(),
        "the model went back into the project"
    );
    assert!(summary.has_scene && summary.error.is_none());
    assert!(library.load(&project.id).unwrap().model.is_some());
}

#[test]
fn a_project_without_a_model_saves_no_model_field() {
    let folder = TempDir::new().unwrap();
    let library = Library::at(folder.path()).unwrap();
    let project = library.create("Westermarkt", westermarkt()).unwrap();

    assert!(!saved_text(&folder, &project.id).contains("\"model\""));
    assert!(library.load(&project.id).unwrap().model.is_none());
}

#[test]
fn a_model_saved_before_site_edits_keeps_its_replaced_buildings_as_edits_under_it() {
    let folder = TempDir::new().unwrap();
    let library = Library::at(folder.path()).unwrap();
    let mut project = library.create("Westermarkt", westermarkt()).unwrap();
    let scene = small_scene();
    project.model = Some(house(&scene));
    project.scene = Some(scene);
    library.save(&mut project).unwrap();
    // Write the file the way the app saved it before edits: the list on the model.
    let path = folder
        .path()
        .join(format!("projects/{}.openarchi", project.id));
    let mut old: serde_json::Value =
        serde_json::from_str(&saved_text(&folder, &project.id)).unwrap();
    old["model"]["replaced"] = serde_json::json!(["0363100012164990", "0363100012164992"]);
    fs::write(&path, serde_json::to_string(&old).unwrap()).unwrap();

    let loaded = library.load(&project.id).unwrap();

    let removed: Vec<(&str, bool)> = loaded
        .edits
        .removed
        .iter()
        .map(|removed| (removed.id.as_str(), removed.is_under_model))
        .collect();
    assert_eq!(
        removed,
        vec![("0363100012164990", true), ("0363100012164992", true)]
    );
    assert!(loaded.model.is_some());
    assert!(loaded.edits.without_model_picks().is_empty());
}
