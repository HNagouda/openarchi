//! The viewpoint tool: where the eye of a sheet's view diagram stands, as the
//! app sets it, on the site centre, on a pin or at a placed point. It joins
//! the router in `server.rs`, like the section tool.

use openarchi_core::crs::{rd_to_wgs84, wgs84_to_rd};
use openarchi_core::pin::Pin;
use openarchi_core::project::Look;
use openarchi_core::viewshed::check_view_point;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::inputs::LonLatInput;
use crate::server::{reply, text, OpenArchi};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetViewPoint {
    /// The project id.
    pub id: String,
    /// Which sheet tab, counting from 1 for the first, which is the default.
    pub tab: Option<usize>,
    /// Stand the eye here: inside the area the sheet draws, the fetched site
    /// and its neighbourhood.
    pub at: Option<LonLatInput>,
    /// Stand the eye on this pin, by its id from list_pins. The eye follows
    /// the pin when it moves, and goes back to the site centre if the pin is removed.
    pub pin: Option<String>,
    /// True stands the eye back on the street or square nearest the site centre, the default.
    pub centre: Option<bool>,
}

#[tool_router(router = view_point_tools, vis = "pub(crate)")]
impl OpenArchi {
    #[tool(
        description = "Set where the eye of a sheet's View diagram stands: the ground a person standing there can see is tinted, worked out by line of sight past the buildings, the woods and the terrain. Pass exactly one of at (a lon/lat inside the site or its neighbourhood), pin (a pin id from list_pins; the eye then follows the pin, and goes back to the site centre if it is removed) or centre: true (back to the street or square nearest the site centre, the default). Then set_look with diagram_id \"view\" shows it. Returns the sheet's viewpoint, null for the site centre."
    )]
    async fn set_view_point(&self, Parameters(input): Parameters<SetViewPoint>) -> CallToolResult {
        let result = self
            .with_library(move |library| {
                let mut project = library.load(&input.id).map_err(text)?;
                let Some(scene) = project.scene.clone() else {
                    return Err(format!(
                        "project '{}' has no site data yet; call fetch_site before setting a viewpoint",
                        input.id
                    ));
                };
                // As in the app: a placed point, or a pin the eye then follows, or neither.
                let (point, pin_id) = match (input.at, input.pin, input.centre) {
                    (Some(at), None, None) => (Some(wgs84_to_rd(at.into())), None),
                    (None, Some(pin_id), None) => (None, Some(pin_id)),
                    (None, None, Some(true)) => (None, None),
                    _ => {
                        return Err("pass exactly one of at, pin or centre: true".to_string());
                    }
                };
                if let Some(pin_id) = &pin_id {
                    let pin = project
                        .pins
                        .iter()
                        .find(|pin| pin.id == *pin_id)
                        .ok_or_else(|| format!("project '{}' has no pin '{pin_id}'", input.id))?;
                    check_view_point(&scene, (pin.x, pin.y)).map_err(text)?;
                }
                if let Some(point) = point {
                    check_view_point(&scene, point).map_err(text)?;
                }
                let pins = project.pins.clone();
                let look = project.sheet_mut(input.tab.unwrap_or(1)).map_err(text)?;
                look.view_point = point;
                look.view_pin = pin_id;
                let shown = view_point_of(look, &pins);
                library.save(&mut project).map_err(text)?;
                Ok(shown)
            })
            .await;
        reply(result)
    }
}

/// The sheet's viewpoint in longitude and latitude and in RD New metres, with
/// the pin it stands on, or null at the site centre.
pub fn view_point_of(look: &Look, pins: &[Pin]) -> Value {
    let view_point = look.eye_among(pins).map(|(x, y)| {
        json!({
            "at": rd_to_wgs84(x, y),
            "rd": [x, y],
            "pin": look.view_pin,
        })
    });

    json!({ "view_point": view_point, "diagram_id": look.diagram_id })
}
