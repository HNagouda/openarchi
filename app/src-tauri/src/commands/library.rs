//! Projects in the library: list, open, rename, delete, restore, save.

use std::time::Instant;

use openarchi_core::area::Area;
use openarchi_core::library::ProjectSummary;
use openarchi_core::project::{Look, Project};
use openarchi_core::report::Report;
use openarchi_core::scene::{Credit, Layer, Scene};
use serde::Serialize;
use tauri::State;

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
    pub has_scene: bool,
    pub fetched_at: Option<String>,
    pub warnings: Vec<String>,
    pub credits: Vec<Credit>,
    /// Layers that hold at least one feature, so the inspector lists only those.
    pub layers_present: Vec<Layer>,
    pub reports: Vec<Report>,
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
        has_scene: scene.is_some(),
        fetched_at: scene.map(|scene| scene.fetched_at.clone()),
        warnings: scene
            .map(|scene| scene.warnings.clone())
            .unwrap_or_default(),
        credits: scene.map(|scene| scene.credits.clone()).unwrap_or_default(),
        layers_present,
        reports: project.reports.clone(),
    }
}

/// Saves through the library and remembers that we did, for the watcher.
pub fn save_project(state: &AppState, project: &mut Project) -> Result<(), String> {
    lock(&state.own_saves).insert(project.id.clone(), Instant::now());
    state.library()?.save(project).map_err(text)?;
    lock(&state.own_saves).insert(project.id.clone(), Instant::now());
    Ok(())
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

/// Saves the look of the open project and returns the new modified time.
#[tauri::command]
pub async fn save_look(state: State<'_, Shared>, look: Look) -> Result<String, String> {
    let state = state.inner().clone();
    blocking(move || {
        let mut project = match lock(&state.open).as_mut() {
            Some(open) => {
                open.project.look = look;
                open.to_project()
            }
            None => return Err("No project is open.".to_string()),
        };
        save_project(&state, &mut project)?;

        if let Some(open) = lock(&state.open).as_mut() {
            open.project.modified = project.modified.clone();
        }
        Ok(project.modified)
    })
    .await
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

        let open_copy = match lock(&state.open).as_mut() {
            Some(open) if open.project.id == id => {
                open.project.name = name.clone();
                Some(open.to_project())
            }
            _ => None,
        };
        let mut project = match open_copy {
            Some(project) => project,
            None => {
                let mut project = state.library()?.load(&id).map_err(text)?;
                project.name = name;
                project
            }
        };
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
