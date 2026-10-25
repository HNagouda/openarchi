//! The tools that write a sheet or a report out: render_preview for a look
//! at the sheet, export for a file of it, and the report tools. They join the
//! router in `server.rs`, like the sheet and pin tools.

use std::path::PathBuf;

use base64::prelude::{Engine, BASE64_STANDARD};
use openarchi_core::export::{self, DxfLayers, ExportFormat};
use openarchi_core::render;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock};
use rmcp::{tool, tool_router};
use serde_json::json;

use crate::export_inputs::{Export, RenderPreview};
use crate::inputs::ProjectId;
use crate::reports::{self, ExportReport};
use crate::resolve::with_render_input;
use crate::series::export_series;
use crate::server::{failure, reply, OpenArchi};

const DEFAULT_PREVIEW_WIDTH_PX: u32 = 1200;
/// Narrower than this the labels are unreadable.
const MIN_PREVIEW_WIDTH_PX: u32 = 200;
/// Wider than this the image is more than an assistant can take in at once.
const MAX_PREVIEW_WIDTH_PX: u32 = 4000;

const DEFAULT_EXPORT_DPI: u32 = 300;

#[tool_router(router = export_tools, vis = "pub(crate)")]
impl OpenArchi {
    #[tool(
        description = "Render the project's sheet as a PNG image so you can see it: the same drawing the app shows and export writes. The site must have been fetched first."
    )]
    async fn render_preview(&self, Parameters(input): Parameters<RenderPreview>) -> CallToolResult {
        let width_px = input
            .width_px
            .unwrap_or(DEFAULT_PREVIEW_WIDTH_PX)
            .clamp(MIN_PREVIEW_WIDTH_PX, MAX_PREVIEW_WIDTH_PX);
        let result = self
            .with_library(move |library| {
                with_render_input(library, &input.id, input.tab.unwrap_or(1), |render| {
                    let missing = render::missing_data(render)?;
                    Ok((export::render_png(render, width_px)?, missing))
                })
            })
            .await;

        match result {
            // The same notes the app shows over the sheet, so an assistant knows why a diagram looks empty.
            Ok((png, missing)) => {
                let mut content = vec![ContentBlock::image(
                    BASE64_STANDARD.encode(png),
                    "image/png",
                )];
                content.extend(missing.into_iter().map(ContentBlock::text));
                CallToolResult::success(content)
            }
            Err(message) => CallToolResult::error(vec![ContentBlock::text(message)]),
        }
    }

    #[tool(
        description = "Write the project's sheet to a file: svg or pdf for print, png for slides, dxf for CAD. A plan's DXF is in RD New metres with a layer per data layer, per legend class of the diagram, for the diagram's computed layers (city blocks, hours of sun), highlights (solid, dashed, dotted), the section line and pins, each in its colour on the sheet as the nearest AutoCAD colour; dxf_layers picks only the layers the sheet shows (the default) or every fetched layer; a section sheet's DXF is the section in metres along the line and metres NAP. series=true writes one file per diagram and variant into folder instead, as the app's \"whole series\" export does. Refuses to replace an existing file unless overwrite is true. The site must have been fetched first."
    )]
    async fn export(&self, Parameters(input): Parameters<Export>) -> CallToolResult {
        let format = ExportFormat::from(input.format);
        let dpi = input.dpi.unwrap_or(DEFAULT_EXPORT_DPI);
        let dxf_layers = input
            .dxf_layers
            .map_or(DxfLayers::default(), DxfLayers::from);
        if input.series.unwrap_or(false) {
            let Some(folder) = input.folder.clone().map(PathBuf::from) else {
                return failure("folder is needed when series is true".to_string());
            };
            if !folder.is_absolute() || !folder.is_dir() {
                return failure(format!(
                    "folder must be an absolute path to an existing folder, got \"{}\"",
                    folder.display()
                ));
            }
            let result = self
                .with_library(move |library| {
                    export_series(
                        library,
                        &input.id,
                        input.tab.unwrap_or(1),
                        format,
                        dpi,
                        dxf_layers,
                        &folder,
                        input.overwrite.unwrap_or(false),
                    )
                })
                .await;
            return reply(result);
        }
        let Some(path_text) = input.path.clone() else {
            return failure("path is needed unless series is true".to_string());
        };
        let path = PathBuf::from(&path_text);
        if !path.is_absolute() {
            return failure(format!("path must be absolute, got \"{path_text}\""));
        }
        if path.exists() && !input.overwrite.unwrap_or(false) {
            return failure(format!(
                "{} already exists; pass overwrite: true to replace it",
                path.display()
            ));
        }
        if !path.parent().is_some_and(|folder| folder.is_dir()) {
            return failure(format!("the folder for {} does not exist", path.display()));
        }

        let result = self
            .with_library(move |library| {
                with_render_input(library, &input.id, input.tab.unwrap_or(1), |render| {
                    export::export_with_dxf_layers(render, format, dpi, dxf_layers, &path)
                })?;
                Ok(json!({ "path": path, "format": format }))
            })
            .await;
        reply(result)
    }

    #[tool(
        description = "List a project's reports: pages the user laid out in the app, each with its paper size and the blocks placed on it (ratios charts, analysis figures, facts and notes, sheets in another diagram, and text), positioned in millimetres. Pass a report id to export_report."
    )]
    async fn list_reports(&self, Parameters(input): Parameters<ProjectId>) -> CallToolResult {
        reply(
            self.with_library(move |library| reports::list(library, &input.id))
                .await,
        )
    }

    #[tool(
        description = "Write one of a project's reports, drawn from the project's current data in the report's palette: a PDF with a page per report page (the default), or an SVG or PNG per page. Refuses to replace existing files unless overwrite is true. The site must have been fetched first."
    )]
    async fn export_report(&self, Parameters(input): Parameters<ExportReport>) -> CallToolResult {
        reply(
            self.with_library(move |library| reports::export(library, input))
                .await,
        )
    }
}
