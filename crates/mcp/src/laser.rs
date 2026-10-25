//! The laser cutting tool: the same files the app's export dialog writes for
//! a physical site model, with the same settings and the same defaults.

use std::path::PathBuf;

use openarchi_core::export::laser::{
    self, LaserFormat, LaserSettings, DEFAULT_BED_MM, DEFAULT_THICKNESS_MM,
};
use openarchi_core::export::series::slug;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

use crate::server::{reply, text, OpenArchi};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExportLaser {
    /// The project id.
    pub id: String,
    /// Absolute path of an existing folder; the files are named "<project>-laser-1.svg" and on. Needed unless check_only is true.
    pub folder: Option<String>,
    /// Which sheet tab, counting from 1 for the first, which is the default. A tab opened from a highlight makes a model of that area only.
    pub tab: Option<usize>,
    /// The model's scale, 1000 for 1:1000. Defaults to the sheet's scale when it is 1:200, 1:500, 1:1000, 1:2000 or 1:2500, else 1:1000.
    pub scale: Option<u32>,
    /// Board thickness in millimetres; one board per this much model height. Defaults to 2 (grey board); 3 is common MDF.
    pub thickness_mm: Option<f64>,
    /// The laser bed's width in millimetres. Defaults to 600.
    pub bed_width_mm: Option<f64>,
    /// The laser bed's height in millimetres. Defaults to 400.
    pub bed_height_mm: Option<f64>,
    /// Cut the base plate, with roads, water, green and footprints scored on it. Defaults to true.
    pub base_plate: Option<bool>,
    /// Cut the buildings as stacked pieces. Defaults to true.
    pub buildings: Option<bool>,
    /// Score terrain contour lines on the base plate, when the site has them. Defaults to true.
    pub contours: Option<bool>,
    /// svg (the default) or dxf with CUT and ENGRAVE layers.
    pub format: Option<LaserFormatInput>,
    /// Only report what the files would hold, writing nothing. Defaults to false.
    pub check_only: Option<bool>,
    /// Replace files that are already there. Defaults to false.
    pub overwrite: Option<bool>,
}

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LaserFormatInput {
    Svg,
    Dxf,
}

#[tool_router(router = laser_tools, vis = "pub(crate)")]
impl OpenArchi {
    #[tool(
        description = "Write laser cutting files for a physical site model of a sheet: one file per laser bed, in real millimetres, cut lines as red (#FF0000) hairlines and score lines as blue (#0000FF) hairlines. The base plate is the site (or the highlight the sheet was opened from) with roads, water, green and footprints scored on it, cut into tiles when it is larger than the bed; each building is a stack of identical footprint pieces, one per board thickness of its height, each engraved with an id like B12·3/5 that the plate engraves at its footprint. Returns the files and a summary: beds, pieces, board area, buildings left out for being smaller than 2 mm and buildings without a measured height. check_only=true returns the summary without writing. The site must have been fetched first."
    )]
    async fn export_laser(&self, Parameters(input): Parameters<ExportLaser>) -> CallToolResult {
        let result = self
            .with_library(move |library| {
                let project = library.load(&input.id).map_err(text)?;
                let Some(scene) = &project.scene else {
                    return Err(format!(
                        "project '{}' has no site data yet; call fetch_site first",
                        input.id
                    ));
                };
                let look = project.sheet(input.tab.unwrap_or(1)).map_err(text)?;
                let settings = settings(&input, look.sheet.scale);

                let summary =
                    laser::summarize(scene, look.selection.as_ref(), &settings).map_err(text)?;
                if input.check_only.unwrap_or(false) {
                    return Ok(json!({ "summary": summary }));
                }
                let folder = written_folder(input.folder.as_deref())?;
                let stem = slug(&project.name);
                for number in 1..=summary.sheet_count {
                    let path = folder.join(laser::file_name(&stem, number, settings.format));
                    if path.exists() && !input.overwrite.unwrap_or(false) {
                        return Err(format!(
                            "{} already exists; pass overwrite: true to replace it",
                            path.display()
                        ));
                    }
                }
                let written =
                    laser::export(scene, look.selection.as_ref(), &settings, &folder, &stem)
                        .map_err(text)?;
                Ok(json!({ "files": written.files, "summary": written.summary }))
            })
            .await;
        reply(result)
    }
}

/// The input's settings, with the dialog's defaults for anything left out.
fn settings(input: &ExportLaser, sheet_scale: u32) -> LaserSettings {
    LaserSettings {
        scale: input
            .scale
            .unwrap_or_else(|| laser::default_scale(sheet_scale)),
        thickness_mm: input.thickness_mm.unwrap_or(DEFAULT_THICKNESS_MM),
        bed_width_mm: input.bed_width_mm.unwrap_or(DEFAULT_BED_MM.0),
        bed_height_mm: input.bed_height_mm.unwrap_or(DEFAULT_BED_MM.1),
        has_base_plate: input.base_plate.unwrap_or(true),
        has_buildings: input.buildings.unwrap_or(true),
        has_contours: input.contours.unwrap_or(true),
        format: match input.format {
            Some(LaserFormatInput::Dxf) => LaserFormat::Dxf,
            _ => LaserFormat::Svg,
        },
    }
}

/// The folder the files go in: needed, absolute and existing once anything is written.
fn written_folder(folder: Option<&str>) -> Result<PathBuf, String> {
    let Some(folder) = folder else {
        return Err("folder is needed unless check_only is true".to_string());
    };
    let path = PathBuf::from(folder);
    if !path.is_absolute() || !path.is_dir() {
        return Err(format!(
            "folder must be an absolute path to an existing folder, got \"{folder}\""
        ));
    }

    Ok(path)
}
