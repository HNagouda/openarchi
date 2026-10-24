//! The palette editor: importing swatches, checking a draft, previewing the
//! open sheet in it, and saving, renaming, deleting and restoring the user's
//! own palettes. The rules all live in `openarchi_core::palette`.

use openarchi_core::palette::{self, PaletteCheck, PaletteDraft, Swatch};
use openarchi_core::project::Look;
use openarchi_core::render::{render_svg, RenderInput};
use openarchi_core::theme::Theme;
use tauri::State;

use crate::commands::render::{current_catalog, shown_scene};
use crate::guard::{blocking, text};
use crate::state::{lock, AppState, Shared};

/// The theme id a draft is checked and previewed under before it has its own.
const DRAFT_ID: &str = "draft";

/// The swatches in pasted text or a dropped file's contents.
#[tauri::command]
pub fn import_palette(text: String) -> Result<Vec<Swatch>, String> {
    palette::parse(&text).map_err(self::text)
}

/// The theme a draft becomes, how each role was filled, and its contrast warnings.
#[tauri::command]
pub async fn check_palette(
    state: State<'_, Shared>,
    draft: PaletteDraft,
) -> Result<PaletteCheck, String> {
    let state = state.inner().clone();
    blocking(move || checked(&state, &draft, DRAFT_ID)).await
}

/// The open sheet drawn in the draft's theme, for the editor's live preview.
#[tauri::command]
pub async fn render_palette_preview(
    state: State<'_, Shared>,
    look: Look,
    draft: PaletteDraft,
) -> Result<String, String> {
    let state = state.inner().clone();
    blocking(move || {
        let scene = shown_scene(&state, look.selection.as_ref())?;
        // The scene is already cut to the sheet area.
        let look = Look {
            selection: None,
            ..look
        };
        let catalog = current_catalog(&state);
        let theme = checked(&state, &draft, DRAFT_ID)?.theme;
        // The editor is modal, so the saved pins are the ones on the sheet.
        let pins = match lock(&state.open).as_ref() {
            Some(open) => open.project.pins.clone(),
            None => Vec::new(),
        };
        let Some(diagram) = catalog
            .diagrams
            .iter()
            .find(|diagram| diagram.id == look.diagram_id)
        else {
            return Err(format!("There is no diagram called “{}”.", look.diagram_id));
        };
        let input = RenderInput {
            scene: &scene,
            diagram,
            theme: &theme,
            look: &look,
            pins: &pins,
        };
        render_svg(&input).map_err(text)
    })
    .await
}

/// A draft of an existing palette: to edit it when it is the user's own, or
/// to start a copy of it under `name`.
#[tauri::command]
pub async fn palette_draft(
    state: State<'_, Shared>,
    theme_id: String,
    name: String,
) -> Result<PaletteDraft, String> {
    let state = state.inner().clone();
    blocking(move || Ok(palette::draft_from(&find(&state, &theme_id)?, &name))).await
}

/// Saves a draft to the user's themes. `id` is the palette being edited; a
/// new palette gets a free id made from its name. Returns the saved theme.
#[tauri::command]
pub async fn save_palette(
    state: State<'_, Shared>,
    draft: PaletteDraft,
    id: Option<String>,
) -> Result<Theme, String> {
    let state = state.inner().clone();
    blocking(move || {
        let library = state.library()?;
        let id = id.unwrap_or_else(|| library.free_theme_id(&draft.name));
        let theme = checked(&state, &draft, &id)?.theme;
        library.save_theme(&theme).map_err(text)?;
        forget_catalog(&state);
        Ok(theme)
    })
    .await
}

#[tauri::command]
pub async fn rename_palette(
    state: State<'_, Shared>,
    id: String,
    name: String,
) -> Result<(), String> {
    let state = state.inner().clone();
    blocking(move || {
        let name = name.trim().to_string();
        if name.is_empty() {
            return Err("A palette needs a name.".to_string());
        }
        let theme = Theme {
            name,
            ..find(&state, &id)?
        };
        state.library()?.save_theme(&theme).map_err(text)?;
        forget_catalog(&state);
        Ok(())
    })
    .await
}

/// Moves a palette to Recently Deleted. Returns the name `restore_palette` takes.
#[tauri::command]
pub async fn delete_palette(state: State<'_, Shared>, id: String) -> Result<String, String> {
    let state = state.inner().clone();
    blocking(move || {
        let kept = state.library()?.delete_theme(&id).map_err(text)?;
        forget_catalog(&state);
        Ok(kept)
    })
    .await
}

#[tauri::command]
pub async fn restore_palette(state: State<'_, Shared>, kept: String) -> Result<Theme, String> {
    let state = state.inner().clone();
    blocking(move || {
        let theme = state.library()?.restore_theme(&kept).map_err(text)?;
        forget_catalog(&state);
        Ok(theme)
    })
    .await
}

/// Checks a draft, as a copy of the theme it was duplicated from when it names one.
fn checked(state: &AppState, draft: &PaletteDraft, id: &str) -> Result<PaletteCheck, String> {
    match &draft.based_on {
        Some(base_id) => palette::check_copy(draft, id, &find(state, base_id)?),
        None => palette::check(draft, id),
    }
    .map_err(text)
}

fn find(state: &AppState, id: &str) -> Result<Theme, String> {
    current_catalog(state)
        .themes
        .into_iter()
        .find(|theme| theme.id == id)
        .ok_or_else(|| format!("There is no palette called “{id}”."))
}

/// The next catalog read picks the palette files up again.
fn forget_catalog(state: &AppState) {
    *lock(&state.catalog) = None;
}
