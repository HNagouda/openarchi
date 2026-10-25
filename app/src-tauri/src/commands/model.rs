//! The imported model: reading a file, choosing which model is in effect and
//! where it stands, and the buildings under it. Each is a thin call into
//! `openarchi_core::model`.
//!
//! The sheet's undo history holds the model in effect as a key and its
//! placement, so the app keeps every model of the session by key (see
//! `OpenProject::models`) and only switches between them.

use std::path::PathBuf;
use std::sync::Arc;

use openarchi_core::model::{self, Covered, Model, Placement, Unit, UpAxis};
use openarchi_core::scene::Scene;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::commands::render::open_scene;
use crate::guard::{blocking, text};
use crate::state::{lock, model_key, AppState, OpenProject, Shared};

/// What the model panel and the plan need to know about a model.
#[derive(Serialize)]
pub struct ModelView {
    pub key: String,
    pub file_name: String,
    pub unit: Unit,
    pub up: UpAxis,
    /// Width, depth and height in metres.
    pub size_m: [f64; 3],
    pub triangles: usize,
    pub placement: Placement,
    pub replaced: Vec<String>,
    /// The outline of its footprint in its own frame, metres around its
    /// middle, for showing it while it is dragged.
    pub footprint: Vec<Vec<[f64; 2]>>,
    /// The terrain under it, in metres above NAP, when the scene has terrain.
    pub ground_m: Option<f64>,
}

/// The model in effect, as the sheet's undo history keeps it.
#[derive(Deserialize)]
pub struct ModelChoice {
    pub key: String,
    pub placement: Placement,
    pub replaced: Vec<String>,
}

pub fn view_of_model(key: String, model: &Model, scene: Option<&Scene>) -> ModelView {
    let footprint = model
        .shape()
        .footprint
        .iter()
        .map(|polygon| polygon.exterior().0.iter().map(|c| [c.x, c.y]).collect())
        .collect();
    ModelView {
        key,
        file_name: model.file_name.clone(),
        unit: model.unit,
        up: model.up,
        size_m: model.mesh.size(),
        triangles: model.mesh.triangles.len(),
        placement: model.placement.clone(),
        replaced: model.replaced.clone(),
        footprint,
        ground_m: scene.and_then(|scene| model::ground_m(model, scene)),
    }
}

/// The model in effect, with the key it is kept under.
pub fn current_model_view(open: &OpenProject) -> Option<ModelView> {
    let current = open.project.model.as_ref()?;
    let (key, _) = open
        .models
        .iter()
        .find(|(_, kept)| Arc::ptr_eq(&kept.mesh, &current.mesh))?;
    Some(view_of_model(key.clone(), current, open.scene.as_deref()))
}

/// Reads a model file and keeps it under a new key. It takes effect when the
/// sheet chooses it with `set_model`, so the import is one undoable step.
#[tauri::command]
pub async fn import_model(
    state: State<'_, Shared>,
    path: String,
    unit: Option<Unit>,
    up: Option<UpAxis>,
    placement: Option<Placement>,
) -> Result<ModelView, String> {
    let state = state.inner().clone();
    blocking(move || {
        let scene = open_scene(&state)?;
        let model =
            model::import(&PathBuf::from(&path), unit, up, &scene, placement).map_err(text)?;
        keep(&state, model, &scene)
    })
    .await
}

/// The model with `key` read in another unit or with another axis up, kept
/// under a new key like an import.
#[tauri::command]
pub async fn reread_model(
    state: State<'_, Shared>,
    key: String,
    unit: Unit,
    up: UpAxis,
) -> Result<ModelView, String> {
    let state = state.inner().clone();
    blocking(move || {
        let scene = open_scene(&state)?;
        let kept = {
            let open = lock(&state.open);
            let open = open.as_ref().ok_or("No project is open.")?;
            kept(open, &key)?.clone()
        };
        keep(&state, kept.reread(unit, up), &scene)
    })
    .await
}

/// Keeps `model` under the next key and describes it. Its faces are worked
/// out before the lock is taken, so previews keep drawing meanwhile.
fn keep(state: &AppState, model: Model, scene: &Scene) -> Result<ModelView, String> {
    model.shape();
    let mut open = lock(&state.open);
    let open = open.as_mut().ok_or("No project is open.")?;
    let key = model_key(open.models.len());
    let shown = view_of_model(key.clone(), &model, Some(scene));
    open.models.push((key, model));
    Ok(shown)
}

fn kept<'a>(open: &'a OpenProject, key: &str) -> Result<&'a Model, String> {
    open.models
        .iter()
        .find(|(kept_key, _)| kept_key == key)
        .map(|(_, model)| model)
        .ok_or_else(|| format!("There is no model “{key}” in this project."))
}

/// Puts the model with `choice.key` in effect where `choice` places it, or
/// takes the model away with `None`. Saved with the next save of the sheets.
#[tauri::command]
pub async fn set_model(
    state: State<'_, Shared>,
    choice: Option<ModelChoice>,
) -> Result<(), String> {
    let state = state.inner().clone();
    blocking(move || {
        let mut open = lock(&state.open);
        let open = open.as_mut().ok_or("No project is open.")?;
        open.project.model = match choice {
            Some(choice) => Some(chosen(open, choice)?),
            None => None,
        };
        Ok(())
    })
    .await
}

/// The buildings the model would reach over where `choice` places it, with
/// how much of each it covers, for the replaced buildings list.
#[tauri::command]
pub async fn covered_buildings(
    state: State<'_, Shared>,
    choice: ModelChoice,
) -> Result<Vec<Covered>, String> {
    let state = state.inner().clone();
    blocking(move || {
        let scene = open_scene(&state)?;
        let model = {
            let open = lock(&state.open);
            chosen(open.as_ref().ok_or("No project is open.")?, choice)?
        };
        Ok(model::covered_buildings(&model, &scene))
    })
    .await
}

fn chosen(open: &OpenProject, choice: ModelChoice) -> Result<Model, String> {
    choice.placement.check().map_err(text)?;
    let mut model = kept(open, &choice.key)?.clone();
    model.placement = choice.placement;
    model.replaced = choice.replaced;
    Ok(model)
}
