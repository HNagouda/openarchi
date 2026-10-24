//! Palettes for an assistant: import one from a palette generator's text, and
//! list the ones the user made. The same core rules as the app's palette editor.

use openarchi_core::catalog;
use openarchi_core::library::Library;
use openarchi_core::palette::{self, PaletteDraft, RoleChoice};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{json, Value};

/// The name an imported palette gets when the assistant gives none.
const DEFAULT_NAME: &str = "Imported palette";

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ImportPalette {
    /// What a palette generator exports or copies: hex codes ("f79f79, f7d08a"
    /// or "#f79f79 #f7d08a"), a JSON array or object of codes, an SVG of
    /// swatches, an XML palette, or a link ending in codes joined by dashes.
    /// 2 to 10 colours.
    pub text: String,
    /// The palette's name in the app, like "Sea glass".
    pub name: Option<String>,
    /// False only checks the palette and returns the roles and warnings
    /// without saving it. Defaults to true.
    pub save: Option<bool>,
}

pub fn import(library: &Library, input: ImportPalette) -> Result<Value, String> {
    let name = input
        .name
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or(DEFAULT_NAME.to_string());
    let draft = PaletteDraft {
        name: name.clone(),
        swatches: palette::parse(&input.text).map_err(|e| e.to_string())?,
        roles: RoleChoice::default(),
        typography: None,
        based_on: None,
    };
    let id = library.free_theme_id(&name);
    let checked = palette::check(&draft, &id).map_err(|e| e.to_string())?;

    let is_saved = input.save.unwrap_or(true);
    // The app's editor will not save a palette whose text cannot be read, so neither does the assistant.
    if is_saved && !checked.is_readable {
        return Err(format!(
            "\"{name}\" was not saved, because its text would be hard to read: {} Pick other colours, or call with save false to only check them.",
            checked.warnings.join(" ")
        ));
    }
    if is_saved {
        library
            .save_theme(&checked.theme)
            .map_err(|e| e.to_string())?;
    }
    Ok(json!({
        "id": if is_saved { Some(id) } else { None },
        "name": name,
        "swatches": draft.swatches,
        "roles": checked.roles,
        "palette": checked.theme.palette,
        "warnings": checked.warnings,
        "next": if is_saved {
            "Pass this id to set_look as theme_id to use it. The user can reassign roles in the app's palette editor."
        } else {
            "Nothing was saved. Call again with save true to keep it."
        },
    }))
}

/// The palettes the user made in the app or imported, with their swatches.
pub fn list_user(library: &Library) -> Value {
    let (themes, problems) = catalog::all_themes(&library.themes_dir());
    let builtin_ids: Vec<String> = catalog::builtin_themes()
        .into_iter()
        .map(|theme| theme.id)
        .collect();
    let palettes: Vec<Value> = themes
        .into_iter()
        .filter(|theme| !builtin_ids.contains(&theme.id))
        .map(|theme| {
            json!({
                "id": theme.id,
                "name": theme.name,
                "palette": theme.palette,
                "swatches": theme.source.map(|source| source.swatches),
            })
        })
        .collect();

    json!({ "palettes": palettes, "problems": problems })
}
