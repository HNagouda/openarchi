//! A model's triangles merged into the flat faces a person would see: one
//! polygon per wall, roof plane or floor, and the footprint those faces cover
//! in plan. A sheet draws faces rather than triangles, which keeps its SVG
//! small, and an exported OBJ splits every wall into many triangles.

use std::collections::HashMap;

use geo::{Coord, LineString, Polygon};
use i_overlay::core::fill_rule::FillRule;
use i_overlay::float::simplify::SimplifyShape;

use super::mesh::Mesh;

/// Triangles whose normals agree to a thousandth and whose planes lie within
/// a centimetre of each other count as one plane.
const NORMAL_STEPS: f64 = 1000.0;
const PLANE_STEP_M: f64 = 0.01;
/// Slivers the merge leaves behind, and triangles too thin to have a facing.
const MIN_FACE_AREA_M2: f64 = 1e-6;
/// A footprint piece smaller than this is a stray edge, not a part of the building.
const MIN_FOOTPRINT_AREA_M2: f64 = 0.01;
/// The footprint joins this many triangles at a time, then the pieces two by
/// two. One overlay of every triangle of a real model, floor stacked on floor
/// in plan, needed gigabytes and was killed; in steps each overlay stays small.
const FOOTPRINT_BATCH: usize = 256;

/// One flat face in the model's frame: its outline first, then its holes.
#[derive(Debug, Clone, PartialEq)]
pub struct Face {
    /// Unit length, pointing out of the model.
    pub normal: [f64; 3],
    pub rings: Vec<Vec<[f64; 3]>>,
}

/// What a sheet needs of a model, worked out once from its mesh.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Shape {
    pub faces: Vec<Face>,
    /// The plan the model covers, in its own frame.
    pub footprint: Vec<Polygon<f64>>,
    /// The top of the model above its base, in metres.
    pub height: f64,
}

pub fn shape(mesh: &Mesh) -> Shape {
    Shape {
        faces: faces(mesh),
        footprint: footprint(mesh),
        height: mesh.size()[2],
    }
}

/// The triangles grouped by plane, each group merged into polygons.
fn faces(mesh: &Mesh) -> Vec<Face> {
    let mut planes: HashMap<(i64, i64, i64, i64), Vec<usize>> = HashMap::new();
    let mut order = Vec::new();
    for index in 0..mesh.triangles.len() {
        let corners = mesh.corners(index);
        let Some(normal) = unit_normal(&corners) else {
            continue;
        };
        let offset = dot(normal, corners[0]);
        let key = (
            (normal[0] * NORMAL_STEPS).round() as i64,
            (normal[1] * NORMAL_STEPS).round() as i64,
            (normal[2] * NORMAL_STEPS).round() as i64,
            (offset / PLANE_STEP_M).round() as i64,
        );
        let group = planes.entry(key).or_default();
        if group.is_empty() {
            order.push(key);
        }
        group.push(index);
    }

    // In the order the planes first appear, so the same file always gives the same faces.
    let mut faces = Vec::new();
    for key in order {
        faces.extend(merged(mesh, &planes[&key]));
    }

    faces
}

/// One plane's triangles as one face per separate piece.
fn merged(mesh: &Mesh, triangles: &[usize]) -> Vec<Face> {
    let first = mesh.corners(triangles[0]);
    let normal = unit_normal(&first).unwrap_or([0.0, 0.0, 1.0]);
    if triangles.len() == 1 {
        return vec![Face {
            normal,
            rings: vec![first.to_vec()],
        }];
    }

    let origin = first[0];
    let (u, v) = plane_axes(normal);
    let to_plane = |p: [f64; 3]| {
        let d = [p[0] - origin[0], p[1] - origin[1], p[2] - origin[2]];
        [dot(d, u), dot(d, v)]
    };
    let to_space = |[a, b]: [f64; 2]| {
        [
            origin[0] + u[0] * a + v[0] * b,
            origin[1] + u[1] * a + v[1] * b,
            origin[2] + u[2] * a + v[2] * b,
        ]
    };
    // With u x v = normal every triangle of the plane turns counterclockwise,
    // so a nonzero fill joins them.
    let contours: Vec<Vec<[f64; 2]>> = triangles
        .iter()
        .map(|&index| mesh.corners(index).iter().map(|&p| to_plane(p)).collect())
        .collect();
    contours
        .simplify_shape(FillRule::NonZero, MIN_FACE_AREA_M2)
        .into_iter()
        .map(|shape| Face {
            normal,
            rings: shape
                .into_iter()
                .map(|ring| ring.into_iter().map(to_space).collect())
                .collect(),
        })
        .collect()
}

/// Every triangle laid flat and joined: the ground the model stands over.
fn footprint(mesh: &Mesh) -> Vec<Polygon<f64>> {
    let mut contours: Vec<Vec<[f64; 2]>> = Vec::new();
    for index in 0..mesh.triangles.len() {
        let [a, b, c] = mesh.corners(index);
        let flat = [[a[0], a[1]], [b[0], b[1]], [c[0], c[1]]];
        let twice_area = (flat[1][0] - flat[0][0]) * (flat[2][1] - flat[0][1])
            - (flat[2][0] - flat[0][0]) * (flat[1][1] - flat[0][1]);
        // Walls stand edge-on in plan and add nothing.
        if twice_area.abs() < 2.0 * MIN_FACE_AREA_M2 {
            continue;
        }
        if twice_area > 0.0 {
            contours.push(flat.to_vec());
        } else {
            contours.push(vec![flat[0], flat[2], flat[1]]);
        }
    }

    joined(contours)
        .into_iter()
        .map(|shape| {
            let mut rings = shape.into_iter().map(|ring| {
                LineString::from(
                    ring.into_iter()
                        .map(|[x, y]| Coord { x, y })
                        .collect::<Vec<_>>(),
                )
            });
            let exterior = rings.next().unwrap_or_else(|| LineString::new(Vec::new()));
            Polygon::new(exterior, rings.collect())
        })
        .collect()
}

/// `contours` joined into shapes, outline first, then holes, in batches. The
/// outlines come back counterclockwise and the holes clockwise, so the pieces
/// of one round join again under the same nonzero fill.
fn joined(contours: Vec<Vec<[f64; 2]>>) -> Vec<Vec<Vec<[f64; 2]>>> {
    let join = |contours: Vec<Vec<[f64; 2]>>| {
        contours.simplify_shape(FillRule::NonZero, MIN_FOOTPRINT_AREA_M2)
    };
    let mut pieces: Vec<Vec<Vec<[f64; 2]>>> = contours
        .chunks(FOOTPRINT_BATCH)
        .map(|batch| join(batch.to_vec()).concat())
        .collect();
    while pieces.len() > 1 {
        pieces = pieces
            .chunks(2)
            .map(|pair| join(pair.concat()).concat())
            .collect();
    }

    join(pieces.concat())
}

fn unit_normal([a, b, c]: &[[f64; 3]; 3]) -> Option<[f64; 3]> {
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let normal = cross(ab, ac);
    let length = dot(normal, normal).sqrt();
    // Twice the triangle's area; a thinner one has no reliable facing.
    (length > 2.0 * MIN_FACE_AREA_M2).then(|| normal.map(|value| value / length))
}

/// Two unit axes in the plane with `normal`, turning counterclockwise seen from its front.
fn plane_axes(normal: [f64; 3]) -> ([f64; 3], [f64; 3]) {
    let helper = if normal[2].abs() < 0.9 {
        [0.0, 0.0, 1.0]
    } else {
        [1.0, 0.0, 0.0]
    };
    let u = cross(helper, normal);
    let length = dot(u, u).sqrt();
    let u = u.map(|value| value / length);
    (u, cross(normal, u))
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

pub(super) fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
