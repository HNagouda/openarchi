//! Footprints and ground areas as triangles that face the right way: the
//! ground and roofs up, walls out, so the 3D view lights each from its front.

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use geo::orient::{Direction, Orient};
use geo::{LineString, Polygon, TriangulateEarcut};

/// Corners, three to a triangle.
#[derive(Debug, Clone, Default)]
pub struct Triangles {
    corners: Vec<[f64; 3]>,
}

impl From<Vec<[f64; 3]>> for Triangles {
    fn from(corners: Vec<[f64; 3]>) -> Triangles {
        Triangles { corners }
    }
}

impl Triangles {
    pub fn count(&self) -> usize {
        self.corners.len() / 3
    }

    /// The corners as little-endian f32, in base64.
    pub fn packed(&self) -> String {
        let mut bytes = Vec::with_capacity(self.corners.len() * 12);
        for corner in &self.corners {
            for value in corner {
                bytes.extend_from_slice(&(*value as f32).to_le_bytes());
            }
        }
        BASE64.encode(bytes)
    }

    fn push(&mut self, a: [f64; 3], b: [f64; 3], c: [f64; 3]) {
        self.corners.extend([a, b, c]);
    }

    /// `polygon` laid flat at height `z`, every triangle facing up.
    fn push_flat(&mut self, polygon: &Polygon<f64>, z: f64) {
        let raw = polygon.earcut_triangles_raw();
        let corner = |index: usize| [raw.vertices[2 * index], raw.vertices[2 * index + 1], z];
        for &[first, second, third] in raw.triangle_indices.as_chunks::<3>().0 {
            let (a, b, c) = (corner(first), corner(second), corner(third));
            let turn = (b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1]);
            if turn >= 0.0 {
                self.push(a, b, c);
            } else {
                self.push(a, c, b);
            }
        }
    }
}

/// `polygons` laid flat at height `z`, facing up.
pub fn flat(polygons: &[Polygon<f64>], z: f64) -> Triangles {
    let mut triangles = Triangles::default();
    for polygon in polygons {
        triangles.push_flat(polygon, z);
    }
    triangles
}

/// `polygon` stood up `height` metres: its walls facing out and a flat roof.
pub fn extrude(triangles: &mut Triangles, polygon: &Polygon<f64>, height: f64) {
    // Outline counterclockwise and holes clockwise, so the solid is always on
    // the left of each edge and a wall's outside on its right.
    let polygon = polygon.orient(Direction::Default);
    for ring in std::iter::once(polygon.exterior()).chain(polygon.interiors()) {
        push_walls(triangles, ring, height);
    }
    triangles.push_flat(&polygon, height);
}

fn push_walls(triangles: &mut Triangles, ring: &LineString<f64>, height: f64) {
    for edge in ring.lines() {
        let (p, q) = (edge.start, edge.end);
        let (p0, q0) = ([p.x, p.y, 0.0], [q.x, q.y, 0.0]);
        let (p1, q1) = ([p.x, p.y, height], [q.x, q.y, height]);
        triangles.push(p0, q0, q1);
        triangles.push(p0, q1, p1);
    }
}
