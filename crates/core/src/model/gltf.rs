//! glTF 2.0, as SketchUp, Rhino and Blender write it: a binary .glb, or a
//! .gltf with its buffers embedded or in files beside it. Every node's
//! transform is applied; materials and textures are not read.

use std::path::Path;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use gltf::buffer::Source;
use gltf::mesh::Mode;
use gltf::Node;

use super::mesh::RawMesh;
use crate::{Error, Result};

/// Column-major, as glTF stores it.
type Matrix = [[f32; 4]; 4];

const IDENTITY: Matrix = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

pub fn read(bytes: &[u8], path: &Path) -> Result<RawMesh> {
    let format_error = |detail: String| Error::Format {
        path: path.display().to_string(),
        detail,
    };

    let file = gltf::Gltf::from_slice(bytes).map_err(|error| format_error(error.to_string()))?;
    let mut buffers = Vec::new();
    for buffer in file.document.buffers() {
        let data = match buffer.source() {
            Source::Bin => file
                .blob
                .clone()
                .ok_or_else(|| format_error("the binary chunk is missing".to_string()))?,
            Source::Uri(uri) => uri_bytes(uri, path).map_err(format_error)?,
        };
        buffers.push(data);
    }

    let scene = file
        .document
        .default_scene()
        .or_else(|| file.document.scenes().next())
        .ok_or_else(|| format_error("the file holds no scene".to_string()))?;
    let mut raw = RawMesh::default();
    for node in scene.nodes() {
        add_node(&mut raw, &node, IDENTITY, &buffers);
    }

    Ok(raw)
}

/// A buffer written into the file as a data URI, or kept in a file beside it.
fn uri_bytes(uri: &str, path: &Path) -> std::result::Result<Vec<u8>, String> {
    if let Some(data) = uri.strip_prefix("data:") {
        let (_, encoded) = data
            .split_once(";base64,")
            .ok_or_else(|| "an embedded buffer is not base64".to_string())?;
        return BASE64.decode(encoded).map_err(|error| error.to_string());
    }

    let folder = path.parent().unwrap_or(Path::new("."));
    let beside = folder.join(uri);
    std::fs::read(&beside).map_err(|error| format!("{}: {error}", beside.display()))
}

/// The triangles of `node` and its children, moved by their transforms.
fn add_node(raw: &mut RawMesh, node: &Node, parent: Matrix, buffers: &[Vec<u8>]) {
    let matrix = multiply(&parent, &node.transform().matrix());
    if let Some(mesh) = node.mesh() {
        for primitive in mesh.primitives() {
            // Strips and fans are rare in exported buildings; lines and points have no surface.
            // #! NOTE[3d, gltf] : Triangle strips and fans are skipped; convert them if a real export uses them
            if primitive.mode() != Mode::Triangles {
                continue;
            }
            let reader = primitive.reader(|buffer| buffers.get(buffer.index()).map(Vec::as_slice));
            let Some(positions) = reader.read_positions() else {
                continue;
            };
            let offset = raw.positions.len() as u32;
            for position in positions {
                raw.positions.push(transform(&matrix, position));
            }
            let count = raw.positions.len() as u32 - offset;
            let indices: Vec<u32> = match reader.read_indices() {
                Some(indices) => indices.into_u32().collect(),
                None => (0..count).collect(),
            };
            for &[a, b, c] in indices.as_chunks::<3>().0 {
                raw.triangles.push([offset + a, offset + b, offset + c]);
            }
        }
    }
    for child in node.children() {
        add_node(raw, &child, matrix, buffers);
    }
}

fn multiply(a: &Matrix, b: &Matrix) -> Matrix {
    let mut product = [[0.0; 4]; 4];
    for column in 0..4 {
        for row in 0..4 {
            product[column][row] = (0..4).map(|k| a[k][row] * b[column][k]).sum();
        }
    }

    product
}

fn transform(matrix: &Matrix, [x, y, z]: [f32; 3]) -> [f64; 3] {
    let row = |r: usize| matrix[0][r] * x + matrix[1][r] * y + matrix[2][r] * z + matrix[3][r];
    [f64::from(row(0)), f64::from(row(1)), f64::from(row(2))]
}
