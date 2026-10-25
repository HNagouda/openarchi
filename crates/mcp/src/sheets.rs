//! The tools for sheet tabs and pins. They live apart from `server.rs` and
//! join its router there, so the tool list stays one list for the client.

use openarchi_core::area::LonLat;
use openarchi_core::crs::{rd_to_wgs84, wgs84_to_rd};
use openarchi_core::pin::{self, number_of, Pin};
use openarchi_core::project::{Look, Project};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{tool, tool_router};
use serde_json::{json, Value};

use crate::inputs::ProjectId;
use crate::server::{reply, text, OpenArchi};
use crate::sheet_inputs::{AddPin, AddSheet, EditPin, PinId};

#[tool_router(router = sheet_tools, vis = "pub(crate)")]
impl OpenArchi {
    #[tool(
        description = "List a project's sheet tabs in order. Each has its number (pass it as `tab` to set_look, edit_text, render_preview, get_ratios and export), name, diagram, variant, theme, view, sheet area (the highlight it was opened from with open_highlight_as_sheet, in longitude and latitude, or null for the whole site) and whether it hides the pins and the highlights, its section line (null until set_section draws one) and the View diagram's viewpoint (null for the site centre, see set_view_point)."
    )]
    async fn list_sheets(&self, Parameters(input): Parameters<ProjectId>) -> CallToolResult {
        let result = self
            .with_library(move |library| {
                let project = library.load(&input.id).map_err(text)?;
                Ok(sheets_of(&project))
            })
            .await;
        reply(result)
    }

    #[tool(
        description = "Add a sheet tab to a project, copied from an existing tab and showing the whole site. For a sheet of part of the site, highlight that part with add_highlight and use open_highlight_as_sheet, as the app does. Returns every tab; the new one is last. Style it with set_look and `tab`."
    )]
    async fn add_sheet(&self, Parameters(input): Parameters<AddSheet>) -> CallToolResult {
        let result = self
            .with_library(move |library| {
                let mut project = library.load(&input.id).map_err(text)?;
                let number = project.sheet_count() + 1;
                let source = project.sheet(input.copy_of.unwrap_or(1)).map_err(text)?;
                // A new tab shows the whole site, as the app's + does.
                let look = Look {
                    name: input.name.unwrap_or_else(|| format!("Sheet {number}")),
                    selection: None,
                    hide_pins: false,
                    hide_highlights: false,
                    ..source.clone()
                };
                project.more_sheets.push(look);
                library.save(&mut project).map_err(text)?;
                Ok(sheets_of(&project))
            })
            .await;
        reply(result)
    }

    #[tool(
        description = "List a project's pins: landmarks marked on every sheet that does not hide them. Each has its id, icon, size, label, location (lon, lat), RD New x and y in metres, and for numbered pins the number they show."
    )]
    async fn list_pins(&self, Parameters(input): Parameters<ProjectId>) -> CallToolResult {
        let result = self
            .with_library(move |library| {
                let project = library.load(&input.id).map_err(text)?;
                Ok(pins_of(&project))
            })
            .await;
        reply(result)
    }

    #[tool(
        description = "Pin a landmark on a project: a marker (pin, star, flag or dot) in one of four sizes (small, medium, large, extra_large) with a short label, at a location inside the fetched site. It shows on every sheet that does not hide pins, in plan, axonometric and explode views, and in DXF on its own layer. Returns every pin."
    )]
    async fn add_pin(&self, Parameters(input): Parameters<AddPin>) -> CallToolResult {
        let result = self
            .with_library(move |library| {
                let mut project = library.load(&input.id).map_err(text)?;
                let (x, y) = place_on_site(&project, input.location.into())?;
                let pin = Pin {
                    id: pin::next_pin_id(&project.pins),
                    icon: input.icon.map(Into::into).unwrap_or_default(),
                    label: input.label.unwrap_or_default(),
                    x,
                    y,
                    size: input.size.map(Into::into).unwrap_or_default(),
                };
                pin.check().map_err(text)?;
                project.pins.push(pin);
                library.save(&mut project).map_err(text)?;
                Ok(pins_of(&project))
            })
            .await;
        reply(result)
    }

    #[tool(
        description = "Move a pin, or change its label, marker or size. Pass only what should change. Returns every pin."
    )]
    async fn move_pin(&self, Parameters(input): Parameters<EditPin>) -> CallToolResult {
        let result = self
            .with_library(move |library| {
                let mut project = library.load(&input.id).map_err(text)?;
                let place = match input.location {
                    Some(location) => Some(place_on_site(&project, location.into())?),
                    None => None,
                };
                let pin = find_pin(&mut project, &input.pin_id)?;
                if let Some((x, y)) = place {
                    (pin.x, pin.y) = (x, y);
                }
                if let Some(label) = input.label {
                    pin.label = label;
                }
                if let Some(icon) = input.icon {
                    pin.icon = icon.into();
                }
                if let Some(size) = input.size {
                    pin.size = size.into();
                }
                pin.check().map_err(text)?;
                library.save(&mut project).map_err(text)?;
                Ok(pins_of(&project))
            })
            .await;
        reply(result)
    }

    #[tool(
        description = "Remove a pin from a project. Earlier versions stay in the project's backups. Returns the pins left."
    )]
    async fn remove_pin(&self, Parameters(input): Parameters<PinId>) -> CallToolResult {
        let result = self
            .with_library(move |library| {
                let mut project = library.load(&input.id).map_err(text)?;
                find_pin(&mut project, &input.pin_id)?;
                project.pins.retain(|pin| pin.id != input.pin_id);
                library.save(&mut project).map_err(text)?;
                Ok(pins_of(&project))
            })
            .await;
        reply(result)
    }
}

pub(crate) fn sheets_of(project: &Project) -> Value {
    let sheets: Vec<Value> = project
        .sheets()
        .iter()
        .enumerate()
        .map(|(index, look)| {
            let name = if look.name.is_empty() {
                format!("Sheet {}", index + 1)
            } else {
                look.name.clone()
            };
            json!({
                "tab": index + 1,
                "name": name,
                "diagram_id": look.diagram_id,
                "variant": look.variant,
                "theme_id": look.theme_id,
                "view": look.view.mode,
                "scale": look.sheet.scale,
                "selection": look.selection,
                "hide_pins": look.hide_pins,
                "hide_highlights": look.hide_highlights,
                "section": crate::sections::section_of(look)["section"],
                "view_point": crate::view_point::view_point_of(look)["view_point"],
            })
        })
        .collect();

    json!({ "sheets": sheets })
}

fn pins_of(project: &Project) -> Value {
    let pins: Vec<Value> = project
        .pins
        .iter()
        .map(|pin| {
            json!({
                "id": pin.id,
                "icon": pin.icon,
                "size": pin.size,
                "label": pin.label,
                "number": number_of(&project.pins, &pin.id),
                "location": rd_to_wgs84(pin.x, pin.y),
                "x": pin.x,
                "y": pin.y,
            })
        })
        .collect();

    json!({ "pins": pins })
}

/// The location in RD New metres, if it lies inside the fetched site.
fn place_on_site(project: &Project, location: LonLat) -> Result<(f64, f64), String> {
    let Some(scene) = &project.scene else {
        return Err(format!(
            "project '{}' has no site data yet; call fetch_site before pinning landmarks",
            project.id
        ));
    };
    let (x, y) = wgs84_to_rd(location);
    if !pin::is_on_site(scene, x, y) {
        return Err(format!(
            "({}, {}) lies outside the fetched site, so a pin there would never show; pick a place inside it",
            location.lon, location.lat
        ));
    }

    Ok((x, y))
}

fn find_pin<'a>(project: &'a mut Project, pin_id: &str) -> Result<&'a mut Pin, String> {
    let ids: Vec<String> = project.pins.iter().map(|pin| pin.id.clone()).collect();
    project
        .pins
        .iter_mut()
        .find(|pin| pin.id == pin_id)
        .ok_or_else(|| {
            format!(
                "no pin '{pin_id}'; the project's pins are: {}",
                if ids.is_empty() {
                    "none".to_string()
                } else {
                    ids.join(", ")
                }
            )
        })
}
