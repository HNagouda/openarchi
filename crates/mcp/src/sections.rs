//! The section tool: draw, change or clear the line a sheet is cut along.
//! It joins the router in `server.rs`, like the sheet and pin tools.

use openarchi_core::crs::{rd_to_wgs84, wgs84_to_rd};
use openarchi_core::project::Look;
use openarchi_core::section::{self, SectionLine, DEFAULT_DEPTH_M};
use openarchi_core::view::ViewMode;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::inputs::LonLatInput;
use crate::server::{fetched_scene, reply, text, OpenArchi};

/// The `scale` an assistant passes to go back to the scale that fits the sheet.
const FITTED_SCALE: u32 = 0;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetSection {
    /// The project id.
    pub id: String,
    /// Which sheet tab, counting from 1 for the first, which is the default.
    pub tab: Option<usize>,
    /// The end labelled A. The section is seen from A towards A' with A on the
    /// left, so it looks to the left of the line as you walk from A to A'.
    pub from: Option<LonLatInput>,
    /// The end labelled A'. At least 5 m from `from`.
    pub to: Option<LonLatInput>,
    /// How far beyond the line buildings and trees are drawn in elevation, in
    /// metres, more than 0 and at most 500. Defaults to 60.
    pub depth_m: Option<f64>,
    /// The scale the section is drawn at, as N in 1:N: one of 100, 200, 250,
    /// 500, 750, 1000, 1250, 1500, 2000, 2500, 3000, 4000, 5000, 7500, 10000,
    /// 20000. 0 goes back to the largest scale that fits the sheet, the default.
    pub scale: Option<u32>,
    /// How many times taller than true heights are drawn: 1 (the default), 2 or 5.
    /// The legend says so when it is not 1.
    pub exaggeration: Option<u32>,
    /// True swaps A and A', turning the section round to look the other way.
    pub flip: Option<bool>,
    /// True removes the line; a sheet in the section view goes back to its plan.
    pub clear: Option<bool>,
}

#[tool_router(router = section_tools, vis = "pub(crate)")]
impl OpenArchi {
    #[tool(
        description = "Draw, change or clear the section line of a sheet: the line its section view is cut along, shown on its plan as a cut marker with A and A'. Pass from and to (both inside the area the sheet draws: the site and its neighbourhood) to draw or move it, depth_m for how far beyond the line the elevation reaches, scale and exaggeration for how large and how tall it is drawn, flip to look the other way, or clear. Then set_look with view.mode \"section\" shows the section. Returns the sheet's line and view."
    )]
    async fn set_section(&self, Parameters(input): Parameters<SetSection>) -> CallToolResult {
        let result = self
            .with_library(move |library| {
                let mut project = library.load(&input.id).map_err(text)?;
                let tab = input.tab.unwrap_or(1);
                let scene = fetched_scene(&project)?;
                let current = project.sheet(tab).map_err(text)?.section.as_ref();
                let line = if input.clear == Some(true) {
                    None
                } else {
                    let line = updated_line(current, &input)?;
                    line.check().map_err(text)?;
                    if !section::lies_within(scene, &line) {
                        return Err(
                            "both ends must lie inside the area the sheet draws: the fetched site and its neighbourhood"
                                .to_string(),
                        );
                    }
                    Some(line)
                };

                let look = project.sheet_mut(tab).map_err(text)?;
                match line {
                    Some(line) => look.section = Some(line),
                    None => clear(look),
                }
                let shown = section_of(look);
                library.save(&mut project).map_err(text)?;
                Ok(shown)
            })
            .await;
        reply(result)
    }
}

/// No line, and the plan in place of a section that can no longer be drawn, as in the app.
fn clear(look: &mut Look) {
    look.section = None;
    if look.view.mode == ViewMode::Section {
        look.view.mode = ViewMode::Plan;
    }
}

/// The sheet's line with every passed field replaced, or a new one.
fn updated_line(line: Option<&SectionLine>, input: &SetSection) -> Result<SectionLine, String> {
    let start = match input.from {
        Some(from) => Some(wgs84_to_rd(from.into())),
        None => line.map(|line| line.start),
    };
    let end = match input.to {
        Some(to) => Some(wgs84_to_rd(to.into())),
        None => line.map(|line| line.end),
    };
    let (Some(start), Some(end)) = (start, end) else {
        return Err("this sheet has no section line yet; pass both from and to".to_string());
    };
    let drawn = SectionLine {
        start,
        end,
        depth_m: input
            .depth_m
            .or(line.map(|line| line.depth_m))
            .unwrap_or(DEFAULT_DEPTH_M),
        scale: match input.scale {
            Some(FITTED_SCALE) => None,
            Some(scale) => Some(scale),
            None => line.and_then(|line| line.scale),
        },
        exaggeration: input
            .exaggeration
            .or(line.map(|line| line.exaggeration))
            .unwrap_or(1),
    };

    if input.flip == Some(true) {
        Ok(drawn.flipped())
    } else {
        Ok(drawn)
    }
}

/// The sheet's line in longitude and latitude and in RD New metres, and its view.
pub fn section_of(look: &Look) -> Value {
    let section = look.section.as_ref().map(|line| {
        json!({
            "from": rd_to_wgs84(line.start.0, line.start.1),
            "to": rd_to_wgs84(line.end.0, line.end.1),
            "start_rd": [line.start.0, line.start.1],
            "end_rd": [line.end.0, line.end.1],
            "depth_m": line.depth_m,
            "scale": line.scale,
            "exaggeration": line.exaggeration,
        })
    });

    json!({ "section": section, "view": look.view.mode })
}
