//! The highlight tools: areas marked inside the site, like the plot or a
//! proposed footprint, and turning one into a sheet of its own. They follow
//! the app's rules: a highlight's middle lies inside the fetched site, and a
//! sheet opened from one frames exactly its shape. They join the router in
//! `server.rs`, like the sheet and pin tools.

use openarchi_core::area::Area;
use openarchi_core::crs::rd_to_wgs84;
use openarchi_core::highlight::{self, Highlight, HighlightShape};
use openarchi_core::project::{Look, Project};
use openarchi_core::render::fitting_scale;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{tool, tool_router};
use serde_json::{json, Value};

use crate::highlight_inputs::{AddHighlight, EditHighlight, HighlightId, OpenHighlight, ShowMarks};
use crate::server::{reply, text, OpenArchi};

#[tool_router(router = highlight_tools, vis = "pub(crate)")]
impl OpenArchi {
    #[tool(
        description = "List a project's highlights: areas marked inside the site, like the plot or a proposed house, shown on every sheet that does not hide them. Each has its id, label, outline (solid, dashed or dotted), colour, whether it dims the rest and shows its area, its area in m², and its shape in longitude and latitude."
    )]
    async fn list_highlights(
        &self,
        Parameters(input): Parameters<crate::inputs::ProjectId>,
    ) -> CallToolResult {
        let result = self
            .with_library(move |library| {
                let project = library.load(&input.id).map_err(text)?;
                Ok(highlights_of(&project))
            })
            .await;
        reply(result)
    }

    #[tool(
        description = "Highlight an area inside a project's site: a circle, rectangle or polygon in longitude and latitude, drawn in the palette's accent or a colour of its own with a light tint, a solid, dashed or dotted outline and an optional label, on every sheet that does not hide highlights, in plan, axonometric and on every plate of an explode view, and in the legend. Use it for the plot, a proposed footprint or an area of intervention. Returns every highlight."
    )]
    async fn add_highlight(&self, Parameters(input): Parameters<AddHighlight>) -> CallToolResult {
        let result = self
            .with_library(move |library| {
                let mut project = library.load(&input.id).map_err(text)?;
                let highlight = Highlight {
                    id: highlight::next_highlight_id(&project.highlights),
                    label: input.label.unwrap_or_default(),
                    shape: HighlightShape::from_area(&Area::from(input.shape)),
                    outline: input.outline.map(Into::into).unwrap_or_default(),
                    colour: input
                        .colour
                        .unwrap_or_else(|| highlight::DEFAULT_COLOUR.to_string()),
                    dims_rest: input.dims_rest.unwrap_or(false),
                    shows_area: input.shows_area.unwrap_or(false),
                };
                check_on_site(&project, &highlight)?;
                project.highlights.push(highlight);
                library.save(&mut project).map_err(text)?;
                Ok(highlights_of(&project))
            })
            .await;
        reply(result)
    }

    #[tool(
        description = "Move, reshape, relabel or restyle a highlight. Pass only what should change; a new shape replaces the old one. Returns every highlight."
    )]
    async fn edit_highlight(&self, Parameters(input): Parameters<EditHighlight>) -> CallToolResult {
        let result = self
            .with_library(move |library| {
                let mut project = library.load(&input.id).map_err(text)?;
                let mut edited = find_highlight(&project, &input.highlight_id)?.clone();
                if let Some(shape) = input.shape {
                    edited.shape = HighlightShape::from_area(&Area::from(shape));
                }
                if let Some(label) = input.label {
                    edited.label = label;
                }
                if let Some(outline) = input.outline {
                    edited.outline = outline.into();
                }
                if let Some(colour) = input.colour {
                    edited.colour = colour;
                }
                if let Some(dims_rest) = input.dims_rest {
                    edited.dims_rest = dims_rest;
                }
                if let Some(shows_area) = input.shows_area {
                    edited.shows_area = shows_area;
                }
                check_on_site(&project, &edited)?;
                for highlight in &mut project.highlights {
                    if highlight.id == edited.id {
                        *highlight = edited.clone();
                    }
                }
                library.save(&mut project).map_err(text)?;
                Ok(highlights_of(&project))
            })
            .await;
        reply(result)
    }

    #[tool(
        description = "Remove a highlight from a project. Earlier versions stay in the project's backups. Returns the highlights left."
    )]
    async fn remove_highlight(&self, Parameters(input): Parameters<HighlightId>) -> CallToolResult {
        let result = self
            .with_library(move |library| {
                let mut project = library.load(&input.id).map_err(text)?;
                find_highlight(&project, &input.highlight_id)?;
                project
                    .highlights
                    .retain(|highlight| highlight.id != input.highlight_id);
                library.save(&mut project).map_err(text)?;
                Ok(highlights_of(&project))
            })
            .await;
        reply(result)
    }

    #[tool(
        description = "Open a highlight as a sheet of its own: adds a sheet tab, named after the highlight and copied from an existing tab, whose area is exactly the highlight's shape, at the largest round scale it fits, down to 1:50. This is the only way to make a sheet of part of the site. Its ratios and figures (get_ratios, get_environment with its `tab`) then describe the highlighted area. Returns every tab; the new one is last."
    )]
    async fn open_highlight_as_sheet(
        &self,
        Parameters(input): Parameters<OpenHighlight>,
    ) -> CallToolResult {
        let result = self
            .with_library(move |library| {
                let mut project = library.load(&input.id).map_err(text)?;
                let highlight = find_highlight(&project, &input.highlight_id)?;
                let Some(scene) = &project.scene else {
                    return Err(no_site(&project));
                };
                let source = project.sheet(input.copy_of.unwrap_or(1)).map_err(text)?;
                let mut look = Look {
                    name: highlight.legend_label().to_string(),
                    selection: Some(highlight.shape.to_area()),
                    ..source.clone()
                };
                look.sheet.scale = fitting_scale(scene, &look).map_err(text)?;
                project.more_sheets.push(look);
                library.save(&mut project).map_err(text)?;
                Ok(crate::sheets::sheets_of(&project))
            })
            .await;
        reply(result)
    }

    #[tool(
        description = "Show or hide the project's pins and highlights on one sheet tab, as the eye buttons under the app's sheet do. Pass only what should change. Returns the sheet's tab list entry."
    )]
    async fn show_on_sheet(&self, Parameters(input): Parameters<ShowMarks>) -> CallToolResult {
        let result = self
            .with_library(move |library| {
                let mut project = library.load(&input.id).map_err(text)?;
                let tab = input.tab.unwrap_or(1);
                let look = project.sheet_mut(tab).map_err(text)?;
                if let Some(pins) = input.pins {
                    look.hide_pins = !pins;
                }
                if let Some(highlights) = input.highlights {
                    look.hide_highlights = !highlights;
                }
                library.save(&mut project).map_err(text)?;
                Ok(crate::sheets::sheets_of(&project)["sheets"][tab - 1].clone())
            })
            .await;
        reply(result)
    }
}

fn highlights_of(project: &Project) -> Value {
    let highlights: Vec<Value> = project
        .highlights
        .iter()
        .map(|highlight| {
            json!({
                "id": highlight.id,
                "label": highlight.label,
                "outline": highlight.outline,
                "colour": highlight.colour,
                "dims_rest": highlight.dims_rest,
                "shows_area": highlight.shows_area,
                "area_m2": highlight.shape.area_m2().round(),
                "shape": shape_in_degrees(&highlight.shape),
            })
        })
        .collect();

    json!({ "highlights": highlights })
}

/// The shape as an assistant passes it back: in longitude and latitude, a
/// rectangle by its south-west and north-east corners.
fn shape_in_degrees(shape: &HighlightShape) -> Value {
    match shape {
        HighlightShape::Rectangle { min, max } => json!({
            "kind": "rectangle",
            "south_west": rd_to_wgs84(min[0], min[1]),
            "north_east": rd_to_wgs84(max[0], max[1]),
        }),
        _ => json!(shape.to_area()),
    }
}

/// The app's rule: a highlight the sheet can draw, whose middle lies inside the fetched site.
fn check_on_site(project: &Project, highlight: &Highlight) -> Result<(), String> {
    let Some(scene) = &project.scene else {
        return Err(no_site(project));
    };
    highlight.check().map_err(text)?;
    if !highlight::is_on_site(scene, highlight) {
        return Err(format!(
            "the middle of highlight {} lies outside the fetched site, so it would never show; draw it inside the site",
            highlight.id
        ));
    }

    Ok(())
}

fn no_site(project: &Project) -> String {
    format!(
        "project '{}' has no site data yet; call fetch_site before highlighting areas",
        project.id
    )
}

fn find_highlight<'a>(project: &'a Project, highlight_id: &str) -> Result<&'a Highlight, String> {
    project
        .highlights
        .iter()
        .find(|highlight| highlight.id == highlight_id)
        .ok_or_else(|| {
            let ids: Vec<&str> = project
                .highlights
                .iter()
                .map(|highlight| highlight.id.as_str())
                .collect();
            format!(
                "no highlight '{highlight_id}'; the project's highlights are: {}",
                if ids.is_empty() {
                    "none".to_string()
                } else {
                    ids.join(", ")
                }
            )
        })
}
