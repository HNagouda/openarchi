//! Ratios of the open site, for the inspector's Ratios view and its export.

use std::path::Path;

use openarchi_core::ratios::{self, ChartStyle, Ratios};
use openarchi_core::theme::Theme;
use tauri::State;

use crate::commands::export::slug;
use crate::commands::render::{current_catalog, open_scene};
use crate::guard::{blocking, text};
use crate::state::{lock, AppState, Shared};

/// The ratios of the open site, coloured by `theme_id` so they match the sheet.
#[tauri::command]
pub async fn get_ratios(state: State<'_, Shared>, theme_id: String) -> Result<Ratios, String> {
    let state = state.inner().clone();
    blocking(move || {
        let scene = open_scene(&state)?;
        let theme = find_theme(&state, &theme_id)?;
        ratios::compute(&scene, &theme).map_err(text)
    })
    .await
}

/// The suggested name in the save dialog: "westermarkt-ratios.csv".
#[tauri::command]
pub fn ratios_file_name(state: State<'_, Shared>) -> String {
    let name = lock(&state.open)
        .as_ref()
        .map(|open| open.project.name.clone())
        .unwrap_or_default();
    format!("{}-ratios.csv", slug(&name))
}

/// Writes the ratios as a CSV table or an SVG page of charts, chosen by the
/// extension the user picked in the save dialog.
#[tauri::command]
pub async fn export_ratios(
    state: State<'_, Shared>,
    theme_id: String,
    style: ChartStyle,
    path: String,
) -> Result<String, String> {
    let state = state.inner().clone();
    blocking(move || {
        let scene = open_scene(&state)?;
        let theme = find_theme(&state, &theme_id)?;
        let ratios = ratios::compute(&scene, &theme).map_err(text)?;
        let extension = Path::new(&path)
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or_default()
            .to_lowercase();
        let contents = match extension.as_str() {
            "csv" => ratios::to_csv(&ratios),
            "svg" => {
                let title = lock(&state.open)
                    .as_ref()
                    .map(|open| open.project.name.clone())
                    .unwrap_or_default();
                ratios::chart_svg(&ratios, &theme, style, &title)
            }
            _ => return Err(format!("Ratios export as .csv or .svg, not “{extension}”.")),
        };
        std::fs::write(&path, contents)
            .map_err(|error| format!("Couldn't write {path}: {error}"))?;
        Ok(path)
    })
    .await
}

fn find_theme(state: &AppState, id: &str) -> Result<Theme, String> {
    current_catalog(state)
        .themes
        .into_iter()
        .find(|theme| theme.id == id)
        .ok_or_else(|| format!("There is no theme called “{id}”."))
}
