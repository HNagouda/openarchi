//! The report tools: listing a project's reports and writing one as a PDF.
//! Reports are laid out in the app; the assistant reads and exports them.

use std::path::PathBuf;

use openarchi_core::catalog;
use openarchi_core::library::Library;
use openarchi_core::report::{self, ReportData, ReportFormat};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::resolve::find_theme;

/// PDF pages are vector; only raster images inside them would use this.
const PDF_DPI: u32 = 300;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExportReport {
    /// The project id.
    pub id: String,
    /// The report id from list_reports, like "report-1".
    pub report_id: String,
    /// Absolute path of the PDF to write, including ".pdf". The folder must exist.
    pub path: String,
    /// Replace a file that is already at `path`. Defaults to false.
    pub overwrite: Option<bool>,
}

/// Every report of the project, with its pages and the blocks on them.
pub fn list(library: &Library, id: &str) -> Result<Value, String> {
    let project = library.load(id).map_err(|error| error.to_string())?;
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

/// Writes the report as one PDF with a page per report page.
pub fn export_pdf(library: &Library, input: ExportReport) -> Result<Value, String> {
    let path = checked_path(&input)?;
    let project = library.load(&input.id).map_err(|error| error.to_string())?;
    let Some(scene) = &project.scene else {
        return Err(format!(
            "project '{}' has no site data yet; call fetch_site first",
            input.id
        ));
    };
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
        &diagrams,
        &theme,
        &cache,
        contents,
    )
    .map_err(|error| error.to_string())?;
    report::export(report, &data, ReportFormat::Pdf, PDF_DPI, &path)
        .map_err(|error| error.to_string())?;

    Ok(json!({ "path": path, "pages": report.pages.len() }))
}

/// An absolute path to a PDF in a folder that exists, not over a file unless asked.
fn checked_path(input: &ExportReport) -> Result<PathBuf, String> {
    let path = PathBuf::from(&input.path);
    if !path.is_absolute() {
        return Err(format!("path must be absolute, got \"{}\"", input.path));
    }
    let is_pdf = path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"));
    if !is_pdf {
        return Err(format!("path must end in .pdf, got \"{}\"", input.path));
    }
    if path.exists() && !input.overwrite.unwrap_or(false) {
        return Err(format!(
            "{} already exists; pass overwrite: true to replace it",
            path.display()
        ));
    }
    if !path.parent().is_some_and(|folder| folder.is_dir()) {
        return Err(format!("the folder for {} does not exist", path.display()));
    }

    Ok(path)
}
