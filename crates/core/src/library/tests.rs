use std::fs;

use tempfile::TempDir;

use super::*;
use crate::area::LonLat;

pub(super) fn westermarkt() -> Area {
    Area::Circle {
        center: LonLat {
            lon: 4.8837,
            lat: 52.3747,
        },
        radius_m: 150.0,
    }
}

/// An empty library in a folder that is removed after the test.
pub(super) fn library() -> (TempDir, Library) {
    let folder = TempDir::new().unwrap();
    let library = Library::at(folder.path()).unwrap();
    (folder, library)
}

/// Saves `project` with a new title, so each version is distinguishable.
fn save_with_title(library: &Library, project: &mut Project, title: &str) {
    project.look.sheet.title = title.to_string();
    library.save(project).unwrap();
}

#[test]
fn saved_project_loads_back_unchanged() {
    let (_folder, library) = library();
    let mut project = library.create("Westermarkt", westermarkt()).unwrap();
    save_with_title(&library, &mut project, "Context");

    let loaded = library.load(&project.id).unwrap();

    assert_eq!(loaded.name, "Westermarkt");
    assert_eq!(loaded.area, westermarkt());
    assert_eq!(loaded.look, project.look);
    assert_eq!(loaded.modified, project.modified);
}

#[test]
fn project_id_is_date_slug_and_four_hex_characters() {
    let (_folder, library) = library();
    let project = library
        .create("Westermarkt 20, Amsterdam!", westermarkt())
        .unwrap();

    let (date, rest) = project.id.split_at(10);
    let (slug, suffix) = rest.rsplit_once('-').unwrap();

    assert_eq!(date, &project.created[..10]);
    assert_eq!(slug, "-westermarkt-20-amsterdam");
    assert_eq!(suffix.len(), 4);
    assert!(suffix.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn name_without_ascii_letters_still_gets_an_id() {
    assert_eq!(slugify("日本"), "project");
    assert_eq!(slugify("  --Café  Noord-- "), "caf-noord");
}

#[test]
fn empty_name_is_refused() {
    let (_folder, library) = library();
    assert!(library.create("   ", westermarkt()).is_err());
}

#[test]
fn second_save_backs_up_the_first_version() {
    let (_folder, library) = library();
    let mut project = library.create("Westermarkt", westermarkt()).unwrap();
    assert!(library.backups(&project.id).unwrap().is_empty());

    let first_modified = project.modified.clone();
    save_with_title(&library, &mut project, "Second");

    let backups = library.backups(&project.id).unwrap();
    assert_eq!(backups.len(), 1);
    assert_eq!(backups[0].modified, first_modified);
}

#[test]
fn backups_are_pruned_to_the_newest_kept() {
    let (_folder, library) = library();
    let mut project = library.create("Westermarkt", westermarkt()).unwrap();
    for version in 0..BACKUPS_KEPT + 5 {
        save_with_title(&library, &mut project, &format!("Version {version}"));
    }

    let backups = library.backups(&project.id).unwrap();
    assert_eq!(backups.len(), BACKUPS_KEPT);

    let newest = library
        .restore_backup(&project.id, &backups[0].stamp)
        .unwrap();
    assert_eq!(
        newest.look.sheet.title,
        format!("Version {}", BACKUPS_KEPT + 3)
    );
}

#[test]
fn backups_are_listed_newest_first() {
    let (_folder, library) = library();
    let mut project = library.create("Westermarkt", westermarkt()).unwrap();
    for version in 0..3 {
        save_with_title(&library, &mut project, &format!("Version {version}"));
    }

    let stamps: Vec<String> = library
        .backups(&project.id)
        .unwrap()
        .into_iter()
        .map(|backup| backup.stamp)
        .collect();
    let mut sorted = stamps.clone();
    sorted.sort_by(|a, b| b.cmp(a));
    assert_eq!(stamps, sorted);
}

#[test]
fn deleted_project_moves_to_trash_and_restores() {
    let (_folder, library) = library();
    let project = library.create("Westermarkt", westermarkt()).unwrap();

    library.delete(&project.id).unwrap();
    assert!(library.list().unwrap().is_empty());
    assert!(library.load(&project.id).is_err());
    assert_eq!(library.trash().unwrap()[0].id, project.id);

    library.restore(&project.id).unwrap();
    assert!(library.trash().unwrap().is_empty());
    assert_eq!(library.load(&project.id).unwrap().name, "Westermarkt");
}

#[test]
fn trash_lists_the_most_recently_deleted_first() {
    let (_folder, library) = library();
    let first = library.create("First", westermarkt()).unwrap();
    let second = library.create("Second", westermarkt()).unwrap();

    library.delete(&second.id).unwrap();
    // File times on some file systems are only accurate to a few milliseconds.
    std::thread::sleep(std::time::Duration::from_millis(20));
    library.delete(&first.id).unwrap();

    let ids: Vec<String> = library.trash().unwrap().into_iter().map(|p| p.id).collect();
    assert_eq!(ids, vec![first.id, second.id]);
}

#[test]
fn restore_backup_backs_up_the_current_version_first() {
    let (_folder, library) = library();
    let mut project = library.create("Westermarkt", westermarkt()).unwrap();
    save_with_title(&library, &mut project, "Old");
    save_with_title(&library, &mut project, "Current");
    let old_stamp = library.backups(&project.id).unwrap()[0].stamp.clone();

    let restored = library.restore_backup(&project.id, &old_stamp).unwrap();

    assert_eq!(restored.look.sheet.title, "Old");
    assert_eq!(library.load(&project.id).unwrap().look.sheet.title, "Old");
    let newest_backup = library.backups(&project.id).unwrap()[0].stamp.clone();
    let undone = library.restore_backup(&project.id, &newest_backup).unwrap();
    assert_eq!(undone.look.sheet.title, "Current");
}

#[test]
fn list_is_sorted_newest_first() {
    let (_folder, library) = library();
    let older = library.create("Older", westermarkt()).unwrap();
    let mut newer = library.create("Newer", westermarkt()).unwrap();
    library.save(&mut newer).unwrap();

    let ids: Vec<String> = library.list().unwrap().into_iter().map(|p| p.id).collect();
    assert_eq!(ids, vec![newer.id, older.id]);
}

#[test]
fn list_carries_each_project_look() {
    let (_folder, library) = library();
    let mut project = library.create("Westermarkt", westermarkt()).unwrap();
    save_with_title(&library, &mut project, "Building age");

    let listed = library.list().unwrap();

    assert_eq!(listed[0].look.as_ref(), Some(&project.look));
}

#[test]
fn corrupt_file_is_listed_with_an_error_and_the_rest_still_list() {
    let (folder, library) = library();
    let good = library.create("Westermarkt", westermarkt()).unwrap();
    let broken_path = folder.path().join("projects/broken.openarchi");
    fs::write(&broken_path, "{ this is not json").unwrap();

    let projects = library.list().unwrap();
    let broken = projects.iter().find(|p| p.id == "broken").unwrap();
    let healthy = projects.iter().find(|p| p.id == good.id).unwrap();

    assert!(broken.error.as_ref().unwrap().contains("broken.openarchi"));
    assert!(healthy.error.is_none());
    let error = library.load("broken").unwrap_err().to_string();
    assert!(error.contains("broken.openarchi"), "{error}");
}

#[test]
fn corrupt_file_is_backed_up_before_it_is_overwritten() {
    let (folder, library) = library();
    let mut project = library.create("Westermarkt", westermarkt()).unwrap();
    let path = folder
        .path()
        .join(format!("projects/{}.openarchi", project.id));
    fs::write(&path, "{ half a file").unwrap();

    library.save(&mut project).unwrap();

    let backups = library.backups(&project.id).unwrap();
    assert_eq!(backups.len(), 1);
    let backup_path = folder.path().join(format!(
        "backups/{}/{}.openarchi",
        project.id, backups[0].stamp
    ));
    assert_eq!(fs::read_to_string(backup_path).unwrap(), "{ half a file");
}

#[test]
fn ids_that_leave_the_library_are_refused() {
    let (_folder, library) = library();
    for id in ["../escape", "a/b", "a\\b", "", ".hidden", "c:evil"] {
        assert!(library.load(id).is_err(), "{id:?} was accepted");
        assert!(library.backups(id).is_err(), "{id:?} was accepted");
    }
}

#[test]
fn stamps_and_times_convert_both_ways() {
    let time = "2026-10-24T15:30:12.123456789Z";
    let stamp = stamp_from_rfc3339(time);
    assert_eq!(stamp, "2026-10-24T15-30-12.123456789Z");
    assert_eq!(rfc3339_from_stamp(&stamp), time);
}
