//! The catalog, the live preview and every thumbnail. All of it comes from the
//! core renderer, so the preview is exactly what an export will contain.

use std::sync::Arc;

use base64::Engine;
use openarchi_core::catalog;
use openarchi_core::export::render_png;
use openarchi_core::pin::Pin;
use openarchi_core::project::Look;
use openarchi_core::render::{render_svg, RenderInput};
use openarchi_core::scene::{Layer, Scene};
use tauri::State;

use crate::guard::{blocking, text};
use crate::state::{lock, AppState, Catalog, LayerInfo, Shared};

fn read_catalog(state: &AppState) -> Catalog {
    let diagrams = catalog::builtin_diagrams();
    let (themes, warnings) = match state.library() {
        Ok(library) => catalog::all_themes(&library.themes_dir()),
        Err(_) => (catalog::builtin_themes(), Vec::new()),
    };
    let layers = Layer::ALL
        .iter()
        .map(|layer| LayerInfo {
            id: *layer,
            label: layer.label().to_string(),
        })
        .collect();

    Catalog {
        diagrams,
        themes,
        warnings,
        layers,
    }
}

/// The cached catalog, loaded on first use.
pub fn current_catalog(state: &AppState) -> Catalog {
    let mut cached = lock(&state.catalog);
    if cached.is_none() {
        *cached = Some(read_catalog(state));
    }
    cached.clone().expect("catalog was just loaded")
}

/// Resolves the look's diagram and theme ids and hands `work` a render input.
pub fn with_input<T>(
    catalog: &Catalog,
    scene: &Scene,
    look: &Look,
    pins: &[Pin],
    work: impl FnOnce(&RenderInput) -> openarchi_core::Result<T>,
) -> Result<T, String> {
    let Some(diagram) = catalog
        .diagrams
        .iter()
        .find(|diagram| diagram.id == look.diagram_id)
    else {
        return Err(format!("There is no diagram called “{}”.", look.diagram_id));
    };
    let Some(theme) = catalog
        .themes
        .iter()
        .find(|theme| theme.id == look.theme_id)
    else {
        return Err(format!("There is no theme called “{}”.", look.theme_id));
    };

    let input = RenderInput {
        scene,
        diagram,
        theme,
        look,
        pins,
    };
    work(&input).map_err(text)
}

pub fn open_scene(state: &AppState) -> Result<Arc<Scene>, String> {
    match lock(&state.open).as_ref() {
        Some(open) => match &open.scene {
            Some(scene) => Ok(scene.clone()),
            None => Err("This site hasn't been fetched yet.".to_string()),
        },
        None => Err("No project is open.".to_string()),
    }
}

fn png_data_url(bytes: &[u8]) -> String {
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    format!("data:image/png;base64,{encoded}")
}

/// Reads the catalog again, for example after a theme file was added.
#[tauri::command]
pub async fn get_catalog(state: State<'_, Shared>) -> Result<Catalog, String> {
    let state = state.inner().clone();
    blocking(move || {
        *lock(&state.catalog) = None;
        Ok(current_catalog(&state))
    })
    .await
}

#[tauri::command]
pub async fn render_preview(
    state: State<'_, Shared>,
    look: Look,
    pins: Vec<Pin>,
) -> Result<String, String> {
    let state = state.inner().clone();
    blocking(move || {
        let scene = open_scene(&state)?;
        let catalog = current_catalog(&state);
        with_input(&catalog, &scene, &look, &pins, render_svg)
    })
    .await
}

/// A PNG of the open project in `look`, for the diagram column. Pins would
/// be specks at this size, so they are left off.
#[tauri::command]
pub async fn render_thumbnail(
    state: State<'_, Shared>,
    look: Look,
    width: u32,
) -> Result<String, String> {
    let state = state.inner().clone();
    blocking(move || {
        let scene = open_scene(&state)?;
        let catalog = current_catalog(&state);
        let bytes = with_input(&catalog, &scene, &look, &[], |input| {
            render_png(input, width)
        })?;
        Ok(png_data_url(&bytes))
    })
    .await
}

/// A gallery card's picture. `None` while the site has not been fetched.
#[tauri::command]
pub async fn project_thumbnail(
    state: State<'_, Shared>,
    id: String,
    modified: String,
    width: u32,
) -> Result<Option<String>, String> {
    let state = state.inner().clone();
    blocking(move || {
        let key = format!("{id}|{modified}|{width}");
        if let Some(url) = lock(&state.thumbnails).get(&key) {
            return Ok(Some(url.clone()));
        }

        let project = state.library()?.load(&id).map_err(text)?;
        let Some(scene) = project.scene.as_ref() else {
            return Ok(None);
        };
        let catalog = current_catalog(&state);
        let bytes = with_input(&catalog, scene, &project.look, &project.pins, |input| {
            render_png(input, width)
        })?;
        let url = png_data_url(&bytes);
        lock(&state.thumbnails).insert(key, url.clone());
        Ok(Some(url))
    })
    .await
}
