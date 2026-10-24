//! Saving a theme an assistant designed into the user's themes folder.
//!
//! The theme is checked by `catalog::read_theme`, the same function the app
//! uses to load themes, so a theme that saves here is one the app will open.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use openarchi_core::catalog;
use openarchi_core::library::Library;
use openarchi_core::theme::Theme;
use serde_json::{Map, Value};

/// Replaced user themes go here, under the library's Recently Deleted folder.
const REPLACED_THEMES_FOLDER: &str = "trash/themes";

pub fn save_user_theme(library: &Library, theme_json: Map<String, Value>) -> Result<Theme, String> {
    let id = theme_id(&theme_json)?;
    if catalog::builtin_themes().iter().any(|theme| theme.id == id) {
        return Err(format!(
            "'{id}' is a built-in theme; give your theme a new id"
        ));
    }

    let theme = validate(&Value::Object(theme_json))?;
    let themes_dir = library.themes_dir();
    let target = themes_dir.join(format!("{id}.json"));
    if target.exists() {
        set_aside(library.root(), &target, &id)?;
    }

    let text = serde_json::to_string_pretty(&theme).map_err(|e| e.to_string())?;
    write_atomically(&target, text.as_bytes())?;
    Ok(theme)
}

/// "warm-brick". The id becomes the file name, so it has to be plain.
fn theme_id(theme_json: &Map<String, Value>) -> Result<String, String> {
    let Some(id) = theme_json.get("id").and_then(Value::as_str) else {
        return Err("the theme needs an \"id\", like \"warm-brick\"".to_string());
    };
    let is_plain = !id.is_empty()
        && !id.starts_with('-')
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !is_plain {
        return Err(format!(
            "theme id \"{id}\" must be lowercase letters, digits and dashes, like \"warm-brick\""
        ));
    }

    Ok(id.to_string())
}

/// Runs the app's own theme reader over a temporary copy. The copy lives
/// outside the themes folder, so a theme that fails never shows in the app.
fn validate(theme: &Value) -> Result<Theme, String> {
    let mut draft = tempfile::Builder::new()
        .suffix(".json")
        .tempfile()
        .map_err(|e| format!("could not write a temporary theme file: {e}"))?;
    serde_json::to_writer(&mut draft, theme).map_err(|e| e.to_string())?;
    draft.flush().map_err(|e| e.to_string())?;

    catalog::read_theme(draft.path()).map_err(|e| format!("the theme is not valid: {e}"))
}

/// Moves the theme being replaced into Recently Deleted, so redesigning a
/// theme never loses the earlier one.
fn set_aside(root: &Path, current: &Path, id: &str) -> Result<(), String> {
    let folder = root.join(REPLACED_THEMES_FOLDER);
    fs::create_dir_all(&folder).map_err(|e| format!("{}: {e}", folder.display()))?;

    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis())
        .unwrap_or_default();
    let target = folder.join(format!("{id}-{millis}.json"));
    fs::rename(current, &target).map_err(|e| format!("{}: {e}", current.display()))
}

/// Temp file plus rename, as the library saves projects, so a crash never
/// leaves half a theme that the app would report as broken.
fn write_atomically(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let temp = path.with_extension("json.tmp");
    let describe = |e: std::io::Error| format!("{}: {e}", path.display());

    let mut file = fs::File::create(&temp).map_err(describe)?;
    file.write_all(bytes).map_err(describe)?;
    file.sync_all().map_err(describe)?;
    // Windows cannot rename a file that is still open.
    drop(file);

    fs::rename(&temp, path).map_err(describe)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn json(value: Value) -> Map<String, Value> {
        value.as_object().unwrap().clone()
    }

    #[test]
    fn theme_without_an_id_is_refused() {
        let error = theme_id(&json(serde_json::json!({ "name": "Brick" }))).unwrap_err();
        assert!(error.contains("\"id\""), "{error}");
    }

    #[test]
    fn theme_id_that_is_not_a_plain_slug_is_refused() {
        for id in ["Warm Brick", "../escape", "", "-lead"] {
            assert!(
                theme_id(&json(serde_json::json!({ "id": id }))).is_err(),
                "{id:?}"
            );
        }
        assert_eq!(
            theme_id(&json(serde_json::json!({ "id": "warm-brick-2" }))).unwrap(),
            "warm-brick-2"
        );
    }
}
