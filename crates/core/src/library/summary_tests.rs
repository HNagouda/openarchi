use std::fs;
use std::path::PathBuf;

use super::tests::{library, westermarkt};
use super::*;
use crate::scene::Scene;

const SCENE_KEY: &str = ",\"scene\":";
const LOOK_KEY: &str = ",\"look\":";

fn small_scene() -> Scene {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/westerkerk-environment.scene.json"
    );
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

fn project_path(library: &Library, id: &str) -> PathBuf {
    library.root().join(format!("projects/{id}.openarchi"))
}

/// The file at `path` rewritten with the scene right after the area, the
/// order `Library::save` wrote before the scene moved last.
fn move_scene_before_look(path: &Path) {
    let text = fs::read_to_string(path).unwrap();
    let scene_at = text.find(SCENE_KEY).unwrap();
    let (head, scene) = text.split_at(scene_at);
    let scene = scene.strip_suffix('}').unwrap();
    let look_at = head.find(LOOK_KEY).unwrap();
    let (before_look, from_look) = head.split_at(look_at);
    fs::write(path, format!("{before_look}{scene}{from_look}}}")).unwrap();
}

#[test]
fn a_saved_scene_comes_last_and_the_list_still_sees_it() {
    let (_folder, library) = library();
    let mut project = library.create("Westermarkt", westermarkt()).unwrap();
    project.scene = Some(small_scene());
    library.save(&mut project).unwrap();

    let text = fs::read_to_string(project_path(&library, &project.id)).unwrap();
    let listed = &library.list().unwrap()[0];

    assert!(text.find(LOOK_KEY).unwrap() < text.find(SCENE_KEY).unwrap());
    assert!(listed.has_scene);
    assert_eq!(listed.look.as_ref(), Some(&project.look));
    assert_eq!(library.modified(&project.id).unwrap(), project.modified);
}

#[test]
fn a_file_with_its_scene_before_its_look_still_lists_and_backs_up_under_its_stamp() {
    let (_folder, library) = library();
    let mut project = library.create("Westermarkt", westermarkt()).unwrap();
    project.scene = Some(small_scene());
    library.save(&mut project).unwrap();
    let path = project_path(&library, &project.id);
    move_scene_before_look(&path);

    let listed = &library.list().unwrap()[0];
    assert!(listed.has_scene);
    assert_eq!(listed.look.as_ref(), Some(&project.look));
    assert_eq!(library.modified(&project.id).unwrap(), project.modified);

    let stamp_before = stamp_from_rfc3339(&project.modified);
    library.save(&mut project).unwrap();
    assert_eq!(library.backups(&project.id).unwrap()[0].stamp, stamp_before);
}

#[test]
fn a_name_that_spells_the_scene_key_lists_as_written() {
    let (_folder, library) = library();
    let name = r#"Plein ,"scene":null"#;
    let mut project = library.create(name, westermarkt()).unwrap();
    project.scene = Some(small_scene());
    library.save(&mut project).unwrap();

    let listed = &library.list().unwrap()[0];

    assert_eq!(listed.name, name);
    assert!(listed.has_scene);
}

#[test]
fn a_project_without_a_scene_lists_without_one() {
    let (_folder, library) = library();
    let project = library.create("Westermarkt", westermarkt()).unwrap();

    let listed = &library.list().unwrap()[0];

    assert!(!listed.has_scene);
    assert_eq!(listed.look.as_ref(), Some(&project.look));
}
