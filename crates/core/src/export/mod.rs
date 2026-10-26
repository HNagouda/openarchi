//! Files that leave the app. PDF and PNG come from the rendered SVG; DXF is
//! written from the scene directly so CAD users get real layers: `dxf.rs` for
//! a plan, `dxf_section.rs` for a section. Laser cutting files are written
//! from the scene too, see `laser`.

pub mod aci;
mod colour_difference;
mod convert;
mod dxf;
mod dxf_colours;
mod dxf_marks;
mod dxf_section;
mod dxf_write;
pub mod laser;
pub mod series;

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::render::{render_svg, RenderInput};
use crate::selection;
use crate::view::ViewMode;
use crate::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    Svg,
    Pdf,
    Png,
    Dxf,
}

/// Which layers a plan DXF holds. A section sheet's DXF is its section either way.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DxfLayers {
    /// The layers the sheet draws, without those the user hid, and the
    /// neighbourhood when the sheet shows it: the drawing as it looks.
    #[default]
    Shown,
    /// Every fetched layer and the whole neighbourhood, whatever the sheet shows.
    All,
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

/// Writes the sheet to `path` in `format`. PNG is rendered at `dpi`, and a
/// DXF holds the layers the sheet shows.
pub fn export(input: &RenderInput, format: ExportFormat, dpi: u32, path: &Path) -> Result<()> {
    export_with_dxf_layers(input, format, dpi, DxfLayers::default(), path)
}

/// `export`, with a DXF holding `dxf_layers`. Other formats ignore it.
pub fn export_with_dxf_layers(
    input: &RenderInput,
    format: ExportFormat,
    dpi: u32,
    dxf_layers: DxfLayers,
    path: &Path,
) -> Result<()> {
    check_extension(path, format.extension())?;
    let bytes = match format {
        ExportFormat::Svg => render_svg(input)?.into_bytes(),
        ExportFormat::Pdf => convert::pdf(&render_svg(input)?)?,
        ExportFormat::Png => convert::png_at_dpi(&render_svg(input)?, dpi)?,
        ExportFormat::Dxf => sheet_dxf(input, dxf_layers)?.into_bytes(),
    };

    std::fs::write(path, bytes).map_err(|source| Error::io(path, source))
}

/// Refuses a path that does not end in `extension`, so a call from the web
/// view can only write the kind of file it asked for, never ".zshrc".
pub fn check_extension(path: &Path, extension: &str) -> Result<()> {
    let is_match = path
        .extension()
        .is_some_and(|found| found.eq_ignore_ascii_case(extension));
    if is_match {
        Ok(())
    } else {
        Err(Error::Invalid(format!(
            "a .{extension} file can only be written to a path ending in .{extension}, got {}",
            path.display()
        )))
    }
}

/// The DXF of what the sheet shows, cut to its area like the SVG: a section
/// sheet as its section, any other view as the plan under it.
fn sheet_dxf(input: &RenderInput, dxf_layers: DxfLayers) -> Result<String> {
    let look = input.look;
    let drawn = input.diagram.drawn_as(look.variant.as_deref());
    let scene = selection::framed_or_whole(input.scene, look.selection.as_ref())?;
    // The same classes as the sheet, so the DXF's class layers match its legend.
    let drawn = crate::radiation::sited(&drawn, &scene).unwrap_or(drawn);
    let input = &RenderInput {
        diagram: &drawn,
        scene: &scene,
        ..*input
    };
    match look.view.mode {
        ViewMode::Section => dxf_section::write(input),
        _ => dxf::write(input, dxf_layers),
    }
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
