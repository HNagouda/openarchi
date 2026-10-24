//! Files that leave the app. PDF and PNG come from the rendered SVG; DXF is
//! written from the scene directly so CAD users get real layers.

mod convert;
mod dxf;

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::render::{render_svg, RenderInput};
use crate::selection;
use crate::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    Svg,
    Pdf,
    Png,
    Dxf,
}

impl ExportFormat {
    pub fn extension(self) -> &'static str {
        match self {
            ExportFormat::Svg => "svg",
            ExportFormat::Pdf => "pdf",
            ExportFormat::Png => "png",
            ExportFormat::Dxf => "dxf",
        }
    }
}

/// Writes the sheet to `path` in `format`. PNG is rendered at `dpi`.
pub fn export(input: &RenderInput, format: ExportFormat, dpi: u32, path: &Path) -> Result<()> {
    let bytes = match format {
        ExportFormat::Svg => render_svg(input)?.into_bytes(),
        ExportFormat::Pdf => convert::pdf(&render_svg(input)?)?,
        ExportFormat::Png => convert::png_at_dpi(&render_svg(input)?, dpi)?,
        ExportFormat::Dxf => sheet_dxf(input)?.into_bytes(),
    };

    std::fs::write(path, bytes).map_err(|source| Error::io(path, source))
}

/// The DXF of what the sheet shows: its sub-selection, and its pins unless it hides them.
fn sheet_dxf(input: &RenderInput) -> Result<String> {
    let look = input.look;
    let scene = selection::framed_or_whole(input.scene, look.selection.as_ref())?;
    let pins = if look.hide_pins { &[] } else { input.pins };
    Ok(dxf::write(&scene, pins))
}

/// The sheet as PNG bytes, `width_px` wide. Used for thumbnails and the MCP preview.
pub fn render_png(input: &RenderInput, width_px: u32) -> Result<Vec<u8>> {
    convert::png_at_width(&render_svg(input)?, width_px)
}

/// Any page the core drew, such as an analysis page, as PDF bytes.
pub fn svg_to_pdf(svg: &str) -> Result<Vec<u8>> {
    convert::pdf(svg)
}

/// Pages the core drew, such as the A4 analysis pages, as one PDF of that many
/// pages, titled `title` in the document's properties.
pub fn svg_pages_to_pdf(pages: &[String], title: &str) -> Result<Vec<u8>> {
    convert::pdf_pages(pages, title)
}

/// Any page the core drew as PNG bytes, `width_px` wide.
pub fn svg_to_png(svg: &str, width_px: u32) -> Result<Vec<u8>> {
    convert::png_at_width(svg, width_px)
}
