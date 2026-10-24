use std::fs;

use tempfile::TempDir;

use super::*;
use crate::area::LonLat;
use crate::project::packed_scene;
use crate::scene::Scene;

fn small_scene() -> Scene {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/westerkerk-environment.scene.json"
    );
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
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

#[test]
fn a_project_saved_with_its_scene_packed_ahead_loads_back_whole() {
    let folder = TempDir::new().unwrap();
    let library = Library::at(folder.path()).unwrap();
    let mut project = library.create("Westermarkt", westermarkt()).unwrap();
    let scene = small_scene();
    let packed = packed_scene::pack(&scene).unwrap();

    project.look.sheet.title = "Packed ahead".to_string();
    library
        .save_with_packed_scene(&mut project, &packed)
        .unwrap();
    let loaded = library.load(&project.id).unwrap();

    assert_eq!(loaded.look.sheet.title, "Packed ahead");
    assert_eq!(loaded.scene.unwrap().features.len(), scene.features.len());
}

#[test]
fn a_packed_scene_is_refused_next_to_the_scene_itself() {
    let folder = TempDir::new().unwrap();
    let library = Library::at(folder.path()).unwrap();
    let mut project = library.create("Westermarkt", westermarkt()).unwrap();
    let scene = small_scene();
    let packed = packed_scene::pack(&scene).unwrap();
    project.scene = Some(scene);

    assert!(library
        .save_with_packed_scene(&mut project, &packed)
        .is_err());
}

#[test]
fn a_project_without_a_scene_saves_and_loads_without_one() {
    let folder = TempDir::new().unwrap();
    let library = Library::at(folder.path()).unwrap();
    let mut project = library.create("Westermarkt", westermarkt()).unwrap();

    library.save(&mut project).unwrap();

    assert!(library.load(&project.id).unwrap().scene.is_none());
}
