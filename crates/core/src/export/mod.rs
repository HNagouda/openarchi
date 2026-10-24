//! Files that leave the app. PDF and PNG come from the rendered SVG; DXF is
//! written from the scene directly so CAD users get real layers.

mod convert;
mod dxf;

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::render::{render_svg, RenderInput};
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
        ExportFormat::Dxf => dxf::write(input.scene).into_bytes(),
    };

    std::fs::write(path, bytes).map_err(|source| Error::io(path, source))
}

/// The sheet as PNG bytes, `width_px` wide. Used for thumbnails and the MCP preview.
pub fn render_png(input: &RenderInput, width_px: u32) -> Result<Vec<u8>> {
    convert::png_at_width(&render_svg(input)?, width_px)
}
