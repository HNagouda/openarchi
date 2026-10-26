//! The model tools: importing a building model, placing and turning it,
//! reading it in another unit, choosing which buildings it replaces, and
//! removing it. Each does what the app's Model section and the sheet's model
//! tool do, no more: placing picks the replaced buildings again by the same
//! rule, and only a building under the model can be marked replaced. They
//! join the router in `server.rs`.

use std::path::PathBuf;

use openarchi_core::crs::{rd_to_wgs84, wgs84_to_rd};
use openarchi_core::model::{self, Model};
use openarchi_core::project::Project;
use openarchi_core::scene::Scene;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{tool, tool_router};
use serde_json::{json, Value};

use crate::model_inputs::{ImportModel, ModelUnits, PlaceModel, ReplacedBuildings};
use crate::server::{fetched_scene, reply, text, OpenArchi};

#[tool_router(router = model_tools, vis = "pub(crate)")]
impl OpenArchi {
    #[tool(
        description = "Import a building model into a project, to see a design on its site: an .obj (Archicad: File > Save As > Wavefront) or a .glb/.gltf (SketchUp, Rhino, Blender), read in metres with y up as OBJ and glTF write it, and centred on its plan. It stands at the site centre, or where the project's previous model stood, on the terrain, and the existing buildings it covers more than half of are left out of every sheet. It draws in the palette's accent: its footprint in plan, standing up in the axonometric and casting shadows in the shadow diagram. Returns the model: size, placement and the buildings under it."
    )]
    async fn import_model(&self, Parameters(input): Parameters<ImportModel>) -> CallToolResult {
        let result = self
            .with_library(move |library| {
                let mut project = library.load(&input.id).map_err(text)?;
                let placement = project.model.as_ref().map(|kept| kept.placement.clone());
                let imported = model::import(
                    &PathBuf::from(&input.path),
                    input.unit.map(Into::into),
                    input.up.map(Into::into),
                    fetched_scene(&project)?,
                    placement,
                )
                .map_err(text)?;
                project.model = Some(imported);
                library.save(&mut project).map_err(text)?;
                model_of(&project)
            })
            .await;
        reply(result)
    }

    #[tool(
        description = "Move, turn or lift the project's model. Pass only what should change: `at` (longitude and latitude) or `rd` ([x, y] in RD New metres) for the middle of its plan, `rotation_deg` clockwise, `base_m` above NAP or `on_terrain`. Like dragging it in the app, this picks again the buildings it replaces: those it covers more than half of. Returns the model."
    )]
    async fn place_model(&self, Parameters(input): Parameters<PlaceModel>) -> CallToolResult {
        let result = self
            .with_library(move |library| {
                let mut project = library.load(&input.id).map_err(text)?;
                let mut model = taken_model(&mut project)?;
                let mut placement = model.placement.clone();
                if let Some((x, y)) = rd_point(&input)? {
                    (placement.x, placement.y) = (x, y);
                }
                if let Some(rotation_deg) = input.rotation_deg {
                    placement.rotation_deg = rotation_deg.rem_euclid(360.0);
                }
                if let Some(base_m) = input.base_m {
                    placement.base_m = Some(base_m);
                }
                if input.on_terrain == Some(true) {
                    placement.base_m = None;
                }
                placement.check().map_err(text)?;
                model.placement = placement;
                model.replaced = model::buildings_under(&model, fetched_scene(&project)?);
                project.model = Some(model);
                library.save(&mut project).map_err(text)?;
                model_of(&project)
            })
            .await;
        reply(result)
    }

    #[tool(
        description = "Read the project's model again in another unit (metre, centimetre, millimetre) or with another axis up (y, z), as the Model section's switches do, when it came in a thousand times too large or lying on its side. It keeps its place and the buildings it replaces. Returns the model."
    )]
    async fn set_model_units(&self, Parameters(input): Parameters<ModelUnits>) -> CallToolResult {
        let result = self
            .with_library(move |library| {
                let mut project = library.load(&input.id).map_err(text)?;
                let model = taken_model(&mut project)?;
                project.model = Some(model.reread(input.unit.into(), input.up.into()));
                library.save(&mut project).map_err(text)?;
                model_of(&project)
            })
            .await;
        reply(result)
    }

    #[tool(
        description = "Choose which existing buildings under the project's model it replaces: they are left out of every sheet. Each id must be one of the buildings listed under `covered` in the model, as the app lists them with a box to tick. Returns the model."
    )]
    async fn set_replaced_buildings(
        &self,
        Parameters(input): Parameters<ReplacedBuildings>,
    ) -> CallToolResult {
        let result = self
            .with_library(move |library| {
                let mut project = library.load(&input.id).map_err(text)?;
                let mut model = taken_model(&mut project)?;
                let covered = model::covered_buildings(&model, fetched_scene(&project)?);
                if let Some(stray) = input
                    .replaced
                    .iter()
                    .find(|id| !covered.iter().any(|building| &building.id == *id))
                {
                    return Err(format!(
                        "building '{stray}' is not under the model; it can only replace the buildings listed under `covered`"
                    ));
                }
                model.replaced = input.replaced;
                project.model = Some(model);
                library.save(&mut project).map_err(text)?;
                model_of(&project)
            })
            .await;
        reply(result)
    }

    #[tool(
        description = "Remove the project's model. The buildings it replaced come back on every sheet. The previous version stays in the project's backups."
    )]
    async fn remove_model(
        &self,
        Parameters(input): Parameters<crate::inputs::ProjectId>,
    ) -> CallToolResult {
        let result = self
            .with_library(move |library| {
                let mut project = library.load(&input.id).map_err(text)?;
                taken_model(&mut project)?;
                library.save(&mut project).map_err(text)?;
                Ok(json!({ "id": project.id, "model": null }))
            })
            .await;
        reply(result)
    }
}

/// The project's model, for the overview and every model tool's answer.
pub(crate) fn model_json(model: &Model, scene: Option<&Scene>) -> Value {
    let placement = &model.placement;
    let at = rd_to_wgs84(placement.x, placement.y);
    json!({
        "file_name": model.file_name,
        "unit": model.unit,
        "up": model.up,
        "size_m": model.mesh.size(),
        "triangles": model.mesh.triangles.len(),
        "placement": placement,
        "at": { "lon": at.lon, "lat": at.lat },
        "ground_m": scene.and_then(|scene| model::ground_m(model, scene)),
        "replaced": model.replaced,
        "covered": scene.map(|scene| model::covered_buildings(model, scene)),
    })
}

fn model_of(project: &Project) -> Result<Value, String> {
    let model = project
        .model
        .as_ref()
        .ok_or_else(|| format!("project '{}' has no model", project.id))?;
    Ok(json!({ "id": project.id, "model": model_json(model, project.scene.as_ref()) }))
}

/// The project's model, taken out so the scene can be read while it changes.
/// Nothing is saved until the tool puts it back.
fn taken_model(project: &mut Project) -> Result<Model, String> {
    project.model.take().ok_or_else(|| {
        format!(
            "project '{}' has no model; call import_model first",
            project.id
        )
    })
}

/// The new middle of the model in RD New metres, `None` to keep it.
fn rd_point(input: &PlaceModel) -> Result<Option<(f64, f64)>, String> {
    match (input.at, input.rd) {
        (Some(_), Some(_)) => Err("give the model's place as `at` or `rd`, not both".to_string()),
        (Some(at), None) => Ok(Some(wgs84_to_rd(at.into()))),
        (None, Some([x, y])) => Ok(Some((x, y))),
        (None, None) => Ok(None),
    }
}
