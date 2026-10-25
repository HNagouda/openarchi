//! Laser cutting files for a physical site model of the sheet shown.

use std::path::Path;
use std::sync::Arc;

use openarchi_core::export::laser::{self, LaserExport, LaserSettings, LaserSummary};
use openarchi_core::export::series::slug;
use openarchi_core::project::Look;
use openarchi_core::scene::Scene;
use tauri::State;

use crate::guard::blocking;
use crate::state::{lock, Shared};

/// The open project's name and scene, or the message the dialog shows.
fn open_scene(state: &Shared) -> Result<(String, Arc<Scene>), String> {
    let open = lock(&state.open);
    let Some(open) = open.as_ref() else {
        return Err("No project is open.".to_string());
    };
    match &open.scene {
        Some(scene) => Ok((open.project.name.clone(), scene.clone())),
        None => Err("Fetch the site before exporting.".to_string()),
    }
}

/// What the files would hold, shown in the dialog before anything is written.
#[tauri::command]
pub async fn laser_summary(
    state: State<'_, Shared>,
    look: Look,
    settings: LaserSettings,
) -> Result<LaserSummary, String> {
    let state = state.inner().clone();
    blocking(move || {
        let (_, scene) = open_scene(&state)?;
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
        let (name, scene) = open_scene(&state)?;
        laser::export(
            &scene,
            look.selection.as_ref(),
            &settings,
            Path::new(&folder),
            &slug(&name),
        )
        .map_err(|error| error.to_string())
    })
    .await
}
