//! Reports of the open project: saving them, drawing a block or a page for
//! the report editor, and exporting a whole report.

use std::path::PathBuf;

use openarchi_core::catalog;
use openarchi_core::export::series::slug;
use openarchi_core::highlight::Highlight;
use openarchi_core::pin::Pin;
use openarchi_core::project::Look;
use openarchi_core::report::{
    self, backdrop_svg, block_svg, Block, Content, Report, ReportData, ReportFormat,
};
use openarchi_core::theme::Theme;
use tauri::State;

use crate::commands::library::save_open_project;
use crate::commands::ratios::find_theme;
use crate::commands::render::open_scene;
use crate::guard::{blocking, text};
use crate::state::{lock, AppState, Shared};

/// Saves every report of the open project and returns the new modified time.
#[tauri::command]
pub async fn save_reports(
    state: State<'_, Shared>,
    reports: Vec<Report>,
) -> Result<String, String> {
    let state = state.inner().clone();
    blocking(move || {
        for report in &reports {
            report.check().map_err(text)?;
        }
        match lock(&state.open).as_mut() {
            Some(open) => {
                open.project.reports = reports;
            }
            None => return Err("No project is open.".to_string()),
        }
        save_open_project(&state)
    })
    .await
}

/// One block on its own, drawn in `theme_id`, for the editor to place on its page.
#[tauri::command]
pub async fn render_report_block(
    state: State<'_, Shared>,
    theme_id: String,
    block: Block,
) -> Result<String, String> {
    let state = state.inner().clone();
    blocking(move || {
        let theme = find_theme(&state, &theme_id)?;
        with_data(&state, &theme, [&block.content], |data| {
            block_svg(&block, data)
        })
    })
    .await
}

/// Page `index` of `report` with nothing on it but its background and page number.
#[tauri::command]
pub async fn render_report_backdrop(
    state: State<'_, Shared>,
    report: Report,
    index: usize,
) -> Result<String, String> {
    let state = state.inner().clone();
    blocking(move || {
        let theme = report_theme(&state, &report)?;
        backdrop_svg(&report, index, &theme).map_err(text)
    })
    .await
}

/// The suggested name in the save dialog: "westermarkt-site-report.pdf".
#[tauri::command]
pub fn report_file_name(report_name: String, format: ReportFormat) -> String {
    format!("{}.{}", slug(&report_name), format.extension())
}

/// Writes `report` as one PDF, or as an SVG or PNG per page next to `path`.
/// Returns the files written.
#[tauri::command]
pub async fn export_report(
    state: State<'_, Shared>,
    report: Report,
    format: ReportFormat,
    dpi: u32,
    path: String,
) -> Result<Vec<String>, String> {
    let state = state.inner().clone();
    blocking(move || {
        let theme = report_theme(&state, &report)?;
        let contents = report
            .pages
            .iter()
            .flat_map(|page| page.blocks.iter().map(|block| &block.content));
        let written = with_data(&state, &theme, contents, |data| {
            report::export(&report, data, format, dpi, &PathBuf::from(&path))
        })?;
        Ok(written
            .iter()
            .map(|file| file.display().to_string())
            .collect())
    })
    .await
}

/// The report's own palette, or the open project's when it has none.
fn report_theme(state: &AppState, report: &Report) -> Result<Theme, String> {
    let theme_id = match &report.theme_id {
        Some(id) => id.clone(),
        None => open_look(state)?.theme_id,
    };
    find_theme(state, &theme_id)
}

/// Gathers what `contents` need from the open project and hands it to `work`.
fn with_data<'c, T>(
    state: &AppState,
    theme: &Theme,
    contents: impl IntoIterator<Item = &'c Content> + Clone,
    work: impl FnOnce(&ReportData) -> openarchi_core::Result<T>,
) -> Result<T, String> {
    let scene = open_scene(state)?;
    let look = open_look(state)?;
    let (pins, highlights) = open_marks(state)?;
    let diagrams = catalog::builtin_diagrams();
    let cache = state.library()?.tile_cache();
    let data = ReportData::gather(
        &scene,
        &look,
        &pins,
        &highlights,
        &diagrams,
        theme,
        &cache,
        contents,
    )
    .map_err(text)?;
    work(&data).map_err(text)
}

fn open_look(state: &AppState) -> Result<Look, String> {
    lock(&state.open)
        .as_ref()
        .map(|open| open.project.look.clone())
        .ok_or_else(|| "No project is open.".to_string())
}

/// The open project's pins and highlights.
fn open_marks(state: &AppState) -> Result<(Vec<Pin>, Vec<Highlight>), String> {
    lock(&state.open)
        .as_ref()
        .map(|open| (open.project.pins.clone(), open.project.highlights.clone()))
        .ok_or_else(|| "No project is open.".to_string())
}
