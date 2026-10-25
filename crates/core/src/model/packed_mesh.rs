//! A model's mesh inside a project file: little-endian numbers compressed with
//! zstd, as base64 text, beside the packed scene (see `project/packed_scene.rs`).
//!
//! The bytes are the corner count as a u32, every corner as three f32 in
//! metres, then every triangle as three u32. f32 keeps a 100 m building to a
//! hundredth of a millimetre, at half the size of f64.

use std::sync::Arc;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use serde::de::Error as _;
use serde::ser::Error as _;
use serde::{Deserialize, Deserializer, Serializer};

use super::mesh::Mesh;
use crate::{Error, Result};

/// Corners and indices compress well; level 6 matches the packed scene.
const COMPRESSION_LEVEL: i32 = 6;
const NUMBER_BYTES: usize = 4;

pub fn pack(mesh: &Mesh) -> Result<String> {
    let mut bytes = Vec::with_capacity(
        NUMBER_BYTES * (1 + 3 * mesh.positions.len() + 3 * mesh.triangles.len()),
    );
    bytes.extend_from_slice(&(mesh.positions.len() as u32).to_le_bytes());
    for corner in &mesh.positions {
        for value in corner {
            bytes.extend_from_slice(&(*value as f32).to_le_bytes());
        }
    }
    for triangle in &mesh.triangles {
        for index in triangle {
            bytes.extend_from_slice(&index.to_le_bytes());
        }
    }
    let compressed = zstd::encode_all(bytes.as_slice(), COMPRESSION_LEVEL)
        .map_err(|error| Error::Invalid(format!("could not pack the model: {error}")))?;

    Ok(BASE64.encode(compressed))
}

pub fn unpack(text: &str) -> Result<Mesh> {
    let damaged = |detail: String| Error::Invalid(format!("the stored model is damaged: {detail}"));

    let compressed = BASE64
        .decode(text)
        .map_err(|error| damaged(error.to_string()))?;
    let bytes =
        zstd::decode_all(compressed.as_slice()).map_err(|error| damaged(error.to_string()))?;
    let (numbers, left_over) = bytes.as_chunks::<NUMBER_BYTES>();
    let Some((count, rest)) = numbers.split_first() else {
        return Err(damaged("it is empty".to_string()));
    };
    let corner_count = u32::from_le_bytes(*count) as usize;
    let is_whole = left_over.is_empty()
        && rest.len() >= 3 * corner_count
        && (rest.len() - 3 * corner_count).is_multiple_of(3);
    if !is_whole {
        return Err(damaged(format!(
            "{} bytes do not hold {corner_count} corners and whole triangles",
            bytes.len()
        )));
    }

    let (corners, indices) = rest.split_at(3 * corner_count);
    let positions = corners
        .as_chunks::<3>()
        .0
        .iter()
        .map(|corner| corner.map(|value| f64::from(f32::from_le_bytes(value))))
        .collect();
    let triangles: Vec<[u32; 3]> = indices
        .as_chunks::<3>()
        .0
        .iter()
        .map(|triangle| triangle.map(u32::from_le_bytes))
        .collect();
    if triangles
        .iter()
        .flatten()
        .any(|&index| index as usize >= corner_count)
    {
        return Err(damaged("a triangle points past the corners".to_string()));
    }

    Ok(Mesh {
        positions,
        triangles,
    })
}

/// For `#[serde(with = "packed_mesh")]` on `Model::mesh`.
pub fn serialize<S: Serializer>(
    mesh: &Arc<Mesh>,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error> {
    serializer.serialize_str(&pack(mesh).map_err(S::Error::custom)?)
}

pub fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Arc<Mesh>, D::Error> {
    let text = String::deserialize(deserializer)?;
    unpack(&text).map(Arc::new).map_err(D::Error::custom)
}
