//! Laser cutting files for a physical site model of the sheet shown.

use std::path::Path;

use openarchi_core::export::laser::{self, LaserExport, LaserSettings, LaserSummary};
use openarchi_core::export::series::slug;
use openarchi_core::project::Look;
use tauri::State;

use crate::commands::render::{edited_scene, project_name};
use crate::guard::blocking;
use crate::state::Shared;

/// What the files would hold, shown in the dialog before anything is written.
#[tauri::command]
pub async fn laser_summary(
    state: State<'_, Shared>,
    look: Look,
    settings: LaserSettings,
) -> Result<LaserSummary, String> {
    let state = state.inner().clone();
    blocking(move || {
        let scene = edited_scene(&state)?;
        laser::summarize(&scene, look.selection.as_ref(), &settings)
            .map_err(|error| error.to_string())
    })
    .await
}

/// One file per laser bed into `folder`, named after the project.
#[tauri::command]
pub async fn export_laser(
    state: State<'_, Shared>,
    look: Look,
    settings: LaserSettings,
    folder: String,
) -> Result<LaserExport, String> {
    let state = state.inner().clone();
    blocking(move || {
        let scene = edited_scene(&state)?;
        laser::export(
            &scene,
            look.selection.as_ref(),
            &settings,
            Path::new(&folder),
            &slug(&project_name(&state)),
        )
        .map_err(|error| error.to_string())
    })
    .await
}
