//! A model's triangles in metres with z up, standing on z = 0 and centred on
//! its plan, whatever unit and up axis the file was written in.

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// A larger model is refused: it would take minutes to draw on a sheet, and
/// no massing model of one building needs this many.
// #! NOTE[3d, size] : Measure real student exports before trusting this cap; no Archicad file was at hand
pub const MAX_TRIANGLES: usize = 500_000;

/// Above this extent in the file's own units a model is taken to be in
/// millimetres, above `CENTIMETRE_EXTENT` in centimetres. A building is a few
/// to a few hundred metres across, so 2000 units is a 2 km building in metres
/// or a 2 m one in millimetres; the first is far less likely.
const MILLIMETRE_EXTENT: f64 = 2000.0;
const CENTIMETRE_EXTENT: f64 = 300.0;

/// The unit a file's numbers are in. OBJ does not say, so the user chooses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Unit {
    Metre,
    Centimetre,
    Millimetre,
}

impl Unit {
    pub fn metres(self) -> f64 {
        match self {
            Unit::Metre => 1.0,
            Unit::Centimetre => 0.01,
            Unit::Millimetre => 0.001,
        }
    }

    /// The likely unit of a model whose largest extent is `extent` file units.
    pub fn guessed(extent: f64) -> Unit {
        if extent > MILLIMETRE_EXTENT {
            Unit::Millimetre
        } else if extent > CENTIMETRE_EXTENT {
            Unit::Centimetre
        } else {
            Unit::Metre
        }
    }
}

/// Which of the file's axes points up. glTF is y up by its specification;
/// CAD programs mostly write z up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UpAxis {
    Y,
    Z,
}

/// Triangles as a file holds them, before units and axes are settled.
#[derive(Debug, Clone, Default)]
pub struct RawMesh {
    pub positions: Vec<[f64; 3]>,
    pub triangles: Vec<[u32; 3]>,
}

impl RawMesh {
    /// The largest of the three extents, in file units.
    pub fn extent(&self) -> f64 {
        let (min, max) = bounds(&self.positions);
        (0..3).map(|axis| max[axis] - min[axis]).fold(0.0, f64::max)
    }
}

/// Metres, x east, y north and z up in the model's own frame, centred on the
/// middle of its plan with its lowest point at z = 0.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Mesh {
    pub positions: Vec<[f64; 3]>,
    pub triangles: Vec<[u32; 3]>,
}

impl Mesh {
    /// Width (x), depth (y) and height (z), in metres.
    pub fn size(&self) -> [f64; 3] {
        let (min, max) = bounds(&self.positions);
        [max[0] - min[0], max[1] - min[1], max[2] - min[2]]
    }

    /// The three corners of triangle `index`.
    pub fn corners(&self, index: usize) -> [[f64; 3]; 3] {
        let [a, b, c] = self.triangles[index];
        [
            self.positions[a as usize],
            self.positions[b as usize],
            self.positions[c as usize],
        ]
    }
}

/// `raw` in metres with z up, centred, standing on z = 0. Refuses an empty
/// model, one with a corner that is not a number, and one over `MAX_TRIANGLES`.
pub fn normalised(raw: RawMesh, unit: Unit, up: UpAxis) -> Result<Mesh> {
    if raw.triangles.is_empty() {
        return Err(Error::Invalid(
            "the model has no faces: export it with its surfaces, not only lines or points"
                .to_string(),
        ));
    }
    if raw.triangles.len() > MAX_TRIANGLES {
        return Err(Error::Invalid(format!(
            "the model has {} triangles, more than the {MAX_TRIANGLES} openarchi can draw; export a simpler model",
            raw.triangles.len()
        )));
    }
    let vertex_count = raw.positions.len() as u32;
    if let Some(triangle) = raw
        .triangles
        .iter()
        .find(|t| t.iter().any(|&i| i >= vertex_count))
    {
        return Err(Error::Invalid(format!(
            "the model's face {triangle:?} points past its {vertex_count} corners"
        )));
    }
    if raw
        .positions
        .iter()
        .flatten()
        .any(|value| !value.is_finite())
    {
        return Err(Error::Invalid(
            "the model has a corner that is not a number".to_string(),
        ));
    }

    Ok(normalised_unchecked(raw, unit.metres(), up))
}

/// `raw` scaled by `scale`, turned so `up` points up, centred and standing on z = 0.
fn normalised_unchecked(raw: RawMesh, scale: f64, up: UpAxis) -> Mesh {
    // glTF's y up turns into z up with its front (+z) facing south (-y), so a
    // model drawn facing the viewer in its program faces south on the site.
    let turned: Vec<[f64; 3]> = raw
        .positions
        .iter()
        .map(|&[x, y, z]| match up {
            UpAxis::Z => [x * scale, y * scale, z * scale],
            UpAxis::Y => [x * scale, -z * scale, y * scale],
        })
        .collect();
    let (min, max) = bounds(&turned);
    let centre = [(min[0] + max[0]) / 2.0, (min[1] + max[1]) / 2.0, min[2]];
    let positions = turned
        .iter()
        .map(|p| [p[0] - centre[0], p[1] - centre[1], p[2] - centre[2]])
        .collect();

    Mesh {
        positions,
        triangles: raw.triangles,
    }
}

/// `mesh`, read in `unit` with `up` as up, as if the file were read so again.
/// Reading undoes the old axis and scale and applies the new ones, and both
/// are linear, so it gives what reading the file would, without the file.
pub fn reread(mesh: &Mesh, from: (Unit, UpAxis), to: (Unit, UpAxis)) -> Mesh {
    let scale = to.0.metres() / from.0.metres();
    let file_axes: Vec<[f64; 3]> = mesh
        .positions
        .iter()
        .map(|&[x, y, z]| match from.1 {
            UpAxis::Z => [x, y, z],
            // The inverse of y up's [x, -z, y].
            UpAxis::Y => [x, z, -y],
        })
        .collect();
    let raw = RawMesh {
        positions: file_axes,
        triangles: mesh.triangles.clone(),
    };
    normalised_unchecked(raw, scale, to.1)
}

/// The lowest and highest value on each axis; zeros for no points.
fn bounds(positions: &[[f64; 3]]) -> ([f64; 3], [f64; 3]) {
    if positions.is_empty() {
        return ([0.0; 3], [0.0; 3]);
    }
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for point in positions {
        for axis in 0..3 {
            min[axis] = min[axis].min(point[axis]);
            max[axis] = max[axis].max(point[axis]);
        }
    }

    (min, max)
}
