//! What the model tools accept, with the schema an assistant sees.

use openarchi_core::model::{Unit, UpAxis};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::inputs::LonLatInput;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ImportModel {
    /// The project id. Its site must be fetched.
    pub id: String,
    /// Absolute path to an .obj (Archicad: File > Save As > Wavefront) or a
    /// .glb or .gltf (SketchUp, Rhino, Blender). A model already in the
    /// project is replaced, and the new one stands where it stood.
    pub path: String,
    /// The unit the file's numbers are in. Left out, glTF is read in metres
    /// and an OBJ's unit is guessed from its size.
    pub unit: Option<UnitInput>,
    /// Which of the file's axes points up. Left out, glTF is "y" and OBJ "z".
    pub up: Option<UpInput>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PlaceModel {
    /// The project id.
    pub id: String,
    /// Where the middle of the model's plan stands, in longitude and latitude.
    /// Leave out `at` and `rd` to keep it where it is.
    pub at: Option<LonLatInput>,
    /// The same point in RD New metres, `[x, y]`, instead of `at`.
    pub rd: Option<[f64; 2]>,
    /// The turn clockwise seen from above, in degrees: 90 turns the side that
    /// faced north in the file to face east.
    pub rotation_deg: Option<f64>,
    /// The base in metres above NAP. Leave out to keep the base as it is.
    pub base_m: Option<f64>,
    /// True stands the model on the terrain under it, dropping any `base_m`.
    pub on_terrain: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ModelUnits {
    /// The project id.
    pub id: String,
    pub unit: UnitInput,
    pub up: UpInput,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReplacedBuildings {
    /// The project id.
    pub id: String,
    /// The ids of the buildings the model replaces, each one listed under
    /// `covered` in the model's description. An empty list keeps every building.
    pub replaced: Vec<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UnitInput {
    Metre,
    Centimetre,
    Millimetre,
}

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UpInput {
    Y,
    Z,
}

impl From<UnitInput> for Unit {
    fn from(input: UnitInput) -> Unit {
        match input {
            UnitInput::Metre => Unit::Metre,
            UnitInput::Centimetre => Unit::Centimetre,
            UnitInput::Millimetre => Unit::Millimetre,
        }
    }
}

impl From<UpInput> for UpAxis {
    fn from(input: UpInput) -> UpAxis {
        match input {
            UpInput::Y => UpAxis::Y,
            UpInput::Z => UpAxis::Z,
        }
    }
}
