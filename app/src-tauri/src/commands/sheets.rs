//! The open project's sheet tabs, pins and highlights: saving them, fitting a
//! sheet's scale to a new sub-selection, and turning a highlight into one.

use openarchi_core::area::Area;
use openarchi_core::highlight::Highlight;
use openarchi_core::pin::Pin;
use openarchi_core::project::Look;
use openarchi_core::render::fitting_scale as fit;
use tauri::State;

use crate::commands::library::save_open_project;
use crate::commands::render::open_scene;
use crate::guard::{blocking, text};
use crate::state::{lock, Shared};

/// Saves every sheet, in tab order, and the pins and highlights of the open project.
/// Returns the new modified time.
#[tauri::command]
pub async fn save_sheets(
    state: State<'_, Shared>,
    sheets: Vec<Look>,
    pins: Vec<Pin>,
    highlights: Vec<Highlight>,
) -> Result<String, String> {
    let state = state.inner().clone();
    blocking(move || {
        for pin in &pins {
            pin.check().map_err(text)?;
        }
        for highlight in &highlights {
            highlight.check().map_err(text)?;
        }
        match lock(&state.open).as_mut() {
            Some(open) => {
                open.project.set_sheets(sheets).map_err(text)?;
                open.project.pins = pins;
                open.project.highlights = highlights;
            }
            None => return Err("No project is open.".to_string()),
        }
        save_open_project(&state)
    })
    .await
}

/// The largest round scale at which `look`'s area, or its sub-selection, fills its drawing zone.
#[tauri::command]
pub async fn fitting_scale(state: State<'_, Shared>, look: Look) -> Result<u32, String> {
    let state = state.inner().clone();
    blocking(move || {
        let scene = open_scene(&state)?;
        fit(&scene, &look).map_err(text)
    })
    .await
}

/// The sheet area a highlight becomes when it is opened as a sheet, in longitude and latitude.
#[tauri::command]
pub fn highlight_area(highlight: Highlight) -> Result<Area, String> {
    highlight.check().map_err(text)?;
    Ok(highlight.shape.to_area())
}
