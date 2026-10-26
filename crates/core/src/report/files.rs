//! Writing a report to disk: one PDF of every page, or one SVG or PNG per page.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::compose::pages_svg;
use super::data::ReportData;
use super::Report;
use crate::export::{check_extension, svg_pages_to_pdf, svg_to_png};
use crate::{Error, Result};

const MM_PER_INCH: f64 = 25.4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportFormat {
    Pdf,
    Svg,
    Png,
}

impl ReportFormat {
    pub fn extension(self) -> &'static str {
        match self {
            ReportFormat::Pdf => "pdf",
            ReportFormat::Svg => "svg",
            ReportFormat::Png => "png",
        }
    }
}

/// Writes `report` and returns the files written. A PDF holds every page,
/// titled with the report's name. SVG and PNG write one file per page next
/// to `path`, "westermarkt-report-1.svg" and on; a one-page report is `path` itself.
pub fn export(
    report: &Report,
    data: &ReportData,
    format: ReportFormat,
    dpi: u32,
    path: &Path,
) -> Result<Vec<PathBuf>> {
    report.check()?;
    check_extension(path, format.extension())?;
    let pages = pages_svg(report, data)?;
    if format == ReportFormat::Pdf {
        write(path, &svg_pages_to_pdf(&pages, &report.name)?)?;
        return Ok(vec![path.to_path_buf()]);
    }

    let targets = output_paths(report, format, path);
    for ((index, svg), target) in pages.iter().enumerate().zip(&targets) {
        let bytes = match format {
            ReportFormat::Png => {
                let (width_mm, _) = report.pages[index].size_mm();
                let width_px = (width_mm / MM_PER_INCH * f64::from(dpi)).round() as u32;
                svg_to_png(svg, width_px)?
            }
            _ => svg.clone().into_bytes(),
        };
        write(target, &bytes)?;
    }

    Ok(targets)
}

/// The files `export` writes for `report` at `path`, so a caller can check
/// them before anything is written.
pub fn output_paths(report: &Report, format: ReportFormat, path: &Path) -> Vec<PathBuf> {
    if format == ReportFormat::Pdf || report.pages.len() == 1 {
        return vec![path.to_path_buf()];
    }
    (1..=report.pages.len())
        .map(|number| numbered(path, number, format))
        .collect()
}

/// "report.svg" and 2 -> "report-2.svg".
fn numbered(path: &Path, number: usize, format: ReportFormat) -> PathBuf {
    let stem = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_default();
    path.with_file_name(format!("{stem}-{number}.{}", format.extension()))
}

fn write(path: &Path, bytes: &[u8]) -> Result<()> {
    std::fs::write(path, bytes).map_err(|source| Error::io(path, source))
}
