//! The site edit tools: finding a tree or building at a point, taking it
//! away or bringing it back, moving and planting trees, and listing the
//! edits. Each does what the app's edit tool on the plan does, no more:
//! buildings can be removed but not moved, and trees stand inside the site.
//! They join the router in `server.rs`.

use openarchi_core::crs::wgs84_to_rd;
use openarchi_core::edits::{editable_at, edited_things, surveyed, PLANTED_PREFIX};
use openarchi_core::project::Project;
use openarchi_core::scene::Layer;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{tool, tool_router};
use serde_json::{json, Value};

use crate::edits_inputs::{MoveTree, PickOnSite, PlantTree, SiteThing};
use crate::inputs::LonLatInput;
use crate::server::{fetched_scene, reply, text, OpenArchi};

/// A tree's crown as every view draws it (`render::solid`), the reach a click on the plan has.
const DEFAULT_REACH_M: f64 = 3.0;
/// Far enough to find any tree near a point, close enough to stay on the spot asked about.
const MAX_REACH_M: f64 = 50.0;

#[tool_router(router = edit_tools, vis = "pub(crate)")]
impl OpenArchi {
    #[tool(
        description = "Find the tree or building at a point on a project's site, as clicking it with the app's edit tool does: the nearest tree within `reach_m` (default 3 m), or else the building whose footprint holds the point. Removed ones are found too, so they can be restored. Returns its id, layer, where it stands and whether it is removed, moved or planted, or null when nothing is there."
    )]
    async fn pick_on_site(&self, Parameters(input): Parameters<PickOnSite>) -> CallToolResult {
        let result = self
            .with_library(move |library| {
                let project = library.load(&input.id).map_err(text)?;
                let scene = fetched_scene(&project)?;
                let (x, y) = rd_point(input.at, input.rd)?;
                let reach_m = input.reach_m.unwrap_or(DEFAULT_REACH_M);
                if !(0.0..=MAX_REACH_M).contains(&reach_m) {
                    return Err(format!(
                        "reach_m must be between 0 and {MAX_REACH_M}, got {reach_m}"
                    ));
                }
                let found = editable_at(scene, &project.edits, x, y, reach_m);
                Ok(json!({ "id": project.id, "found": found }))
            })
            .await;
        reply(result)
    }

    #[tool(
        description = "List a project's site edits: every tree and building taken away (by hand, or under the model), every tree moved, with where the survey has it, and every tree planted. Sheets, the 3D view, the computed diagrams and the exports all draw the site with these edits; ratios and the analysis count the site as fetched."
    )]
    async fn list_site_edits(
        &self,
        Parameters(input): Parameters<crate::inputs::ProjectId>,
    ) -> CallToolResult {
        let result = self
            .with_library(move |library| {
                let project = library.load(&input.id).map_err(text)?;
                Ok(edits_of(&project))
            })
            .await;
        reply(result)
    }

    #[tool(
        description = "Take a tree or building away from a project's site, as Delete does with the app's edit tool. It is left out of every sheet, the 3D view, the computed diagrams and the exports, and restore_on_site brings it back. A planted tree is simply gone. Returns the edits."
    )]
    async fn remove_from_site(&self, Parameters(input): Parameters<SiteThing>) -> CallToolResult {
        let result = self
            .with_library(move |library| {
                let mut project = library.load(&input.id).map_err(text)?;
                known_thing(&project, &input.thing)?;
                project.edits = project.edits.with_removed(&input.thing);
                library.save(&mut project).map_err(text)?;
                Ok(edits_of(&project))
            })
            .await;
        reply(result)
    }

    #[tool(
        description = "Bring a removed tree or building back on a project's site, and stand a moved tree back where the survey has it, as Restore does in the app. Returns the edits."
    )]
    async fn restore_on_site(&self, Parameters(input): Parameters<SiteThing>) -> CallToolResult {
        let result = self
            .with_library(move |library| {
                let mut project = library.load(&input.id).map_err(text)?;
                let is_edited = project.edits.is_removed(&input.thing)
                    || project
                        .edits
                        .moved
                        .iter()
                        .any(|tree| tree.id == input.thing);
                if !is_edited {
                    return Err(format!(
                        "'{}' is neither removed nor moved; list_site_edits lists what is",
                        input.thing
                    ));
                }
                project.edits = project.edits.with_restored(&input.thing);
                library.save(&mut project).map_err(text)?;
                Ok(edits_of(&project))
            })
            .await;
        reply(result)
    }

    #[tool(
        description = "Move a tree on a project's site, as dragging it with the app's edit tool does. It must stay inside the site's outline. Buildings cannot be moved. Returns the edits."
    )]
    async fn move_tree(&self, Parameters(input): Parameters<MoveTree>) -> CallToolResult {
        let result = self
            .with_library(move |library| {
                let mut project = library.load(&input.id).map_err(text)?;
                if known_thing(&project, &input.tree)? != Layer::Tree {
                    return Err(format!(
                        "'{}' is a building; buildings can be removed but not moved",
                        input.tree
                    ));
                }
                let (x, y) = rd_point(input.at, input.rd)?;
                let edits = project.edits.with_moved(&input.tree, x, y);
                edits.check(fetched_scene(&project)?).map_err(text)?;
                project.edits = edits;
                library.save(&mut project).map_err(text)?;
                Ok(edits_of(&project))
            })
            .await;
        reply(result)
    }

    #[tool(
        description = "Plant a new tree on a project's site, as placing one with the app's edit tool does. It stands inside the site's outline and draws like any other tree, as tall and wide as every tree is drawn. Returns its id and the edits."
    )]
    async fn plant_tree(&self, Parameters(input): Parameters<PlantTree>) -> CallToolResult {
        let result = self
            .with_library(move |library| {
                let mut project = library.load(&input.id).map_err(text)?;
                let (x, y) = rd_point(input.at, input.rd)?;
                let (edits, tree) = project.edits.with_planted(x, y);
                edits.check(fetched_scene(&project)?).map_err(text)?;
                project.edits = edits;
                library.save(&mut project).map_err(text)?;
                Ok(json!({ "tree": tree, "edits": edits_json(&project) }))
            })
            .await;
        reply(result)
    }
}

/// Every edit with what it names, for the overview and the edit tools' answers.
pub(crate) fn edits_json(project: &Project) -> Value {
    match &project.scene {
        Some(scene) => json!(edited_things(scene, &project.edits)),
        None => json!([]),
    }
}

fn edits_of(project: &Project) -> Value {
    json!({ "id": project.id, "edits": edits_json(project) })
}

/// The layer of the tree or building `thing` names, or why it names none.
fn known_thing(project: &Project, thing: &str) -> Result<Layer, String> {
    let is_planted = thing.starts_with(PLANTED_PREFIX)
        && project.edits.planted.iter().any(|tree| tree.id == thing);
    if is_planted {
        return Ok(Layer::Tree);
    }
    surveyed(fetched_scene(project)?, thing).ok_or_else(|| {
        format!("the site has no tree or building '{thing}'; pick_on_site finds them by place")
    })
}

/// A point given as `at` or `rd`, in RD New metres.
fn rd_point(at: Option<LonLatInput>, rd: Option<[f64; 2]>) -> Result<(f64, f64), String> {
    match (at, rd) {
        (Some(_), Some(_)) => Err("give the point as `at` or `rd`, not both".to_string()),
        (Some(at), None) => Ok(wgs84_to_rd(at.into())),
        (None, Some([x, y])) => Ok((x, y)),
        (None, None) => Err("give the point as `at` (lon and lat) or `rd` ([x, y])".to_string()),
    }
}
