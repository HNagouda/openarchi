//! The live 3D view: the open site as triangles, the aerial photo for its
//! realistic finish, the sun through a chosen day, and saving the pictures
//! the view takes. The view itself draws in the
//! window with WebGL; see app/src/viewer/.

use std::path::Path;

use base64::Engine;
use openarchi_core::project::Look;
use openarchi_core::view3d::photo::{self, Photo};
use openarchi_core::view3d::{self, SunDay, View3d};
use tauri::State;

use crate::commands::render::{current_catalog, open_model, open_scene, with_input};
use crate::guard::{blocking, text};
use crate::state::Shared;

const PNG_EXTENSION: &str = "png";

/// The open site, its neighbourhood and the model in effect, in the colours
/// of the sheet `look` draws: its diagram's classes and its palette.
#[tauri::command]
pub async fn view_3d(state: State<'_, Shared>, look: Look) -> Result<View3d, String> {
    let state = state.inner().clone();
    blocking(move || {
        let scene = open_scene(&state)?;
        let model = open_model(&state);
        let catalog = current_catalog(&state);
        with_input(
            &catalog,
            &scene,
            &look,
            None,
            &[],
            &[],
            model.as_ref(),
            view3d::view,
        )
    })
    .await
}

/// The aerial photo over the open site and its neighbourhood, from the tile
/// cache or PDOK. Asked for the first time the view shows its realistic
/// finish, not with the site, so a site fetch never waits for it.
#[tauri::command]
pub async fn aerial_photo(state: State<'_, Shared>) -> Result<Photo, String> {
    let state = state.inner().clone();
    blocking(move || {
        let scene = open_scene(&state)?;
        let cache = state.library()?.tile_cache();
        photo::photo(&scene, &cache).map_err(text)
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
