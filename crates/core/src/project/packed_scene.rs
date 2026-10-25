//! The scene inside a project file: JSON, compressed with zstd, as base64 text.
//!
//! A scene is nearly all of a project file, and its JSON is mostly repeated
//! coordinate keys, which zstd shrinks seven- to eightfold, five- to sixfold
//! once written as base64 text (the three test projects). The rest of the file
//! stays plain JSON, so the project list can read a name without unpacking the
//! scene, and an older openarchi still sees `format_version` and says to update.

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use serde::de::Error as _;
use serde::ser::Error as _;
use serde::{Deserialize, Deserializer, Serializer};

use crate::scene::Scene;
use crate::{Error, Result};

/// The MCP server packs on every save, and the app on every fetch, so speed
/// counts as much as size. On the Westermarkt scene level 6 is a quarter
/// smaller than level 3 and still packs about 240 MB of JSON a second; level 9
/// is barely smaller and half as fast.
// #! NOTE[mcp, save] : Should MCP edits that leave the scene alone save with the packed text read from disk, as the app does?
const COMPRESSION_LEVEL: i32 = 6;

/// The scene as stored text.
pub fn pack(scene: &Scene) -> Result<String> {
    let pack_error = |detail: String| Error::Invalid(format!("could not pack the scene: {detail}"));

    // Written straight into the compressor, so the full JSON never sits in memory.
    let mut encoder = zstd::Encoder::new(Vec::new(), COMPRESSION_LEVEL)
        .map_err(|error| pack_error(error.to_string()))?;
    serde_json::to_writer(&mut encoder, scene).map_err(|error| pack_error(error.to_string()))?;
    let compressed = encoder
        .finish()
        .map_err(|error| pack_error(error.to_string()))?;

    Ok(BASE64.encode(compressed))
}

/// The scene back from stored text.
pub fn unpack(text: &str) -> Result<Scene> {
    let unpack_error =
        |detail: String| Error::Invalid(format!("the stored scene is damaged: {detail}"));

    let compressed = BASE64
        .decode(text)
        .map_err(|error| unpack_error(error.to_string()))?;
    let json =
        zstd::decode_all(compressed.as_slice()).map_err(|error| unpack_error(error.to_string()))?;
    serde_json::from_slice(&json).map_err(|error| unpack_error(error.to_string()))
}

/// For `#[serde(with = "packed_scene")]` on `Project::scene`.
pub fn serialize<S: Serializer>(
    scene: &Option<Scene>,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error> {
    match scene {
        Some(scene) => serializer.serialize_some(&pack(scene).map_err(S::Error::custom)?),
        None => serializer.serialize_none(),
    }
}

pub fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Option<Scene>, D::Error> {
    match Option::<String>::deserialize(deserializer)? {
        Some(text) => unpack(&text).map(Some).map_err(D::Error::custom),
        None => Ok(None),
    }
}
