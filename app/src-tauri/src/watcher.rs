//! Watches the library's projects folder, so a project an assistant edited
//! through the MCP server shows up in the open window.
//!
//! Our own saves also touch the folder. Those are recognised by the time we
//! last saved each project and are not reported.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use openarchi_core::project::PROJECT_EXTENSION;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::state::{lock, AppState};

pub const LIBRARY_CHANGED_EVENT: &str = "library-changed";

/// A write within this long after our own save is taken to be ours.
const OWN_SAVE_WINDOW: Duration = Duration::from_secs(2);

#[derive(Clone, Serialize)]
struct LibraryChanged {
    id: String,
}

fn changed_id(path: &Path) -> Option<String> {
    if path.extension()?.to_str()? != PROJECT_EXTENSION {
        return None;
    }
    Some(path.file_stem()?.to_str()?.to_string())
}

fn is_own_save(state: &AppState, id: &str) -> bool {
    match lock(&state.own_saves).get(id) {
        Some(saved_at) => saved_at.elapsed() < OWN_SAVE_WINDOW,
        None => false,
    }
}

/// Starts watching. The returned watcher must be kept alive.
pub fn watch_projects(app: AppHandle, state: Arc<AppState>) -> Result<RecommendedWatcher, String> {
    let projects_dir = state.library()?.root().join("projects");
    std::fs::create_dir_all(&projects_dir)
        .map_err(|error| format!("Could not create {}: {error}", projects_dir.display()))?;

    let mut watcher = notify::recommended_watcher(move |result: notify::Result<notify::Event>| {
        let Ok(event) = result else { return };
        if event.kind.is_access() {
            return;
        }
        for path in &event.paths {
            let Some(id) = changed_id(path) else { continue };
            if is_own_save(&state, &id) {
                continue;
            }
            // Thumbnails of this project are stale now.
            lock(&state.thumbnails).remove(&id);
            let _ = app.emit(LIBRARY_CHANGED_EVENT, LibraryChanged { id });
        }
    })
    .map_err(|error| error.to_string())?;

    watcher
        .watch(&projects_dir, RecursiveMode::NonRecursive)
        .map_err(|error| error.to_string())?;
    Ok(watcher)
}
