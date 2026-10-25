//! The site step: address search, measuring a drawn area, and fetching it.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use openarchi_core::area::Area;
use openarchi_core::geocode::{self, Place};
use openarchi_core::site::{self, build_scene, AreaSize, Progress};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::commands::library::{save_open_project, view_of, ProjectView};
use crate::guard::{blocking, calm, text};
use crate::state::{lock, AppState, Shared};

const SEARCH_LIMIT: usize = 6;

pub const FETCH_PROGRESS_EVENT: &str = "fetch-progress";
pub const FETCH_FINISHED_EVENT: &str = "fetch-finished";

#[derive(Clone, Serialize)]
struct FetchProgress {
    generation: u64,
    progress: Progress,
}

#[derive(Serialize, Clone)]
struct FetchFinished {
    generation: u64,
    error: Option<String>,
}

#[tauri::command]
pub async fn search_places(query: String) -> Result<Vec<Place>, String> {
    blocking(move || geocode::search(&query, SEARCH_LIMIT).map_err(text)).await
}

/// "2.6 ha, inside the Netherlands": shown live while the user draws.
#[tauri::command]
pub fn measure_area(area: Area) -> AreaSize {
    site::measure(&area)
}

fn fetch_and_store(
    app: &AppHandle,
    state: &AppState,
    area: Area,
    generation: u64,
) -> Result<(), String> {
    let on_progress = |progress: Progress| {
        let _ = app.emit(
            FETCH_PROGRESS_EVENT,
            FetchProgress {
                generation,
                progress,
            },
        );
    };
    let cache = state.library()?.tile_cache();
    let scene = build_scene(&area, &cache, &on_progress).map_err(text)?;
    if state.fetch_generation.load(Ordering::SeqCst) != generation {
        return Ok(());
    }

    match lock(&state.open).as_mut() {
        Some(open) => {
            open.project.area = area;
            open.scene = Some(Arc::new(scene));
            open.packed_scene = None;
        }
        None => return Err("The project was closed during the fetch.".to_string()),
    }
    save_open_project(state).map(|_| ())
}

/// Starts fetching `area` for the open project on a background thread.
/// Progress and the result arrive as events; the returned number names this fetch.
#[tauri::command]
pub fn fetch_site(app: AppHandle, state: State<'_, Shared>, area: Area) -> u64 {
    let state = state.inner().clone();
    let generation = state.fetch_generation.fetch_add(1, Ordering::SeqCst) + 1;

    std::thread::spawn(move || {
        let result = calm(|| fetch_and_store(&app, &state, area, generation));
        let _ = app.emit(
            FETCH_FINISHED_EVENT,
            FetchFinished {
                generation,
                error: result.err(),
            },
        );
    });
    generation
}

/// Stops listening to the running fetch. The data services still finish
/// their requests, but the result is dropped.
#[tauri::command]
pub fn cancel_fetch(state: State<'_, Shared>) {
    state.fetch_generation.fetch_add(1, Ordering::SeqCst);
}

/// The open project after a fetch finished.
#[tauri::command]
pub async fn current_project(state: State<'_, Shared>) -> Result<ProjectView, String> {
    match lock(&state.open).as_ref() {
        Some(open) => Ok(view_of(open)),
        None => Err("No project is open.".to_string()),
    }
}
