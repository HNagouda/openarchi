//! The catalog, the live preview and every thumbnail. All of it comes from the
//! core renderer, so the preview is exactly what an export will contain.

use std::sync::Arc;

use base64::Engine;
use openarchi_core::area::Area;
use openarchi_core::catalog;
use openarchi_core::export::render_png;
use openarchi_core::highlight::Highlight;
use openarchi_core::pin::Pin;
use openarchi_core::project::Look;
use openarchi_core::render::{render_svg, RenderInput};
use openarchi_core::scene::{Layer, Scene};
use openarchi_core::selection;
use openarchi_core::theme::Theme;
use tauri::State;

use crate::guard::{blocking, text};
use crate::state::{lock, AppState, Catalog, Framed, LayerInfo, Shared, Thumbnail};

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
pub fn current_catalog(state: &AppState) -> Arc<Catalog> {
    let mut cached = lock(&state.catalog);
    cached
        .get_or_insert_with(|| Arc::new(read_catalog(state)))
        .clone()
}

/// A theme of the catalog by id, as a copy the caller can keep.
pub fn find_theme(state: &AppState, id: &str) -> Result<Theme, String> {
    current_catalog(state)
        .themes
        .iter()
        .find(|theme| theme.id == id)
        .cloned()
        .ok_or_else(|| format!("There is no palette called “{id}”."))
}

/// The open project's name, empty when none is open.
pub fn project_name(state: &AppState) -> String {
    lock(&state.open)
        .as_ref()
        .map(|open| open.project.name.clone())
        .unwrap_or_default()
}

/// Resolves the look's diagram and theme ids and hands `work` a render input.
/// `draft` is a theme to draw in instead of the look's, such as the palette editor's.
pub fn with_input<T>(
    catalog: &Catalog,
    scene: &Scene,
    look: &Look,
    draft: Option<&Theme>,
    pins: &[Pin],
    highlights: &[Highlight],
    work: impl FnOnce(&RenderInput) -> openarchi_core::Result<T>,
) -> Result<T, String> {
    let Some(diagram) = catalog
        .diagrams
        .iter()
        .find(|diagram| diagram.id == look.diagram_id)
    else {
        return Err(format!("There is no diagram called “{}”.", look.diagram_id));
    };
    let Some(theme) = draft.or_else(|| {
        catalog
            .themes
            .iter()
            .find(|theme| theme.id == look.theme_id)
    }) else {
        return Err(format!("There is no theme called “{}”.", look.theme_id));
    };

    let input = RenderInput {
        scene,
        diagram,
        theme,
        look,
        pins,
        highlights,
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

/// Enough for every sheet area of a few tabs at once.
const FRAMED_KEPT: usize = 4;

/// The open scene as a sheet shows it: cut to `selection` when there is one.
/// Cut scenes are kept, so a render with the same area does not cut again.
pub fn shown_scene(state: &AppState, selection: Option<&Area>) -> Result<Arc<Scene>, String> {
    let scene = open_scene(state)?;
    let Some(selection) = selection else {
        return Ok(scene);
    };
    let is_match =
        |framed: &Framed| Arc::ptr_eq(&framed.source, &scene) && &framed.selection == selection;
    if let Some(framed) = lock(&state.framed).iter().find(|framed| is_match(framed)) {
        return Ok(framed.scene.clone());
    }

    let cut = Arc::new(selection::framed(&scene, selection).map_err(text)?);
    let mut kept = lock(&state.framed);
    // Cuts of a scene that is no longer open only hold memory.
    kept.retain(|framed| Arc::ptr_eq(&framed.source, &scene));
    kept.push(Framed {
        source: scene,
        selection: selection.clone(),
        scene: cut.clone(),
    });
    if kept.len() > FRAMED_KEPT {
        kept.remove(0);
    }
    Ok(cut)
}

/// `look` drawn on the scene its sheet shows, in `draft` when one is given.
/// The look's own selection is dropped, since the scene is already cut to it.
pub fn render_shown<T>(
    state: &AppState,
    look: &Look,
    draft: Option<&Theme>,
    pins: &[Pin],
    highlights: &[Highlight],
    work: impl FnOnce(&RenderInput) -> openarchi_core::Result<T>,
) -> Result<T, String> {
    let scene = shown_scene(state, look.selection.as_ref())?;
    let catalog = current_catalog(state);
    let whole = Look {
        selection: None,
        ..look.clone()
    };
    with_input(&catalog, &scene, &whole, draft, pins, highlights, work)
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
        Ok(Catalog::clone(&current_catalog(&state)))
    })
    .await
}

#[tauri::command]
pub async fn render_preview(
    state: State<'_, Shared>,
    look: Look,
    pins: Vec<Pin>,
    highlights: Vec<Highlight>,
) -> Result<String, String> {
    let state = state.inner().clone();
    blocking(move || render_shown(&state, &look, None, &pins, &highlights, render_svg)).await
}

/// A PNG of the open project in `look`, for the diagram column. Pins and
/// highlights would be specks at this size, so they are left off; the pins
/// still go in, hidden, so a View eye standing on one stands there.
#[tauri::command]
pub async fn render_thumbnail(
    state: State<'_, Shared>,
    look: Look,
    width: u32,
) -> Result<String, String> {
    let state = state.inner().clone();
    blocking(move || {
        let pins = match lock(&state.open).as_ref() {
            Some(open) => open.project.pins.clone(),
            None => Vec::new(),
        };
        let look = Look {
            hide_pins: true,
            ..look
        };
        let bytes = render_shown(&state, &look, None, &pins, &[], |input| {
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
        if let Some(kept) = lock(&state.thumbnails).get(&id) {
            if kept.modified == modified && kept.width == width {
                return Ok(Some(kept.url.clone()));
            }
        }

        let project = state.library()?.load(&id).map_err(text)?;
        let Some(scene) = project.scene.as_ref() else {
            return Ok(None);
        };
        let catalog = current_catalog(&state);
        let bytes = with_input(
            &catalog,
            scene,
            &project.look,
            None,
            &project.pins,
            &project.highlights,
            |input| render_png(input, width),
        )?;
        let url = png_data_url(&bytes);
        let kept = Thumbnail {
            modified,
            width,
            url: url.clone(),
        };
        lock(&state.thumbnails).insert(id, kept);
        Ok(Some(url))
    })
    .await
}
