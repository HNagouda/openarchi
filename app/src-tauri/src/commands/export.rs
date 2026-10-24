//! Writing files: one sheet, or the whole series of diagrams into a folder.

use std::path::{Path, PathBuf};

use openarchi_core::export::{export, ExportFormat};
use openarchi_core::project::Look;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::commands::render::{current_catalog, with_input};
use crate::guard::blocking;
use crate::state::{lock, Shared};

/// "Westermarkt, Amsterdam" -> "westermarkt-amsterdam".
fn slug(name: &str) -> String {
    let mut slug = String::new();
    for letter in name.to_lowercase().chars() {
        if letter.is_alphanumeric() {
            slug.push(letter);
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }

    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        "site".to_string()
    } else {
        slug
    }
}

/// The file name an export of `diagram_id` gets: "westermarkt-figure-ground.pdf".
fn file_name(project_name: &str, diagram_id: &str, format: ExportFormat) -> String {
    format!(
        "{}-{}.{}",
        slug(project_name),
        diagram_id,
        format.extension()
    )
}

/// The suggested name in the save dialog, the same one a series export uses.
#[tauri::command]
pub fn export_file_name(
    state: State<'_, Shared>,
    diagram_id: String,
    format: ExportFormat,
) -> String {
    let name = lock(&state.open)
        .as_ref()
        .map(|open| open.project.name.clone())
        .unwrap_or_default();
    file_name(&name, &diagram_id, format)
}

#[tauri::command]
pub async fn export_sheet(
    state: State<'_, Shared>,
    look: Look,
    format: ExportFormat,
    dpi: u32,
    path: String,
) -> Result<String, String> {
    let state = state.inner().clone();
    blocking(move || {
        let scene = match lock(&state.open)
            .as_ref()
            .and_then(|open| open.scene.clone())
        {
            Some(scene) => scene,
            None => return Err("Fetch the site before exporting.".to_string()),
        };
        let catalog = current_catalog(&state);
        with_input(&catalog, &scene, &look, |input| {
            export(input, format, dpi, Path::new(&path))
        })?;
        Ok(path)
    })
    .await
}

/// One file per diagram, in the given look's theme and sheet, named after the project.
#[tauri::command]
pub async fn export_series(
    state: State<'_, Shared>,
    look: Look,
    format: ExportFormat,
    dpi: u32,
    folder: String,
) -> Result<Vec<String>, String> {
    let state = state.inner().clone();
    blocking(move || {
        let (name, scene) = match lock(&state.open).as_ref() {
            Some(open) => (open.project.name.clone(), open.scene.clone()),
            None => return Err("No project is open.".to_string()),
        };
        let Some(scene) = scene else {
            return Err("Fetch the site before exporting.".to_string());
        };
        let catalog = current_catalog(&state);

        let mut written = Vec::new();
        for diagram in &catalog.diagrams {
            let path: PathBuf = Path::new(&folder).join(file_name(&name, &diagram.id, format));
            let diagram_look = Look {
                diagram_id: diagram.id.clone(),
                ..look.clone()
            };
            with_input(&catalog, &scene, &diagram_look, |input| {
                export(input, format, dpi, &path)
            })?;
            written.push(path.display().to_string());
        }
        Ok(written)
    })
    .await
}

/// Shows a file in Finder or Explorer.
#[tauri::command]
pub fn reveal_path(app: AppHandle, path: String) -> Result<(), String> {
    app.opener()
        .reveal_item_in_dir(path)
        .map_err(|error| error.to_string())
}

/// Opens a web page in the user's browser.
#[tauri::command]
pub fn open_link(app: AppHandle, url: String) -> Result<(), String> {
    if !url.starts_with("https://") {
        return Err(format!("Only https links open from the app, got {url:?}."));
    }
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|error| error.to_string())
}
