//! Ratios of the open site, for the inspector's Ratios view and its export.

use std::path::Path;

use openarchi_core::area::Area;
use openarchi_core::export::series::slug;
use openarchi_core::ratios::{self, ChartStyle, Ratios};
use tauri::State;

use crate::commands::render::{find_theme, project_name, shown_scene};
use crate::guard::{blocking, text};
use crate::state::Shared;

/// The ratios of the open site, or of the sheet's sub-selection, coloured by
/// `theme_id` so they match the sheet.
#[tauri::command]
pub async fn get_ratios(
    state: State<'_, Shared>,
    theme_id: String,
    selection: Option<Area>,
) -> Result<Ratios, String> {
    let state = state.inner().clone();
    blocking(move || {
        let scene = shown_scene(&state, selection.as_ref())?;
        let theme = find_theme(&state, &theme_id)?;
        ratios::compute(&scene, &theme).map_err(text)
    })
    .await
}

/// The suggested name in the save dialog: "westermarkt-ratios.csv".
#[tauri::command]
pub fn ratios_file_name(state: State<'_, Shared>) -> String {
    format!("{}-ratios.csv", slug(&project_name(&state)))
}

/// Writes the ratios as a CSV table or an SVG page of charts, chosen by the
/// extension the user picked in the save dialog.
#[tauri::command]
pub async fn export_ratios(
    state: State<'_, Shared>,
    theme_id: String,
    selection: Option<Area>,
    style: ChartStyle,
    path: String,
) -> Result<String, String> {
    let state = state.inner().clone();
    blocking(move || {
        let scene = shown_scene(&state, selection.as_ref())?;
        let theme = find_theme(&state, &theme_id)?;
        let ratios = ratios::compute(&scene, &theme).map_err(text)?;
        let extension = Path::new(&path)
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or_default()
            .to_lowercase();
        let contents = match extension.as_str() {
            "csv" => ratios::to_csv(&ratios),
            "svg" => ratios::chart_svg(&ratios, &theme, style, &project_name(&state)),
            _ => return Err(format!("Ratios export as .csv or .svg, not “{extension}”.")),
        };
        std::fs::write(&path, contents)
            .map_err(|error| format!("Couldn't write {path}: {error}"))?;
        Ok(path)
    })
    .await
}
