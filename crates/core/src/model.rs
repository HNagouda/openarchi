//! A building model the user imported to see their design on the site.
//!
//! Every format is read into one shape: triangles in metres with z up, centred
//! on the model's plan and standing on z = 0 (`mesh.rs`). The model is placed
//! by hand at a point, a turn and a base level, because OBJ and glTF carry no
//! location. Existing buildings it covers are listed as replaced and left out
//! of every sheet. A project holds one model.
//
// #! NOTE[3d, owner] : One model per project; would variants of a proposal side by side be worth several?
// #! NOTE[3d, ifc] : IFC is not read yet. It is the one format that can place itself; see the IFC spike in .scratch/3d-plan.md

mod analysed;
mod faces;
mod gltf;
pub mod mesh;
mod obj;
pub mod packed_mesh;
mod place;
#[cfg(test)]
mod tests;

use std::io::BufReader;
use std::path::Path;
use std::sync::{Arc, OnceLock};

use serde::{Deserialize, Serialize};

pub use analysed::scene_with_model;
pub use faces::{Face, Shape};
pub use mesh::{Mesh, Unit, UpAxis};
pub use place::{
    buildings_under, covered_buildings, footprint_on_site, ground_m, placed, triangles_on_site,
    Covered, PlacedModel, REPLACED_SHARE,
};

use crate::scene::{Feature, Layer, Scene};
use crate::{Error, Result};

/// The extensions openarchi reads, in the words a refusal uses.
pub const READABLE_FORMATS: &str = "OBJ (.obj) or glTF (.glb, .gltf)";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    /// The file it was imported from: "house.obj".
    pub file_name: String,
    /// The unit the file was read in.
    pub unit: Unit,
    /// The file's axis that was taken as up.
    pub up: UpAxis,
    pub placement: Placement,
    /// The source ids (BAG pand ids) of the buildings the model takes the
    /// place of. They are left out of every sheet and the 3D view. Kept as a
    /// list the user edits, so moving the model later never silently changes it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub replaced: Vec<String>,
    /// Last, because it is nearly all of the model. Shared, so a model moved
    /// or turned is a cheap copy of the one before.
    #[serde(with = "packed_mesh")]
    pub mesh: Arc<Mesh>,
    /// The faces and footprint, worked out the first time a sheet asks.
    #[serde(skip)]
    shape: OnceLock<Arc<Shape>>,
}

/// Where the model stands. Saved with the project and changed by dragging on the plan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Placement {
    /// Where the middle of the model's plan stands, in the scene's projected
    /// coordinates (RD New metres in the Netherlands).
    pub x: f64,
    pub y: f64,
    /// The model turned clockwise seen from above, in degrees: 90 turns the
    /// side that faced north in the file to face east.
    #[serde(default)]
    pub rotation_deg: f64,
    /// The model's base in metres above NAP. `None` stands it on the terrain under its footprint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_m: Option<f64>,
}

impl Placement {
    /// At the middle of the scene, unturned, on the terrain.
    pub fn centre_of(scene: &Scene) -> Placement {
        Placement {
            x: scene.origin.0,
            y: scene.origin.1,
            rotation_deg: 0.0,
            base_m: None,
        }
    }

    /// Refuses a point or turn that is not a number, so a saved project never holds one.
    pub fn check(&self) -> Result<()> {
        let numbers = [
            self.x,
            self.y,
            self.rotation_deg,
            self.base_m.unwrap_or(0.0),
        ];
        if numbers.iter().any(|value| !value.is_finite()) {
            return Err(Error::Invalid(format!(
                "a model's place must be numbers, got {self:?}"
            )));
        }
        Ok(())
    }
}

impl Model {
    pub fn new(
        file_name: String,
        unit: Unit,
        up: UpAxis,
        mesh: Mesh,
        placement: Placement,
    ) -> Model {
        Model {
            file_name,
            unit,
            up,
            placement,
            replaced: Vec::new(),
            mesh: Arc::new(mesh),
            shape: OnceLock::new(),
        }
    }

    /// True for an existing building the model takes the place of.
    pub fn replaces(&self, feature: &Feature) -> bool {
        feature.layer == Layer::Building
            && feature
                .attributes
                .source_id
                .as_ref()
                .is_some_and(|id| self.replaced.contains(id))
    }

    /// The same model read in `unit` with `up` as up, at the same place,
    /// replacing the same buildings.
    pub fn reread(&self, unit: Unit, up: UpAxis) -> Model {
        let mesh = mesh::reread(&self.mesh, (self.unit, self.up), (unit, up));
        let mut model = Model::new(
            self.file_name.clone(),
            unit,
            up,
            mesh,
            self.placement.clone(),
        );
        model.replaced = self.replaced.clone();
        model
    }

    pub fn shape(&self) -> Arc<Shape> {
        self.shape
            .get_or_init(|| Arc::new(faces::shape(&self.mesh)))
            .clone()
    }
}

/// The model in the file at `path`, in metres with z up. A unit or up axis
/// left out is guessed: glTF is metres and y up by its specification; an OBJ
/// says neither, so its unit follows from its size and z is taken as up, as
/// CAD programs write it.
// #! NOTE[3d, verify] : Which axis is up in an Archicad OBJ export? Check a real file; the user can turn it in the model panel
pub fn read(path: &Path, unit: Option<Unit>, up: Option<UpAxis>) -> Result<(Mesh, Unit, UpAxis)> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let (raw, default_unit, default_up) = match extension.as_str() {
        "obj" => {
            let file = std::fs::File::open(path).map_err(|e| Error::io(path, e))?;
            let raw = obj::read(&mut BufReader::new(file), path)?;
            let unit = Unit::guessed(raw.extent());
            (raw, unit, UpAxis::Z)
        }
        "glb" | "gltf" => {
            let bytes = std::fs::read(path).map_err(|e| Error::io(path, e))?;
            (gltf::read(&bytes, path)?, Unit::Metre, UpAxis::Y)
        }
        "ifc" => {
            return Err(Error::Invalid(format!(
                "openarchi does not read IFC yet. In Archicad, use File > Save As and choose Wavefront (.obj), then import that; it reads {READABLE_FORMATS}"
            )))
        }
        _ => {
            return Err(Error::Invalid(format!(
                "openarchi reads models as {READABLE_FORMATS}, not {}",
                path.display()
            )))
        }
    };
    let unit = unit.unwrap_or(default_unit);
    let up = up.unwrap_or(default_up);

    Ok((mesh::normalised(raw, unit, up)?, unit, up))
}

/// The model in `path` placed on `scene`: where `placement` says, or at the
/// middle of the site, with the buildings it covers listed as replaced.
pub fn import(
    path: &Path,
    unit: Option<Unit>,
    up: Option<UpAxis>,
    scene: &Scene,
    placement: Option<Placement>,
) -> Result<Model> {
    let (mesh, unit, up) = read(path, unit, up)?;
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let placement = placement.unwrap_or_else(|| Placement::centre_of(scene));
    placement.check()?;
    let mut model = Model::new(file_name, unit, up, mesh, placement);
    model.replaced = buildings_under(&model, scene);

    Ok(model)
}
