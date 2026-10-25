//! Projects in the library: list, open, rename, delete, restore, save.

use std::sync::Arc;
use std::time::Instant;

use openarchi_core::area::Area;
use openarchi_core::highlight::Highlight;
use openarchi_core::library::{Library, ProjectSummary};
use openarchi_core::pin::Pin;
use openarchi_core::project::{packed_scene, Look, Project};
use openarchi_core::report::Report;
use openarchi_core::scene::{Credit, Layer, Scene};
use serde::Serialize;
use tauri::State;

use crate::commands::model::{current_model_view, ModelView};
use crate::guard::{blocking, text};
use crate::state::{lock, AppState, OpenProject, Shared};

/// The open project as the window needs it: everything but the scene itself.
#[derive(Serialize)]
pub struct ProjectView {
    pub id: String,
    pub name: String,
    pub modified: String,
    pub area: Area,
    pub look: Look,
    /// Every sheet in tab order, the first being `look`.
    pub sheets: Vec<Look>,
    pub pins: Vec<Pin>,
    pub highlights: Vec<Highlight>,
    pub has_scene: bool,
    pub fetched_at: Option<String>,
    pub warnings: Vec<String>,
    pub credits: Vec<Credit>,
    /// Layers that hold at least one feature, so the inspector lists only those.
    pub layers_present: Vec<Layer>,
    pub reports: Vec<Report>,
    /// The imported model in effect, `None` when the project has none.
    pub model: Option<ModelView>,
}

pub fn view_of(open: &OpenProject) -> ProjectView {
    let project = &open.project;
    let scene: Option<&Scene> = open.scene.as_deref();
    let mut layers_present: Vec<Layer> = Vec::new();
    if let Some(scene) = scene {
        // The walk reaches past the site, so its layers count as present too.
        let walk_features = scene
            .walk
            .iter()
            .flat_map(|walk| walk.bands.iter().chain(&walk.paths).chain(&walk.places));
        for feature in scene.features.iter().chain(walk_features) {
            if !layers_present.contains(&feature.layer) {
                layers_present.push(feature.layer);
            }
        }
    }

    ProjectView {
        id: project.id.clone(),
        name: project.name.clone(),
        modified: project.modified.clone(),
        area: project.area.clone(),
        look: project.look.clone(),
        sheets: project.sheets(),
        pins: project.pins.clone(),
        highlights: project.highlights.clone(),
        has_scene: scene.is_some(),
        fetched_at: scene.map(|scene| scene.fetched_at.clone()),
        warnings: scene
            .map(|scene| scene.warnings.clone())
            .unwrap_or_default(),
        credits: scene.map(|scene| scene.credits.clone()).unwrap_or_default(),
        layers_present,
        reports: project.reports.clone(),
        model: current_model_view(open),
    }
}

/// Saves a project that is not open through the library and remembers that
/// we did, for the watcher.
fn save_project(state: &AppState, project: &mut Project) -> Result<(), String> {
    lock(&state.own_saves).insert(project.id.clone(), Instant::now());
    state.library()?.save(project).map_err(text)?;
    lock(&state.own_saves).insert(project.id.clone(), Instant::now());
    Ok(())
}

/// Saves the open project as it is in memory and returns its new modified
/// time. Packing the scene takes most of a save, so it is packed once after
/// opening or fetching and the text is reused while the scene stays the same.
pub fn save_open_project(state: &AppState) -> Result<String, String> {
    let library = state.library()?;
    let has_newer_scene = adopt_newer_scene(state, library)?;
    let (mut project, scene, packed) = match lock(&state.open).as_ref() {
        Some(open) => (
            open.project.clone(),
            open.scene.clone(),
            open.packed_scene.clone(),
        ),
        None => return Err("No project is open.".to_string()),
    };
    let packed = match (scene, packed) {
        (Some(scene), None) => Some(pack_and_keep(state, &scene)?),
        (_, packed) => packed,
    };

    // A save that took in someone else's fetch is left for the watcher to
    // report, so the window reloads and draws the new data.
    if !has_newer_scene {
        lock(&state.own_saves).insert(project.id.clone(), Instant::now());
    }
    match &packed {
        Some(packed) => library.save_with_packed_scene(&mut project, packed),
        None => library.save(&mut project),
    }
    .map_err(text)?;
    if !has_newer_scene {
        lock(&state.own_saves).insert(project.id.clone(), Instant::now());
    }

    if let Some(open) = lock(&state.open).as_mut() {
        if open.project.id == project.id {
            open.project.modified = project.modified.clone();
        }
    }
    Ok(project.modified)
}

/// An assistant may fetch the open site again while the app holds the old
/// scene. The app's next save would write that old scene back over the new
/// one, so when the file on disk was written since the app last read or saved
/// it and carries a later fetch, its scene replaces the one in memory. The
/// user's own edits (looks, sheets, pins) still win, as before.
fn adopt_newer_scene(state: &AppState, library: &Library) -> Result<bool, String> {
    let (id, modified, fetched_at) = match lock(&state.open).as_ref() {
        Some(open) => (
            open.project.id.clone(),
            open.project.modified.clone(),
            open.scene.as_ref().map(|scene| scene.fetched_at.clone()),
        ),
        None => return Ok(false),
    };
    // `modified` is always UTC with nine fraction digits, so the strings compare like the times.
    let is_written_since = library.modified(&id).map_err(text)? > modified;
    if !is_written_since {
        return Ok(false);
    }
    let on_disk = library.load(&id).map_err(text)?;
    let Some(theirs) = on_disk.scene else {
        return Ok(false);
    };
    if Some(&theirs.fetched_at) <= fetched_at.as_ref() {
        return Ok(false);
    }

    if let Some(open) = lock(&state.open).as_mut() {
        if open.project.id == id {
            // The drawn site goes with the fetch made from it.
            open.project.area = on_disk.area;
            open.scene = Some(Arc::new(theirs));
            open.packed_scene = None;
        }
    }
    lock(&state.framed).clear();
    Ok(true)
}

/// Packs `scene` outside the lock, so previews keep rendering meanwhile.
fn pack_and_keep(state: &AppState, scene: &Arc<Scene>) -> Result<Arc<String>, String> {
    let packed = Arc::new(packed_scene::pack(scene).map_err(text)?);
    if let Some(open) = lock(&state.open).as_mut() {
        // A fetch may have replaced the scene while this one was packed.
        let is_same_scene = open
            .scene
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, scene));
        if is_same_scene {
            open.packed_scene = Some(packed.clone());
        }
    }
    Ok(packed)
}

fn open_in_state(state: &AppState, project: Project) -> ProjectView {
    let open = OpenProject::from_project(project);
    let view = view_of(&open);
    *lock(&state.open) = Some(open);
    view
}

#[tauri::command]
pub async fn library_path(state: State<'_, Shared>) -> Result<String, String> {
    let state = state.inner().clone();
    blocking(move || Ok(state.library()?.root().display().to_string())).await
}

#[tauri::command]
pub async fn list_projects(state: State<'_, Shared>) -> Result<Vec<ProjectSummary>, String> {
    let state = state.inner().clone();
    blocking(move || state.library()?.list().map_err(text)).await
}

#[tauri::command]
pub async fn list_trash(state: State<'_, Shared>) -> Result<Vec<ProjectSummary>, String> {
    let state = state.inner().clone();
    blocking(move || state.library()?.trash().map_err(text)).await
}

#[tauri::command]
pub async fn create_project(
    state: State<'_, Shared>,
    name: String,
    area: Area,
) -> Result<ProjectView, String> {
    let state = state.inner().clone();
    blocking(move || {
        let project = state.library()?.create(&name, area).map_err(text)?;
        lock(&state.own_saves).insert(project.id.clone(), Instant::now());
        Ok(open_in_state(&state, project))
    })
    .await
}

#[tauri::command]
pub async fn open_project(state: State<'_, Shared>, id: String) -> Result<ProjectView, String> {
    let state = state.inner().clone();
    blocking(move || {
        let project = state.library()?.load(&id).map_err(text)?;
        Ok(open_in_state(&state, project))
    })
    .await
}

/// Reads the open project again after something else changed it on disk.
#[tauri::command]
pub async fn reload_project(state: State<'_, Shared>) -> Result<ProjectView, String> {
    let state = state.inner().clone();
    blocking(move || {
        let id = match lock(&state.open).as_ref() {
            Some(open) => open.project.id.clone(),
            None => return Err("No project is open.".to_string()),
        };
        let project = state.library()?.load(&id).map_err(text)?;
        Ok(open_in_state(&state, project))
    })
    .await
}

#[tauri::command]
pub async fn close_project(state: State<'_, Shared>) -> Result<(), String> {
    *lock(&state.open) = None;
    Ok(())
}

#[tauri::command]
pub async fn rename_project(
    state: State<'_, Shared>,
    id: String,
    name: String,
) -> Result<(), String> {
    let state = state.inner().clone();
    blocking(move || {
        let name = name.trim().to_string();
        if name.is_empty() {
            return Err("A project needs a name.".to_string());
        }

        let is_open = match lock(&state.open).as_mut() {
            Some(open) if open.project.id == id => {
                open.project.name = name.clone();
                true
            }
            _ => false,
        };
        if is_open {
            return save_open_project(&state).map(|_| ());
        }

        let mut project = state.library()?.load(&id).map_err(text)?;
        project.name = name;
        save_project(&state, &mut project)
    })
    .await
}

#[tauri::command]
pub async fn delete_project(state: State<'_, Shared>, id: String) -> Result<(), String> {
    let state = state.inner().clone();
    blocking(move || state.library()?.delete(&id).map_err(text)).await
}

#[tauri::command]
pub async fn restore_project(state: State<'_, Shared>, id: String) -> Result<(), String> {
    let state = state.inner().clone();
    blocking(move || state.library()?.restore(&id).map_err(text)).await
}
