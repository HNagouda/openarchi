//! The site analysis of the open project, for the inspector's Analysis tab and its export.

use std::path::Path;

use openarchi_core::analysis::{self, report_pages, report_svg, section_svg, Part, Section, PARTS};
use openarchi_core::export::{svg_pages_to_pdf, svg_to_pdf};
use openarchi_core::scene::Scene;
use openarchi_core::theme::Theme;
use tauri::State;

use crate::commands::export::slug;
use crate::commands::ratios::find_theme;
use crate::commands::render::open_scene;
use crate::guard::{blocking, text};
use crate::state::{lock, AppState, Shared};

/// One part of the analysis, drawn in `theme_id`. Climate and rain may wait
/// on PVGIS and KNMI the first time; afterwards they come from the cache.
#[tauri::command]
pub async fn get_analysis(
    state: State<'_, Shared>,
    part: Part,
    theme_id: String,
) -> Result<Vec<Section>, String> {
    let state = state.inner().clone();
    blocking(move || {
        let scene = open_scene(&state)?;
        let theme = find_theme(&state, &theme_id)?;
        let cache = state.library()?.tile_cache();
        analysis::part(part, &scene, &theme, &cache).map_err(text)
    })
    .await
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
    section_id: Option<String>,
    path: String,
) -> Result<String, String> {
    let state = state.inner().clone();
    blocking(move || {
        let scene = open_scene(&state)?;
        let theme = find_theme(&state, &theme_id)?;
        let title = project_name(&state);
        let (sections, failures) = every_section(&state, &scene, &theme)?;
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
    theme: &Theme,
) -> Result<(Vec<Section>, Vec<Failure>), String> {
    let cache = state.library()?.tile_cache();
    let mut sections = Vec::new();
    let mut failures = Vec::new();
    for part in PARTS {
        match analysis::part(part, scene, theme, &cache) {
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
