//! Site edits: trees and buildings taken away, trees moved and planted. The
//! sheet's undo history holds the edits; the app is handed them after each
//! change and draws every sheet with them. Each is a thin call into
//! `openarchi_core::edits`.

use openarchi_core::edits::{editable_at, edited_things, Editable, SiteEdits};
use tauri::State;

use crate::commands::render::open_scene;
use crate::guard::{blocking, text};
use crate::state::{lock, Shared};

/// Puts `edits` in effect for every sheet, the 3D view and the exports.
/// Saved with the next save of the sheets.
#[tauri::command]
pub async fn set_site_edits(state: State<'_, Shared>, edits: SiteEdits) -> Result<(), String> {
    let state = state.inner().clone();
    blocking(move || {
        let scene = open_scene(&state)?;
        edits.check(&scene).map_err(text)?;
        let mut open = lock(&state.open);
        let open = open.as_mut().ok_or("No project is open.")?;
        open.project.edits = edits;
        Ok(())
    })
    .await
}

/// The tree within `reach_m` of RD New point (`x`, `y`), or the building
/// under it, where `edits` leave them, removed ones too, for the edit tool.
#[tauri::command]
pub async fn pick_on_site(
    state: State<'_, Shared>,
    edits: SiteEdits,
    x: f64,
    y: f64,
    reach_m: f64,
) -> Result<Option<Editable>, String> {
    let state = state.inner().clone();
    blocking(move || {
        let scene = open_scene(&state)?;
        Ok(editable_at(&scene, &edits, x, y, reach_m))
    })
    .await
}

/// Every tree and building `edits` name, for the edit tool's list and its
/// marks of removed, moved and planted trees on the sheet.
#[tauri::command]
pub async fn site_edit_things(
    state: State<'_, Shared>,
    edits: SiteEdits,
) -> Result<Vec<Editable>, String> {
    let state = state.inner().clone();
    blocking(move || {
        let scene = open_scene(&state)?;
        Ok(edited_things(&scene, &edits))
    })
    .await
}
