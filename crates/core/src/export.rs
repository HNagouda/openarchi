//! Files that leave the app. PDF and PNG come from the rendered SVG; DXF is
//! written from the scene directly so CAD users get real layers.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::render::RenderInput;
use crate::Result;

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
    let _ = (input, format, dpi, path);
    todo!("render layer")
}

/// The sheet as PNG bytes, `width_px` wide. Used for thumbnails and the MCP preview.
pub fn render_png(input: &RenderInput, width_px: u32) -> Result<Vec<u8>> {
    let _ = (input, width_px);
    todo!("render layer")
}
