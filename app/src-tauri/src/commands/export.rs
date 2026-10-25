//! Writing files: one sheet, or the whole series of diagrams into a folder.

use std::path::Path;

use openarchi_core::export::series::{self, file_name};
use openarchi_core::export::{export_with_dxf_layers, DxfLayers, ExportFormat};
use openarchi_core::highlight::Highlight;
use openarchi_core::pin::Pin;
use openarchi_core::project::Look;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::commands::render::{current_catalog, open_model, open_scene, project_name, with_input};
use crate::guard::blocking;
use crate::state::Shared;

/// The suggested name in the save dialog, the same one a series export uses.
#[tauri::command]
pub fn export_file_name(
    state: State<'_, Shared>,
    diagram_id: String,
    variant: Option<String>,
    format: ExportFormat,
) -> String {
    file_name(
        &project_name(&state),
        &diagram_id,
        variant.as_deref(),
        format,
    )
}

// The front end names each argument of the call, so they stay apart.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn export_sheet(
    state: State<'_, Shared>,
    look: Look,
    pins: Vec<Pin>,
    highlights: Vec<Highlight>,
    format: ExportFormat,
    dpi: u32,
    dxf_layers: DxfLayers,
    path: String,
) -> Result<String, String> {
    let state = state.inner().clone();
    blocking(move || {
        let scene = open_scene(&state)?;
        let catalog = current_catalog(&state);
        let model = open_model(&state);
        with_input(
            &catalog,
            &scene,
            &look,
            None,
            &pins,
            &highlights,
            model.as_ref(),
            |input| export_with_dxf_layers(input, format, dpi, dxf_layers, Path::new(&path)),
        )?;
        Ok(path)
    })
    .await
}

/// One file per diagram and per variant, in the given look's theme and sheet,
/// named after the project; see `series::files`.
// The front end names each argument of the call, so they stay apart.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn export_series(
    state: State<'_, Shared>,
    look: Look,
    pins: Vec<Pin>,
    highlights: Vec<Highlight>,
    format: ExportFormat,
    dpi: u32,
    dxf_layers: DxfLayers,
    folder: String,
) -> Result<Vec<String>, String> {
    let state = state.inner().clone();
    blocking(move || {
        let scene = open_scene(&state)?;
        let catalog = current_catalog(&state);
        let name = project_name(&state);
        let model = open_model(&state);

        let files = series::files(&catalog.diagrams, &name, &look, format, Path::new(&folder));

        let mut written = Vec::new();
        for file in &files {
            with_input(
                &catalog,
                &scene,
                &file.look,
                None,
                &pins,
                &highlights,
                model.as_ref(),
                |input| export_with_dxf_layers(input, format, dpi, dxf_layers, &file.path),
            )?;
            written.push(file.path.display().to_string());
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
