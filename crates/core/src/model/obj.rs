//! Wavefront OBJ, as Archicad writes it from File > Save As. Only the
//! surfaces are read: no materials, lines or points.

use std::io::BufRead;
use std::path::Path;

use super::mesh::RawMesh;
use crate::{Error, Result};

pub fn read(reader: &mut impl BufRead, path: &Path) -> Result<RawMesh> {
    let options = tobj::LoadOptions {
        // Faces with more than three corners become triangles.
        triangulate: true,
        ignore_points: true,
        ignore_lines: true,
        ..Default::default()
    };
    // The material file is never opened: a model draws in the sheet's palette.
    let (models, _) =
        tobj::load_obj_buf(reader, &options, |_| Err(tobj::LoadError::OpenFileFailed)).map_err(
            |error| Error::Format {
                path: path.display().to_string(),
                detail: error.to_string(),
            },
        )?;

    // Each object has its own corners; they go one after another into one mesh.
    let mut raw = RawMesh::default();
    for model in &models {
        let offset = raw.positions.len() as u32;
        for &[x, y, z] in model.mesh.positions.as_chunks::<3>().0 {
            raw.positions
                .push([f64::from(x), f64::from(y), f64::from(z)]);
        }
        for &[a, b, c] in model.mesh.indices.as_chunks::<3>().0 {
            raw.triangles.push([offset + a, offset + b, offset + c]);
        }
    }

    Ok(raw)
}
