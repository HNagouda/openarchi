//! The live 3D view: the open site as triangles, the sun through a chosen
//! day, and saving the pictures the view takes. The view itself draws in the
//! window with WebGL; see app/src/viewer/.

use std::path::Path;

use base64::Engine;
use openarchi_core::view3d::{self, SunDay, View3d};
use tauri::State;

use crate::commands::render::{find_theme, open_model, open_scene};
use crate::guard::{blocking, text};
use crate::state::Shared;

const PNG_EXTENSION: &str = "png";

/// The open site, its neighbourhood and the model in effect, in `theme_id`'s ground colours.
#[tauri::command]
pub async fn view_3d(state: State<'_, Shared>, theme_id: String) -> Result<View3d, String> {
    let state = state.inner().clone();
    blocking(move || {
        let scene = open_scene(&state)?;
        let theme = find_theme(&state, &theme_id)?;
        let model = open_model(&state);
        view3d::view(&scene, &theme, model.as_ref()).map_err(text)
    })
    .await
}

/// Where the sun stands at the open site through `month`/`day`.
#[tauri::command]
pub async fn sun_day(state: State<'_, Shared>, month: u32, day: u32) -> Result<SunDay, String> {
    let state = state.inner().clone();
    blocking(move || {
        let scene = open_scene(&state)?;
        Ok(view3d::sun_day(&scene, month, day))
    })
    .await
}

/// Writes a picture the view took, sent as base64 PNG, to `path`, which the
/// user chose in a save dialog or which names a frame in a chosen folder.
#[tauri::command]
pub async fn save_view_png(path: String, png: String) -> Result<String, String> {
    blocking(move || {
        let target = Path::new(&path);
        let is_png = target
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case(PNG_EXTENSION));
        if !is_png {
            return Err(format!(
                "A picture of the view is saved as .png, not {path}."
            ));
        }
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(png)
            .map_err(|error| format!("The picture could not be read: {error}."))?;
        std::fs::write(target, bytes)
            .map_err(|error| format!("Couldn't write {path}: {error}."))?;
        Ok(path)
    })
    .await
}
