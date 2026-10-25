//! The site analysis of the open project, for the inspector's Analysis tab and its export.

use std::path::Path;
use std::sync::Arc;

use openarchi_core::analysis::{self, report_pages, report_svg, section_svg, Part, Section, PARTS};
use openarchi_core::area::Area;
use openarchi_core::export::series::slug;
use openarchi_core::export::{svg_pages_to_pdf, svg_to_pdf};
use openarchi_core::scene::Scene;
use openarchi_core::theme::Theme;
use tauri::State;

use crate::commands::ratios::find_theme;
use crate::commands::render::{open_scene, shown_scene};
use crate::guard::{blocking, text};
use crate::state::{lock, AppState, Shared};

/// One part of the analysis, drawn in `theme_id`. Climate and rain may wait
/// on PVGIS and KNMI the first time; afterwards they come from the cache.
/// With the sheet's sub-selection, the site part sums up that smaller area.
#[tauri::command]
pub async fn get_analysis(
    state: State<'_, Shared>,
    part: Part,
    theme_id: String,
    selection: Option<Area>,
) -> Result<Vec<Section>, String> {
    let state = state.inner().clone();
    blocking(move || {
        let scene = open_scene(&state)?;
        let theme = find_theme(&state, &theme_id)?;
        let cache = state.library()?.tile_cache();
        // Only the site part reads the area, so the others never wait on a cut.
        let area = match part {
            Part::Site => sheet_area(&state, selection.as_ref())?,
            _ => None,
        };
        analysis::part(part, &scene, area.as_deref(), &theme, &cache).map_err(text)
    })
    .await
}

/// The open scene cut to `selection`, from the cut scenes the sheet keeps.
fn sheet_area(state: &AppState, selection: Option<&Area>) -> Result<Option<Arc<Scene>>, String> {
    match selection {
        Some(selection) => Ok(Some(shown_scene(state, Some(selection))?)),
        None => Ok(None),
    }
}

/// The suggested name in the save dialog: "westermarkt-analysis.pdf" or "westermarkt-wind.svg".
#[tauri::command]
pub fn analysis_file_name(state: State<'_, Shared>, section_id: Option<String>) -> String {
    let name = slug(&project_name(&state));
    match section_id {
        Some(section) => format!("{name}-{section}.svg"),
        None => format!("{name}-analysis.pdf"),
    }
}

/// Writes one section, or the whole analysis when `section_id` is missing,
/// as SVG or PDF, chosen by the extension the user picked. The whole analysis
/// as PDF is laid out on A4 pages; as SVG it stays one long page.
#[tauri::command]
pub async fn export_analysis(
    state: State<'_, Shared>,
    theme_id: String,
    selection: Option<Area>,
    section_id: Option<String>,
    path: String,
) -> Result<String, String> {
    let state = state.inner().clone();
    blocking(move || {
        let scene = open_scene(&state)?;
        let area = sheet_area(&state, selection.as_ref())?;
        let theme = find_theme(&state, &theme_id)?;
        let title = project_name(&state);
        let (sections, failures) = every_section(&state, &scene, area.as_deref(), &theme)?;
        let section = match &section_id {
            Some(id) => Some(
                sections
                    .iter()
                    .find(|section| &section.id == id)
                    .ok_or_else(|| format!("There is no analysis section called “{id}”."))?,
            ),
            None => None,
        };

        let extension = Path::new(&path)
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or_default()
            .to_lowercase();
        let bytes = match (extension.as_str(), section) {
            ("svg", Some(section)) => section_svg(section, &theme, &title)
                .map_err(text)?
                .into_bytes(),
            ("svg", None) => report_svg(&sections, &failures, &theme, &title)
                .map_err(text)?
                .into_bytes(),
            ("pdf", Some(section)) => {
                svg_to_pdf(&section_svg(section, &theme, &title).map_err(text)?).map_err(text)?
            }
            ("pdf", None) => {
                let pages = report_pages(&sections, &failures, &theme, &title).map_err(text)?;
                svg_pages_to_pdf(&pages, &title).map_err(text)?
            }
            _ => {
                return Err(format!(
                    "The analysis exports as .svg or .pdf, not “{extension}”."
                ))
            }
        };
        std::fs::write(&path, bytes).map_err(|error| format!("Couldn't write {path}: {error}"))?;
        Ok(path)
    })
    .await
}

/// A part that could not be made: its title and why.
type Failure = (String, String);

/// Every section that could be made, and a failure for each part that could not.
fn every_section(
    state: &AppState,
    scene: &Scene,
    sheet_area: Option<&Scene>,
    theme: &Theme,
) -> Result<(Vec<Section>, Vec<Failure>), String> {
    let cache = state.library()?.tile_cache();
    let mut sections = Vec::new();
    let mut failures = Vec::new();
    for part in PARTS {
        match analysis::part(part, scene, sheet_area, theme, &cache) {
            Ok(found) => sections.extend(found),
            Err(error) => failures.push((part.title().to_string(), error.to_string())),
        }
    }

    Ok((sections, failures))
}

fn project_name(state: &AppState) -> String {
    lock(&state.open)
        .as_ref()
        .map(|open| open.project.name.clone())
        .unwrap_or_default()
}
