//! What render_preview and export accept.

use openarchi_core::export::{DxfLayers, ExportFormat};
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RenderPreview {
    /// The project id.
    pub id: String,
    /// Width of the PNG in pixels. Defaults to 1200, between 200 and 4000.
    pub width_px: Option<u32>,
    /// Which sheet tab, counting from 1 for the first, which is the default. list_sheets shows them.
    pub tab: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct Export {
    /// The project id.
    pub id: String,
    pub format: FormatInput,
    /// Absolute path of the file to write, including the extension. The folder must exist. Needed unless series is true.
    pub path: Option<String>,
    /// Resolution for png. Defaults to 300.
    pub dpi: Option<u32>,
    /// Replace files that are already there. Defaults to false.
    pub overwrite: Option<bool>,
    /// Write the whole series instead of this one diagram: one file per diagram and per variant, in this sheet's palette and paper, named "<project>-<diagram>[-<variant>].<format>". Defaults to false.
    pub series: Option<bool>,
    /// Absolute path of an existing folder for the series. Needed when series is true.
    pub folder: Option<String>,
    /// Which sheet tab, counting from 1 for the first, which is the default. list_sheets shows them.
    pub tab: Option<usize>,
    /// Which layers a plan's dxf holds: "shown" only the layers the sheet draws, without those hidden (the default, as in the app), or "all" every fetched layer and the whole neighbourhood. Other formats and a section sheet's dxf ignore it.
    pub dxf_layers: Option<DxfLayersInput>,
}

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DxfLayersInput {
    Shown,
    All,
}

impl From<DxfLayersInput> for DxfLayers {
    fn from(layers: DxfLayersInput) -> Self {
        match layers {
            DxfLayersInput::Shown => DxfLayers::Shown,
            DxfLayersInput::All => DxfLayers::All,
        }
    }
}

/// svg and pdf for print and layout tools, png for slides and the web, dxf for
/// CAD with real layers: a plan in RD New metres, or for a section sheet the
/// section in metres along the line and metres NAP.
#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FormatInput {
    Svg,
    Pdf,
    Png,
    Dxf,
}

impl From<FormatInput> for ExportFormat {
    fn from(format: FormatInput) -> Self {
        match format {
            FormatInput::Svg => ExportFormat::Svg,
            FormatInput::Pdf => ExportFormat::Pdf,
            FormatInput::Png => ExportFormat::Png,
            FormatInput::Dxf => ExportFormat::Dxf,
        }
    }
}
