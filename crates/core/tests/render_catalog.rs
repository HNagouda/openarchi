use std::path::Path;

use openarchi_core::catalog::{all_themes, builtin_diagrams, builtin_themes, read_theme};
use openarchi_core::sheet::Sheet;
use openarchi_core::theme::THEME_FORMAT_VERSION;

fn paper_json() -> serde_json::Value {
    serde_json::to_value(builtin_themes().into_iter().next().expect("paper"))
        .expect("theme as JSON")
}

fn write(folder: &Path, name: &str, value: &serde_json::Value) -> std::path::PathBuf {
    let path = folder.join(name);
    std::fs::write(&path, serde_json::to_string_pretty(value).expect("json")).expect("write");
    path
}

#[test]
fn builtins_come_in_picker_order_with_the_defaults_first() {
    let diagrams: Vec<String> = builtin_diagrams().into_iter().map(|d| d.id).collect();
    let themes: Vec<String> = builtin_themes().into_iter().map(|t| t.id).collect();

    assert_eq!(diagrams[0], "context");
    assert_eq!(themes[0], "paper");
    assert_eq!(diagrams.len(), 29);
    assert!(themes.len() >= 6);
    assert_eq!(Sheet::default().scale, 2500);
}

#[test]
fn a_copied_builtin_theme_reads_back_unchanged() {
    let folder = tempfile::tempdir().expect("temp folder");
    let mut value = paper_json();
    value["id"] = "my-paper".into();
    let path = write(folder.path(), "my-paper.json", &value);

    let theme = read_theme(&path).expect("valid theme");

    assert_eq!(theme.id, "my-paper");
    assert_eq!(theme.format_version, THEME_FORMAT_VERSION);
}

#[test]
fn read_theme_rejects_a_colour_role_that_does_not_exist() {
    let folder = tempfile::tempdir().expect("temp folder");
    let mut value = paper_json();
    value["layers"]["road"]["fill"] = "@asphalt".into();
    let path = write(folder.path(), "bad.json", &value);

    let message = read_theme(&path).expect_err("invalid").to_string();

    assert!(
        message.contains("layers.road.fill") && message.contains("@asphalt"),
        "{message}"
    );
}

#[test]
fn read_theme_rejects_short_ramps_and_too_few_categories() {
    let folder = tempfile::tempdir().expect("temp folder");
    let mut value = paper_json();
    value["ramp"] = serde_json::json!(["#FFFFFF", "#000000"]);
    value["categories"] = serde_json::json!(["#FFFFFF"]);
    let path = write(folder.path(), "short.json", &value);

    let message = read_theme(&path).expect_err("invalid").to_string();

    assert!(
        message.contains("ramp needs at least 3")
            && message.contains("categories needs at least 6"),
        "{message}"
    );
}

#[test]
fn read_theme_rejects_a_palette_colour_that_is_not_hex() {
    let folder = tempfile::tempdir().expect("temp folder");
    let mut value = paper_json();
    value["palette"]["accent"] = "red".into();
    let path = write(folder.path(), "red.json", &value);

    assert!(read_theme(&path).is_err());
}

#[test]
fn read_theme_rejects_a_newer_format_version() {
    let folder = tempfile::tempdir().expect("temp folder");
    let mut value = paper_json();
    value["format_version"] = (THEME_FORMAT_VERSION + 1).into();
    let path = write(folder.path(), "future.json", &value);

    assert!(read_theme(&path)
        .expect_err("too new")
        .to_string()
        .contains("format_version"));
}

#[test]
fn all_themes_skips_and_reports_broken_files() {
    let folder = tempfile::tempdir().expect("temp folder");
    let mut good = paper_json();
    good["id"] = "mine".into();
    write(folder.path(), "mine.json", &good);
    std::fs::write(folder.path().join("broken.json"), "{ not json").expect("write");
    write(folder.path(), "duplicate.json", &paper_json());
    std::fs::write(folder.path().join("notes.txt"), "ignored").expect("write");

    let (themes, problems) = all_themes(folder.path());

    assert_eq!(themes.len(), builtin_themes().len() + 1);
    assert_eq!(themes.last().map(|t| t.id.as_str()), Some("mine"));
    assert_eq!(problems.len(), 2, "{problems:?}");
}

#[test]
fn all_themes_without_a_user_folder_is_just_the_builtins() {
    let (themes, problems) = all_themes(Path::new("/does/not/exist"));

    assert_eq!(themes.len(), builtin_themes().len());
    assert!(problems.is_empty());
}
