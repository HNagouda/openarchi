//! The report tools: listing a project's reports and writing one as a PDF,
//! or as an SVG or PNG per page. Reports are laid out in the app; the
//! assistant reads and exports them.

use openarchi_core::catalog;
use openarchi_core::library::Library;
use openarchi_core::report::{self, ReportData, ReportFormat};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::export::DEFAULT_EXPORT_DPI;
use crate::resolve::find_theme;
use crate::server::{checked_path, fetched_scene, text};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExportReport {
    /// The project id.
    pub id: String,
    /// The report id from list_reports, like "report-1".
    pub report_id: String,
    /// pdf (the default) writes one file with every page; svg and png write one
    /// file per page, "report-1.png", "report-2.png" and on next to `path`.
    pub format: Option<ReportFormatInput>,
    /// Absolute path of the file to write, with the format's extension. The folder must exist.
    pub path: String,
    /// Resolution for png. Defaults to 300.
    pub dpi: Option<u32>,
    /// Replace files that are already there. Defaults to false.
    pub overwrite: Option<bool>,
}

/// pdf for print, svg for layout tools, png for slides and the web.
#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReportFormatInput {
    Pdf,
    Svg,
    Png,
}

impl From<ReportFormatInput> for ReportFormat {
    fn from(format: ReportFormatInput) -> Self {
        match format {
            ReportFormatInput::Pdf => ReportFormat::Pdf,
            ReportFormatInput::Svg => ReportFormat::Svg,
            ReportFormatInput::Png => ReportFormat::Png,
        }
    }
}

/// Every report of the project, with its pages and the blocks on them.
pub fn list(library: &Library, id: &str) -> Result<Value, String> {
    let project = library.load(id).map_err(text)?;
    let reports: Vec<Value> = project
        .reports
        .iter()
        .map(|report| {
            json!({
                "id": report.id,
                "name": report.name,
                "theme_id": report.theme_id,
                "page_count": report.pages.len(),
                "pages": report.pages,
            })
        })
        .collect();

    Ok(json!({ "reports": reports }))
}

/// Writes the report as one PDF with a page per report page, or one SVG or PNG per page.
pub fn export(library: &Library, input: ExportReport) -> Result<Value, String> {
    let format = ReportFormat::from(input.format.unwrap_or(ReportFormatInput::Pdf));
    let path = checked_path(&input.path, format.extension())?;
    let project = library.load(&input.id).map_err(text)?;
    let scene = fetched_scene(&project)?;
    let Some(report) = project
        .reports
        .iter()
        .find(|report| report.id == input.report_id)
    else {
        let ids: Vec<&str> = project
            .reports
            .iter()
            .map(|report| report.id.as_str())
            .collect();
        let known = if ids.is_empty() {
            "none yet; reports are laid out in the app".to_string()
        } else {
            ids.join(", ")
        };
        return Err(format!(
            "project '{}' has no report '{}'; its reports are: {known}",
            input.id, input.report_id
        ));
    };

    let targets = report::output_paths(report, format, &path);
    if !input.overwrite.unwrap_or(false) {
        if let Some(taken) = targets.iter().find(|target| target.exists()) {
            return Err(format!(
                "{} already exists; pass overwrite: true to replace it",
                taken.display()
            ));
        }
    }

    let theme_id = report.theme_id.as_ref().unwrap_or(&project.look.theme_id);
    let theme = find_theme(&library.themes_dir(), theme_id)?;
    let diagrams = catalog::builtin_diagrams();
    let contents = report
        .pages
        .iter()
        .flat_map(|page| page.blocks.iter().map(|block| &block.content));
    let cache = library.tile_cache();
    let data = ReportData::gather(
        scene,
        &project.look,
        &project.pins,
        &project.highlights,
        project.model.as_ref(),
        &project.edits,
        &diagrams,
        &theme,
        &cache,
        contents,
    )
    .map_err(text)?;
    let dpi = input.dpi.unwrap_or(DEFAULT_EXPORT_DPI);
    let written = report::export(report, &data, format, dpi, &path).map_err(text)?;

    Ok(json!({ "path": path, "files": written, "pages": report.pages.len() }))
}
