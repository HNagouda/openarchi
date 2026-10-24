use std::fs;

use serde_json::Value;
use tempfile::TempDir;

use crate::area::{Area, LonLat};
use crate::library::Library;
use crate::pin::{Pin, PinIcon};
use crate::project::{Look, PROJECT_FORMAT_VERSION};

fn westermarkt() -> Area {
    Area::Circle {
        center: LonLat {
            lon: 4.8837,
            lat: 52.3747,
        },
        radius_m: 150.0,
    }
}

fn library() -> (TempDir, Library) {
    let folder = TempDir::new().unwrap();
    let library = Library::at(folder.path()).unwrap();
    (folder, library)
}

fn saved_json(folder: &TempDir, id: &str) -> Value {
    let path = folder.path().join(format!("projects/{id}.openarchi"));
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

#[test]
fn a_project_with_one_sheet_and_no_pins_saves_exactly_as_before() {
    let (folder, library) = library();
    let project = library.create("Westermarkt", westermarkt()).unwrap();

    let saved = saved_json(&folder, &project.id);

    assert_eq!(saved["format_version"], Value::from(PROJECT_FORMAT_VERSION));
    for field in ["more_sheets", "pins"] {
        assert!(saved.get(field).is_none(), "{field} was written");
    }
    for field in ["name", "selection", "hide_pins"] {
        assert!(
            saved["look"].get(field).is_none(),
            "look.{field} was written"
        );
    }
}

#[test]
fn a_file_from_before_sheets_opens_as_one_sheet_without_pins() {
    let (_folder, library) = library();
    let project = library.create("Westermarkt", westermarkt()).unwrap();

    let loaded = library.load(&project.id).unwrap();

    assert_eq!(loaded.sheet_count(), 1);
    assert!(loaded.pins.is_empty());
    assert_eq!(loaded.look.name, "");
    assert!(loaded.look.selection.is_none());
    assert!(!loaded.look.hide_pins);
}

#[test]
fn sheets_and_pins_survive_a_save_and_load() {
    let (_folder, library) = library();
    let mut project = library.create("Westermarkt", westermarkt()).unwrap();
    let detail = Look {
        name: "Square".to_string(),
        selection: Some(Area::Rectangle {
            south_west: LonLat {
                lon: 4.8830,
                lat: 52.3742,
            },
            north_east: LonLat {
                lon: 4.8845,
                lat: 52.3750,
            },
        }),
        hide_pins: true,
        ..Look::default()
    };
    project
        .set_sheets(vec![project.look.clone(), detail.clone()])
        .unwrap();
    project.pins.push(Pin {
        id: "pin-1".to_string(),
        icon: PinIcon::Star,
        label: "Westerkerk".to_string(),
        x: 120_906.5,
        y: 487_460.25,
    });
    library.save(&mut project).unwrap();

    let loaded = library.load(&project.id).unwrap();

    assert_eq!(loaded.sheets(), project.sheets());
    assert_eq!(loaded.pins, project.pins);
    assert_eq!(loaded.more_sheets[0], detail);
}

#[test]
fn a_project_keeps_at_least_one_sheet() {
    let (_folder, library) = library();
    let mut project = library.create("Westermarkt", westermarkt()).unwrap();

    assert!(project.set_sheets(Vec::new()).is_err());
    assert!(project.sheet_mut(2).is_err());
    assert!(project.sheet_mut(1).is_ok());
}
